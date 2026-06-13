use std::{cell::RefCell, collections::HashMap, rc::Rc};

use std::collections::HashSet;

use crate::{
    ast::{
        expression::{
            BinaryOperator, CallParameter, CastKind, ElseBranch, Expression, TryKind,
            UnaryOperator,
        },
        node::Program,
        statement::{
            AccessModifier, AccessorKind, FunctionBody, GenericParameterKind, ModifierType,
            OperatorFixity, Parameter, Pattern, ProtocolMember, Statement, VariadicKind,
            WhereRequirement, WhereRequirementKind,
        },
    },
    diag::{
        TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token,
        secondary_label_from_token,
    },
    krate::{Module, Package},
    lexer::token::{Position, Token, TokenType},
    scope::Scope,
    symbol::{Symbol, WeakSymbol},
    types::Type,
};

type MethodInfo = Option<(String, Rc<RefCell<Type>>, Vec<Rc<RefCell<Type>>>)>;

#[derive(Debug)]
pub struct TypeResolver {
    pub packages: HashMap<String, Rc<RefCell<Package>>>,
    #[allow(dead_code)]
    current_package: String,
    current_module: Option<Rc<RefCell<Module>>>,
    current_return_type: Option<Rc<RefCell<Type>>>,
    current_scope: Option<Rc<RefCell<Scope>>>,
    current_owner: Option<Rc<RefCell<Symbol>>>,
    closure_expected_type: Option<Rc<RefCell<Type>>>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
    initialized_lets: Vec<HashSet<String>>,
    initialized_properties: HashSet<String>,
    is_in_init: bool,
    yield_context_depth: usize,
    superclass_map: HashMap<String, Rc<RefCell<Type>>>,
}

impl TypeResolver {
    pub fn new(
        packages: HashMap<String, Rc<RefCell<Package>>>,
        current_package: String,
        engine: Rc<RefCell<TrussDiagnosticEngine>>,
    ) -> Self {
        Self {
            packages,
            current_package,
            current_module: None,
            current_return_type: None,
            current_scope: None,
            current_owner: None,
            closure_expected_type: None,
            engine,
            initialized_lets: Vec::new(),
            initialized_properties: HashSet::new(),
            is_in_init: false,
            yield_context_depth: 0,
            superclass_map: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, program: &Program, module: Rc<RefCell<Module>>) {
        self.current_module = Some(module.clone());
        let scope = module.borrow().scope.clone().unwrap();
        self.enter_scope(scope);

        for stmt in &program.statements {
            self.process_decl(stmt.clone());
        }

        for stmt in &program.statements {
            self.resolve_statement(stmt.clone());
        }

        self.leave_scope();
    }

    fn process_decl(&mut self, statement: Rc<RefCell<Statement>>) {
        let container_access = {
            let stmt_ref = statement.borrow();
            Self::get_access_modifier(&stmt_ref)
        };
        let is_container_class = {
            let stmt_ref = statement.borrow();
            matches!(&*stmt_ref, Statement::ClassDecl { .. })
        };

        match &mut *statement.borrow_mut() {
            Statement::FunctionDecl {
                name,
                parameters,
                return_type,
                throws_types,
                body,
                scope,
                ty,
                generic_parameters,
                ..
            } => {
                self.enter_scope(scope.as_ref().unwrap().clone());
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { const_type } => {
                            let resolved = self
                                .infer_type(const_type.clone())
                                .unwrap_or_else(|| Rc::new(RefCell::new(Type::Never)));
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                resolved,
                            )))
                        }
                    };
                    self.current_scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.infer_type(default_value.clone());
                    }
                }

                let ret_type = if let Some(return_type_expr) = return_type {
                    self.infer_type(return_type_expr.clone())
                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
                } else {
                    Rc::new(RefCell::new(Type::Void))
                };

                let mut parameter_types = Vec::new();
                let mut is_vararg = false;
                for param in parameters.iter() {
                    if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                        is_vararg = true;
                        continue;
                    }
                    let param_type = self.infer_type(param.borrow().type_expression.clone());
                    if let Some(ref param_type) = param_type {
                        param.borrow_mut().ty = Some(param_type.clone());
                        parameter_types.push(param_type.clone());
                        if let Some(default_value) = &param.borrow().default_value {
                            self.infer_expression_type(default_value.clone(), param_type.clone());
                        }
                    }
                    if param.borrow().variadic_kind != VariadicKind::NotVariadic {
                        is_vararg = true;
                    }
                }

                let throws_ty = throws_types.as_ref().map(|types| {
                    types.iter().filter_map(|t| self.infer_type(t.clone())).collect()
                });

                let fn_type = Rc::new(RefCell::new(Type::Function(
                    parameter_types,
                    ret_type,
                    is_vararg,
                    throws_ty,
                )));
                *ty = Some(fn_type.clone());

                if let Some(parent) = self.current_scope.as_ref().unwrap().borrow().parent.clone() {
                    parent.borrow_mut().set_type(name.value.clone(), fn_type);
                }

                match &*body.borrow() {
                    FunctionBody::Statements(stmts) => {
                        for s in stmts {
                            self.process_decl(s.clone());
                        }
                    }
                    FunctionBody::Expression(expr) => {
                        self.process_function_decl_in_expr(expr.clone());
                    }
                    FunctionBody::None => {}
                }

                self.leave_scope();
            }
            Statement::StructDecl {
                name,
                body,
                scope,
                conformances,
                generic_parameters,
                ..
            } => {
                let Some(symbol) = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().name_table.get(&name.value).cloned())
                else {
                    return;
                };

                let struct_ty = Rc::new(RefCell::new(Type::Struct(
                    name.value.clone(),
                    WeakSymbol(Rc::downgrade(&symbol)),
                    vec![],
                )));
                let self_ty = struct_ty.clone();
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), struct_ty.clone());

                for conformance in conformances.iter() {
                    self.infer_type(conformance.clone());
                }

                let prev_owner = self.current_owner.replace(symbol.clone());
                self.enter_scope(scope.as_ref().unwrap().clone());
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { const_type } => {
                            let resolved = self
                                .infer_type(const_type.clone())
                                .unwrap_or_else(|| Rc::new(RefCell::new(Type::Never)));
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                resolved,
                            )))
                        }
                    };
                    self.current_scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.infer_type(default_value.clone());
                    }
                }
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type("self".to_string(), self_ty);
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type("Self".to_string(), struct_ty.clone());
                for stmt in body.iter() {
                    let method_info: MethodInfo = {
                        if let Statement::FunctionDecl {
                            name: method_name,
                            parameters,
                            return_type,
                            ..
                        } = &*stmt.borrow()
                        {
                            let ret_type = if let Some(return_type_expr) = return_type {
                                self.infer_type(return_type_expr.clone())
                                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
                            } else {
                                Rc::new(RefCell::new(Type::Void))
                            };

                            let mut parameter_types = Vec::new();
                            let mut is_vararg = false;
                            for param in parameters.iter() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    is_vararg = true;
                                    continue;
                                }
                                let param_type =
                                    self.infer_type(param.borrow().type_expression.clone());
                                if let Some(ref param_type) = param_type {
                                    param.borrow_mut().ty = Some(param_type.clone());
                                    parameter_types.push(param_type.clone());
                                }
                                if param.borrow().variadic_kind != VariadicKind::NotVariadic {
                                    is_vararg = true;
                                }
                            }

                            let fn_type = Rc::new(RefCell::new(Type::Function(
                                parameter_types.clone(),
                                ret_type,
                                is_vararg,
                                None,
                            )));
                            Some((method_name.value.clone(), fn_type, parameter_types))
                        } else {
                            None
                        }
                    };

                    if let Some((_method_name, fn_type, _)) = method_info
                        && let Statement::FunctionDecl { ty, .. } = &mut *stmt.borrow_mut()
                    {
                        *ty = Some(fn_type.clone());
                    }
                    self.process_decl(stmt.clone());
                }
                self.leave_scope();
                self.current_owner = prev_owner;
                self.validate_member_access_levels(
                    container_access.clone(),
                    is_container_class,
                    body,
                );
                self.validate_setter_access_conflicts(body);
                self.check_protocol_conformances(&name.value, name.as_ref(), conformances, false);
            }
            Statement::ClassDecl {
                name,
                body,
                scope,
                superclass,
                conformances,
                generic_parameters,
                ..
            } => {
                let Some(symbol) = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().name_table.get(&name.value).cloned())
                else {
                    return;
                };

                let class_ty = Rc::new(RefCell::new(Type::Class(
                    name.value.clone(),
                    WeakSymbol(Rc::downgrade(&symbol)),
                    vec![],
                )));
                let self_ty = class_ty.clone();
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), class_ty.clone());

                if let Some(superclass_expr) = superclass {
                    self.infer_type(superclass_expr.clone());
                    if let Expression::Type {
                        name: super_name, ..
                    } = &*superclass_expr.borrow()
                    {
                        let super_ty = self
                            .current_scope
                            .as_ref()
                            .unwrap()
                            .borrow()
                            .get_type(&super_name.value);
                        if let Some(super_ty) = super_ty {
                            self.superclass_map.insert(name.value.clone(), super_ty);
                        }
                    }
                }
                for conformance in conformances.iter() {
                    self.infer_type(conformance.clone());
                }

                let prev_owner = self.current_owner.replace(symbol.clone());
                self.enter_scope(scope.as_ref().unwrap().clone());
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { const_type } => {
                            let resolved = self
                                .infer_type(const_type.clone())
                                .unwrap_or_else(|| Rc::new(RefCell::new(Type::Never)));
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                resolved,
                            )))
                        }
                    };
                    self.current_scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.infer_type(default_value.clone());
                    }
                }
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type("self".to_string(), self_ty);
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type("Self".to_string(), class_ty.clone());
                for stmt in body.iter() {
                    let method_info: MethodInfo = {
                        if let Statement::FunctionDecl {
                            name: method_name,
                            parameters,
                            return_type,
                            ..
                        } = &*stmt.borrow()
                        {
                            let ret_type = if let Some(return_type_expr) = return_type {
                                self.infer_type(return_type_expr.clone())
                                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
                            } else {
                                Rc::new(RefCell::new(Type::Void))
                            };

                            let mut parameter_types = Vec::new();
                            let mut is_vararg = false;
                            for param in parameters.iter() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    is_vararg = true;
                                    continue;
                                }
                                let param_type =
                                    self.infer_type(param.borrow().type_expression.clone());
                                if let Some(ref param_type) = param_type {
                                    param.borrow_mut().ty = Some(param_type.clone());
                                    parameter_types.push(param_type.clone());
                                }
                                if param.borrow().variadic_kind != VariadicKind::NotVariadic {
                                    is_vararg = true;
                                }
                            }

                            let fn_type = Rc::new(RefCell::new(Type::Function(
                                parameter_types.clone(),
                                ret_type,
                                is_vararg,
                                None,
                            )));
                            Some((method_name.value.clone(), fn_type, parameter_types))
                        } else {
                            None
                        }
                    };

                    if let Some((_method_name, fn_type, _)) = method_info
                        && let Statement::FunctionDecl { ty, .. } = &mut *stmt.borrow_mut()
                    {
                        *ty = Some(fn_type.clone());
                    }
                    self.process_decl(stmt.clone());
                }
                self.leave_scope();
                self.current_owner = prev_owner;
                self.validate_class_member_overrides(name, superclass, body);
                self.validate_member_access_levels(
                    container_access.clone(),
                    is_container_class,
                    body,
                );
                self.validate_setter_access_conflicts(body);
                self.check_protocol_conformances(&name.value, name.as_ref(), conformances, true);
            }
            Statement::EnumDecl {
                name,
                cases: ast_cases,
                body,
                scope,
                generic_parameters,
                ..
            } => {
                let Some(symbol) = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().name_table.get(&name.value).cloned())
                else {
                    return;
                };

                let enum_ty = Rc::new(RefCell::new(Type::Enum(
                    name.value.clone(),
                    WeakSymbol(Rc::downgrade(&symbol)),
                    vec![],
                )));
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), enum_ty);

                let prev_owner = self.current_owner.replace(symbol.clone());
                self.enter_scope(scope.as_ref().unwrap().clone());
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { const_type } => {
                            let resolved = self
                                .infer_type(const_type.clone())
                                .unwrap_or_else(|| Rc::new(RefCell::new(Type::Never)));
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                resolved,
                            )))
                        }
                    };
                    self.current_scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.infer_type(default_value.clone());
                    }
                }
                if let Symbol::Enum { cases, .. } = &mut *symbol.borrow_mut() {
                    for (case_symbol, ast_case) in cases.iter().zip(ast_cases.iter()) {
                        let mut parameter_types = Vec::new();
                        for param in &ast_case.parameters {
                            let param_type = self.infer_type(param.type_expression.clone());
                            if let Some(ref param_type) = param_type {
                                parameter_types.push(param_type.clone());
                            }
                        }
                        if let Symbol::EnumCase {
                            parameter_types: param_tys,
                            ..
                        } = &mut *case_symbol.borrow_mut()
                        {
                            *param_tys = parameter_types;
                        }
                    }
                }

                for stmt in body {
                    let method_info: MethodInfo = {
                        if let Statement::FunctionDecl {
                            name: method_name,
                            parameters,
                            return_type,
                            ..
                        } = &*stmt.borrow()
                        {
                            let ret_type = if let Some(return_type_expr) = return_type {
                                self.infer_type(return_type_expr.clone())
                                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
                            } else {
                                Rc::new(RefCell::new(Type::Void))
                            };

                            let mut parameter_types = Vec::new();
                            let mut is_vararg = false;
                            for param in parameters.iter() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    is_vararg = true;
                                    continue;
                                }
                                let param_type =
                                    self.infer_type(param.borrow().type_expression.clone());
                                if let Some(ref param_type) = param_type {
                                    param.borrow_mut().ty = Some(param_type.clone());
                                    parameter_types.push(param_type.clone());
                                }
                                if param.borrow().variadic_kind != VariadicKind::NotVariadic {
                                    is_vararg = true;
                                }
                            }

                            let fn_type = Rc::new(RefCell::new(Type::Function(
                                parameter_types.clone(),
                                ret_type,
                                is_vararg,
                                None,
                            )));
                            Some((method_name.value.clone(), fn_type, parameter_types))
                        } else {
                            None
                        }
                    };

                    if let Some((_method_name, fn_type, _)) = method_info
                        && let Statement::FunctionDecl { ty, .. } = &mut *stmt.borrow_mut()
                    {
                        *ty = Some(fn_type.clone());
                    }
                    self.process_decl(stmt.clone());
                }
                self.leave_scope();
                self.current_owner = prev_owner;
            }
            Statement::InitDecl {
                parameters,
                body,
                scope,
                ty,
                is_failable,
                ..
            } => {
                let ret_type = if *is_failable {
                    self.current_scope
                        .as_ref()
                        .and_then(|s| s.borrow().get_type("Optional"))
                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
                } else {
                    Rc::new(RefCell::new(Type::Void))
                };
                let mut parameter_types = Vec::new();
                for param in parameters.iter() {
                    let param_type = self.infer_type(param.borrow().type_expression.clone());
                    if let Some(ref param_type) = param_type {
                        param.borrow_mut().ty = Some(param_type.clone());
                        parameter_types.push(param_type.clone());
                        if let Some(default_value) = &param.borrow().default_value {
                            self.infer_expression_type(default_value.clone(), param_type.clone());
                        }
                    }
                }
                let fn_type = Rc::new(RefCell::new(Type::Function(
                    parameter_types,
                    ret_type,
                    false,
                    None,
                )));
                *ty = Some(fn_type.clone());

                self.enter_scope(scope.as_ref().unwrap().clone());

                if let FunctionBody::Statements(stmts) = &*body.borrow() {
                    for s in stmts {
                        self.process_decl(s.clone());
                    }
                }

                self.leave_scope();
            }
            Statement::DeinitDecl {
                body, scope, ty, ..
            } => {
                let fn_type = Rc::new(RefCell::new(Type::Function(
                    vec![],
                    Rc::new(RefCell::new(Type::Void)),
                    false,
                    None,
                )));
                *ty = Some(fn_type.clone());

                self.enter_scope(scope.as_ref().unwrap().clone());

                if let FunctionBody::Statements(stmts) = &*body.borrow() {
                    for s in stmts {
                        self.process_decl(s.clone());
                    }
                }

                self.leave_scope();
            }
            Statement::VariableDecl {
                name,
                type_expression,
                initializer,
                accessors,
                ty,
                ..
            } => {
                if let Some(type_expr) = type_expression {
                    let annotated = self.infer_type(type_expr.clone());
                    if let Some(annotated) = annotated {
                        if let Some(init) = initializer {
                            self.check_type_with_expected(
                                init.clone(),
                                annotated.clone(),
                                name.as_ref(),
                            );
                        }
                        *ty = Some(annotated.clone());
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(name.value.clone(), annotated);
                    }
                } else if let Some(init) = initializer {
                    let init_ty = self.infer_type(init.clone());
                    if let Some(init_ty) = init_ty {
                        *ty = Some(init_ty.clone());
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(name.value.clone(), init_ty);
                    }
                }
                for accessor in accessors {
                    for stmt in &accessor.body {
                        self.process_decl(stmt.clone());
                    }
                }
            }
            Statement::ExternBlock { items, .. } => {
                for item in items {
                    self.process_decl(item.clone());
                }
            }
            Statement::ExternDecl { statement, .. } => {
                self.process_decl(statement.clone());
            }
            Statement::ProtocolDecl {
                name,
                conformances,
                members,
                scope,
                generic_parameters,
                ..
            } => {
                let Some(symbol) = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().name_table.get(&name.value).cloned())
                else {
                    return;
                };

                let protocol_ty = Rc::new(RefCell::new(Type::Protocol(
                    name.value.clone(),
                    WeakSymbol(Rc::downgrade(&symbol)),
                    vec![],
                )));
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), protocol_ty);

                for conformance in conformances.iter() {
                    self.infer_type(conformance.clone());
                }

                self.enter_scope(scope.as_ref().unwrap().clone());
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { const_type } => {
                            let resolved = self
                                .infer_type(const_type.clone())
                                .unwrap_or_else(|| Rc::new(RefCell::new(Type::Never)));
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                resolved,
                            )))
                        }
                    };
                    self.current_scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.infer_type(default_value.clone());
                    }
                }
                for member in members {
                    match member {
                        ProtocolMember::Method { decl, .. } => {
                            if let Statement::FunctionDecl {
                                name: _method_name,
                                parameters,
                                return_type,
                                throws_types,
                                body,
                                scope: fn_scope,
                                ty,
                                ..
                            } = &mut *decl.borrow_mut()
                            {
                                let ret_type = if let Some(return_type_expr) = return_type {
                                    self.infer_type(return_type_expr.clone())
                                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
                                } else {
                                    Rc::new(RefCell::new(Type::Void))
                                };

                                let mut parameter_types = Vec::new();
                                let mut is_vararg = false;
                                for param in parameters.iter() {
                                    if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                        is_vararg = true;
                                        continue;
                                    }
                                    let param_type =
                                        self.infer_type(param.borrow().type_expression.clone());
                                    if let Some(ref param_type) = param_type {
                                        param.borrow_mut().ty = Some(param_type.clone());
                                        parameter_types.push(param_type.clone());
                                    }
                                    if param.borrow().variadic_kind != VariadicKind::NotVariadic {
                                        is_vararg = true;
                                    }
                                }

                                let throws_ty = throws_types.as_ref().map(|types| {
                                    types.iter().filter_map(|t| self.infer_type(t.clone())).collect()
                                });

                                let fn_type = Rc::new(RefCell::new(Type::Function(
                                    parameter_types,
                                    ret_type,
                                    is_vararg,
                                    throws_ty,
                                )));
                                *ty = Some(fn_type.clone());

                                self.enter_scope(fn_scope.as_ref().unwrap().clone());
                                match &*body.borrow() {
                                    FunctionBody::Statements(stmts) => {
                                        for s in stmts {
                                            self.process_decl(s.clone());
                                        }
                                    }
                                    FunctionBody::Expression(expr) => {
                                        self.process_function_decl_in_expr(expr.clone());
                                    }
                                    FunctionBody::None => {}
                                }
                                self.leave_scope();
                            }
                        }
                        ProtocolMember::Property {
                            name: prop_name,
                            type_expression,
                            ..
                        } => {
                            let prop_ty = self.infer_type(type_expression.clone());
                            if let Some(prop_ty) = prop_ty {
                                self.current_scope
                                    .as_ref()
                                    .unwrap()
                                    .borrow_mut()
                                    .set_type(prop_name.value.clone(), prop_ty);
                            }
                        }
                        ProtocolMember::AssociatedType {
                            name, constraints, ..
                        } => {
                            let at_type =
                                Rc::new(RefCell::new(Type::GenericParam(name.value.clone())));
                            self.current_scope
                                .as_ref()
                                .unwrap()
                                .borrow_mut()
                                .set_type(name.value.clone(), at_type);
                            for constraint in constraints {
                                self.infer_type(constraint.clone());
                            }
                        }
                        ProtocolMember::TypeAlias {
                            name,
                            type_expression,
                            ..
                        } => {
                            if let Some(ty) = self.infer_type(type_expression.clone()) {
                                if let Some(scope) = self.current_scope.as_ref() {
                                    scope.borrow_mut().set_type(name.value.clone(), ty);
                                }
                            }
                        }
                        ProtocolMember::Subscript { .. } => {}
                    }
                }
                self.leave_scope();
            }
            Statement::ExtensionDecl {
                type_name,
                conformances,
                body,
                type_arguments,
                where_clause,
                ..
            } => {
                let Some(target_sym) = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().name_table.get(&type_name.value).cloned())
                else {
                    return;
                };

                let target_ty = match &*target_sym.borrow() {
                    Symbol::Struct { name, .. } => Some(Rc::new(RefCell::new(Type::Struct(
                        name.clone(),
                        WeakSymbol(Rc::downgrade(&target_sym)),
                        vec![],
                    )))),
                    Symbol::Class { name, .. } => Some(Rc::new(RefCell::new(Type::Class(
                        name.clone(),
                        WeakSymbol(Rc::downgrade(&target_sym)),
                        vec![],
                    )))),
                    Symbol::Enum { name, .. } => Some(Rc::new(RefCell::new(Type::Enum(
                        name.clone(),
                        WeakSymbol(Rc::downgrade(&target_sym)),
                        vec![],
                    )))),
                    Symbol::Protocol { name, .. } => Some(Rc::new(RefCell::new(Type::Protocol(
                        name.clone(),
                        WeakSymbol(Rc::downgrade(&target_sym)),
                        vec![],
                    )))),
                    _ => None,
                };

                for conformance in conformances.iter() {
                    self.infer_type(conformance.clone());
                }

                let target_scope = {
                    target_sym.borrow().get_decl().ok().flatten().and_then(|d| {
                        let stmt = d.borrow();
                        match &*stmt {
                            Statement::StructDecl { scope, .. }
                            | Statement::ClassDecl { scope, .. }
                            | Statement::EnumDecl { scope, .. }
                            | Statement::ProtocolDecl { scope, .. } => scope.clone(),
                            _ => None,
                        }
                    })
                };

                let parent_generic_params: Vec<String> = target_sym
                    .borrow()
                    .get_decl()
                    .ok()
                    .flatten()
                    .map(|d| {
                        let stmt = d.borrow();
                        match &*stmt {
                            Statement::StructDecl {
                                generic_parameters,
                                ..
                            }
                            | Statement::ClassDecl {
                                generic_parameters,
                                ..
                            }
                            | Statement::EnumDecl {
                                generic_parameters,
                                ..
                            } => generic_parameters
                                .iter()
                                .map(|gp| gp.name.value.clone())
                                .collect(),
                            _ => vec![],
                        }
                    })
                    .unwrap_or_default();

                if let Some(ref scope) = target_scope {
                    self.enter_scope(scope.clone());
                    if let Some(ref target_ty) = target_ty {
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type("Self".to_string(), target_ty.clone());
                    }

                    if let Some(type_arguments) = type_arguments {
                        for (i, ta) in type_arguments.iter().enumerate() {
                            if i < parent_generic_params.len() {
                                if let Some(concrete_ty) = self.infer_type(ta.clone()) {
                                    self.current_scope
                                        .as_ref()
                                        .unwrap()
                                        .borrow_mut()
                                        .set_type(parent_generic_params[i].clone(), concrete_ty);
                                }
                            }
                        }
                    }

                    if let Some(wc) = where_clause {
                        self.validate_where_clause(wc);
                    }

                    for stmt in body {
                        let method_info: MethodInfo = {
                            if let Statement::FunctionDecl {
                                name: method_name,
                                parameters,
                                return_type,
                                ..
                            } = &*stmt.borrow()
                            {
                                let ret_type = if let Some(return_type_expr) = return_type {
                                    self.infer_type(return_type_expr.clone())
                                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
                                } else {
                                    Rc::new(RefCell::new(Type::Void))
                                };

                                let mut parameter_types = Vec::new();
                                let mut is_vararg = false;
                                for param in parameters.iter() {
                                    if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                        is_vararg = true;
                                        continue;
                                    }
                                    let param_type =
                                        self.infer_type(param.borrow().type_expression.clone());
                                    if let Some(ref param_type) = param_type {
                                        param.borrow_mut().ty = Some(param_type.clone());
                                        parameter_types.push(param_type.clone());
                                    }
                                    if param.borrow().variadic_kind != VariadicKind::NotVariadic {
                                        is_vararg = true;
                                    }
                                }

                                let fn_type = Rc::new(RefCell::new(Type::Function(
                                    parameter_types.clone(),
                                    ret_type,
                                    is_vararg,
                                    None,
                                )));
                                Some((method_name.value.clone(), fn_type, parameter_types))
                            } else {
                                None
                            }
                        };

                        if let Some((_method_name, fn_type, _)) = method_info
                            && let Statement::FunctionDecl { ty, .. } = &mut *stmt.borrow_mut()
                        {
                            *ty = Some(fn_type.clone());
                        }

                        self.process_decl(stmt.clone());
                    }
                    self.leave_scope();
                    self.check_protocol_conformances(
                        &type_name.value,
                        type_name.as_ref(),
                        conformances,
                        false,
                    );
                }
            }
            Statement::TypeAlias {
                type_expression,
                name,
                ..
            } => {
                if let Some(ty) = self.infer_type(type_expression.clone()) {
                    if let Some(scope) = self.current_scope.as_ref() {
                        scope.borrow_mut().set_type(name.value.clone(), ty);
                    }
                }
            }
            Statement::SubscriptDecl {
                parameters,
                return_type_expression,
                accessors,
                ty,
                ..
            } => {
                let ret_type = self
                    .infer_type(return_type_expression.clone())
                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
                let mut param_types = Vec::new();
                for param in parameters.iter() {
                    let pt = self.infer_type(param.borrow().type_expression.clone());
                    if let Some(ref pt) = pt {
                        param.borrow_mut().ty = Some(pt.clone());
                        param_types.push(pt.clone());
                    }
                }
                let fn_type = Rc::new(RefCell::new(Type::Function(param_types, ret_type, false, None)));
                *ty = Some(fn_type);
                for accessor in accessors {
                    for s in &accessor.body {
                        self.process_decl(s.clone());
                    }
                }
            }
            Statement::ModuleDecl { body, scope, .. } => {
                if let Some(s) = scope.clone() {
                    self.enter_scope(s);
                    for stmt in body {
                        self.process_decl(stmt.clone());
                    }
                    self.leave_scope();
                }
            }
            Statement::ConditionalBlock { clauses } => {
                for clause in clauses {
                    for stmt in &clause.body {
                        self.process_decl(stmt.clone());
                    }
                }
            }
            _ => {}
        }
    }

    fn process_function_decl_in_expr(&mut self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Closure { body, .. } = &*expr.borrow() {
            for stmt in body {
                self.process_decl(stmt.clone());
            }
        }
    }

    fn resolve_statement(&mut self, statement: Rc<RefCell<Statement>>) {
        match &mut *statement.borrow_mut() {
            Statement::VariableDecl {
                name,
                type_expression,
                initializer,
                accessors,
                ty,
                ..
            } => {
                if let Some(type_expr) = type_expression {
                    let annotated = self.infer_type(type_expr.clone());
                    if let Some(annotated) = annotated {
                        if let Some(init) = initializer {
                            self.check_type_with_expected(
                                init.clone(),
                                annotated.clone(),
                                name.as_ref(),
                            );
                        }
                        *ty = Some(annotated.clone());
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(name.value.clone(), annotated);
                    }
                } else if let Some(init) = initializer {
                    let init_ty = self.infer_type(init.clone());
                    if let Some(init_ty) = init_ty {
                        *ty = Some(init_ty.clone());
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(name.value.clone(), init_ty);
                    }
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingTypeAnnotation,
                        "Variable declaration requires type annotation or initializer",
                        name.as_ref(),
                    );
                };

                if !accessors.is_empty() {
                    let saved_return_type = self.current_return_type.clone();
                    for accessor in accessors {
                        let accessor_scope = {
                            let sc = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
                            self.enter_scope(sc.clone());
                            sc
                        };
                        if let Some(var_ty) = ty {
                            accessor_scope
                                .borrow_mut()
                                .set_type(format!("_{}", name.value), var_ty.clone());
                        }
                        match accessor.kind {
                            AccessorKind::Get => {
                                if let Some(var_ty) = ty {
                                    self.current_return_type = Some(var_ty.clone());
                                }
                            }
                            AccessorKind::Set | AccessorKind::WillSet | AccessorKind::DidSet => {
                                self.current_return_type = Some(Rc::new(RefCell::new(Type::Void)));
                                let param_name = accessor
                                    .parameter
                                    .as_ref()
                                    .map(|t| t.value.clone())
                                    .unwrap_or_else(|| match accessor.kind {
                                        AccessorKind::DidSet => "oldValue".to_string(),
                                        _ => "newValue".to_string(),
                                    });
                                if let Some(var_ty) = ty {
                                    accessor_scope
                                        .borrow_mut()
                                        .set_type(param_name, var_ty.clone());
                                }
                            }
                        }
                        for stmt in &accessor.body {
                            self.resolve_statement(stmt.clone());
                        }
                        self.leave_scope();
                    }
                    self.current_return_type = saved_return_type;
                }
            }
            Statement::FunctionDecl {
                parameters,
                body,
                scope,
                ty,
                ..
            } => {
                let last_return_type = self.current_return_type.clone();

                let fn_type = if ty.is_some() {
                    ty.clone().unwrap()
                } else {
                    Rc::new(RefCell::new(Type::Function(
                        vec![],
                        Rc::new(RefCell::new(Type::Void)),
                        false,
                        None,
                    )))
                };

                let ret_type = if let Type::Function(_, ret, _, _) = &*fn_type.borrow() {
                    ret.clone()
                } else {
                    Rc::new(RefCell::new(Type::Void))
                };
                self.current_return_type = Some(ret_type.clone());
                self.enter_scope(scope.as_ref().unwrap().clone());
                self.initialized_lets.push(HashSet::new());

                for param in parameters.iter() {
                    if let Some(param_ty) = param.borrow().ty.clone() {
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(param.borrow().name.value.clone(), param_ty);
                    }
                }

                self.resolve_function_body(body.clone());

                self.initialized_lets.pop();
                self.leave_scope();
                self.current_return_type = last_return_type;
            }
            Statement::InitDecl {
                parameters,
                body,
                scope,
                is_failable,
                ty,
                ..
            } => {
                self.enter_scope(scope.as_ref().unwrap().clone());
                self.initialized_lets.push(HashSet::new());
                let saved_in_init = self.is_in_init;
                self.is_in_init = true;
                self.initialized_properties.clear();

                let saved_return_type = self.current_return_type.clone();
                if *is_failable {
                    if let Some(fn_ty) = ty
                        && let Type::Function(_, ret, _, _) = &*fn_ty.borrow()
                    {
                        self.current_return_type = Some(ret.clone());
                    }
                }

                for param in parameters.iter() {
                    if let Some(param_ty) = param.borrow().ty.clone() {
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(param.borrow().name.value.clone(), param_ty);
                    }
                }

                self.resolve_function_body(body.clone());
                self.is_in_init = saved_in_init;
                self.current_return_type = saved_return_type;
                self.initialized_lets.pop();
                self.leave_scope();
            }
            Statement::DeinitDecl { body, scope, .. } => {
                self.enter_scope(scope.as_ref().unwrap().clone());
                self.resolve_function_body(body.clone());
                self.leave_scope();
            }
            Statement::Return {
                value: Some(value), ..
            } => {
                let token = &value.borrow().token();
                if let Some(expected) = self.current_return_type.clone() {
                    self.check_type_with_expected(value.clone(), expected, token);
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::ReturnTypeMismatch,
                        "Return statement outside function",
                        token,
                    );
                }
            }
            Statement::Return { value: None, token } => {
                if let Some(expected) = self.current_return_type.clone() {
                    if !matches!(&*expected.borrow(), Type::Void) {
                        self.emit_error(
                            TrussDiagnosticCode::ReturnTypeMismatch,
                            format!(
                                "Expected return value of type {}, found `return` without value",
                                expected.borrow()
                            ),
                            token.as_ref(),
                        );
                    }
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::ReturnTypeMismatch,
                        "Return statement outside function",
                        token.as_ref(),
                    );
                }
            }
            Statement::Yield {
                value: Some(value), ..
            } => {
                let token = value.borrow().token();
                if let Some(expected) = self.current_return_type.clone() {
                    self.check_type_with_expected(value.clone(), expected, &token);
                } else if self.yield_context_depth == 0 {
                    self.emit_error(
                        TrussDiagnosticCode::YieldNotAllowedHere,
                        "Yield statement outside function or expression context",
                        &token,
                    );
                }
                self.infer_type(value.clone());
            }
            Statement::Yield { value: None, token } => {
                if let Some(expected) = self.current_return_type.clone() {
                    if !matches!(&*expected.borrow(), Type::Void) {
                        self.emit_error(
                            TrussDiagnosticCode::ReturnTypeMismatch,
                            format!(
                                "Expected return value of type {}, found `yield` without value",
                                expected.borrow()
                            ),
                            token.as_ref(),
                        );
                    }
                } else if self.yield_context_depth == 0 {
                    self.emit_error(
                        TrussDiagnosticCode::YieldNotAllowedHere,
                        "Yield statement outside function or expression context",
                        token.as_ref(),
                    );
                }
            }
            Statement::ExpressionStatement { expression } => {
                self.infer_type(expression.clone());
            }
            Statement::While {
                condition, body, ..
            } => {
                let cond_ty = self.infer_type(condition.clone());
                if let Some(cond_ty) = cond_ty
                    && *cond_ty.borrow() != Type::Bool
                {
                    self.emit_error(
                        TrussDiagnosticCode::InvalidConditionType,
                        format!("While condition must be Bool, found {}", cond_ty.borrow()),
                        &condition.borrow().token(),
                    );
                }
                self.resolve_block_expression(body);
            }
            Statement::Loop { body, .. } => {
                self.resolve_block_expression(body);
            }
            Statement::RepeatWhile {
                body, condition, ..
            } => {
                self.resolve_block_expression(body);
                let cond_ty = self.infer_type(condition.clone());
                if let Some(cond_ty) = cond_ty
                    && *cond_ty.borrow() != Type::Bool
                {
                    self.emit_error(
                        TrussDiagnosticCode::InvalidConditionType,
                        format!(
                            "Repeat-while condition must be Bool, found {}",
                            cond_ty.borrow()
                        ),
                        &condition.borrow().token(),
                    );
                }
            }
            Statement::For { iterator, body, .. } => {
                let _ = self.infer_type(iterator.clone());
                self.resolve_block_expression(body);
            }
            Statement::ExternBlock { items, .. } => {
                for item in items {
                    self.resolve_statement(item.clone());
                }
            }
            Statement::ExternDecl { statement, .. } => {
                self.resolve_statement(statement.clone());
            }
            Statement::StructDecl {
                body,
                conformances,
                scope,
                ..
            } => {
                for conformance in conformances {
                    self.infer_type(conformance.clone());
                }
                if let Some(s) = scope.as_ref() {
                    self.enter_scope(s.clone());
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                    self.leave_scope();
                } else {
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                }
            }
            Statement::ClassDecl {
                body,
                conformances,
                scope,
                ..
            } => {
                for conformance in conformances {
                    self.infer_type(conformance.clone());
                }
                if let Some(s) = scope.as_ref() {
                    self.enter_scope(s.clone());
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                    self.leave_scope();
                } else {
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                }
            }
            Statement::ProtocolDecl { scope, members, .. } => {
                self.enter_scope(scope.as_ref().unwrap().clone());
                for member in members {
                    match member {
                        ProtocolMember::Method { decl, .. } => {
                            if let Statement::FunctionDecl {
                                parameters,
                                body,
                                scope: fn_scope,
                                ty,
                                ..
                            } = &mut *decl.borrow_mut()
                            {
                                let last_return_type = self.current_return_type.clone();
                                let fn_type = if ty.is_some() {
                                    ty.clone().unwrap()
                                } else {
                                    Rc::new(RefCell::new(Type::Function(
                                        vec![],
                                        Rc::new(RefCell::new(Type::Void)),
                                        false,
                                        None,
                                    )))
                                };
                                let ret_type = if let Type::Function(_, ret, _, _) = &*fn_type.borrow()
                                {
                                    ret.clone()
                                } else {
                                    Rc::new(RefCell::new(Type::Void))
                                };
                                self.current_return_type = Some(ret_type.clone());
                                self.enter_scope(fn_scope.as_ref().unwrap().clone());
                                self.initialized_lets.push(HashSet::new());
                                for param in parameters.iter() {
                                    if let Some(param_ty) = param.borrow().ty.clone() {
                                        self.current_scope
                                            .as_ref()
                                            .unwrap()
                                            .borrow_mut()
                                            .set_type(param.borrow().name.value.clone(), param_ty);
                                    }
                                }
                                self.resolve_function_body(body.clone());
                                self.initialized_lets.pop();
                                self.leave_scope();
                                self.current_return_type = last_return_type;
                            }
                        }
                        ProtocolMember::Property { .. } => {}
                        ProtocolMember::AssociatedType { .. } => {}
                        ProtocolMember::TypeAlias {
                            name,
                            type_expression,
                            ..
                        } => {
                            if let Some(ty) = self.infer_type(type_expression.clone()) {
                                if let Some(scope) = self.current_scope.as_ref() {
                                    scope.borrow_mut().set_type(name.value.clone(), ty);
                                }
                            }
                        }
                        ProtocolMember::Subscript { .. } => {}
                    }
                }
                self.leave_scope();
            }
            Statement::EnumDecl { body, scope, .. } => {
                if let Some(s) = scope.as_ref() {
                    self.enter_scope(s.clone());
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                    self.leave_scope();
                } else {
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                }
            }
            Statement::ExtensionDecl {
                type_name, body, ..
            } => {
                let target_scope = {
                    self.current_scope
                        .as_ref()
                        .and_then(|scope| scope.borrow().get_symbol(&type_name.value))
                        .and_then(|sym| {
                            let decl = sym.borrow().get_decl().ok().flatten();
                            decl.and_then(|d| {
                                let stmt = d.borrow();
                                match &*stmt {
                                    Statement::StructDecl { scope, .. }
                                    | Statement::ClassDecl { scope, .. }
                                    | Statement::EnumDecl { scope, .. }
                                    | Statement::ProtocolDecl { scope, .. } => scope.clone(),
                                    _ => None,
                                }
                            })
                        })
                };

                if let Some(ref scope) = target_scope {
                    self.enter_scope(scope.clone());
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                    self.leave_scope();
                } else {
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                }
            }
            Statement::TypeAlias {
                type_expression,
                name,
                ..
            } => {
                if let Some(ty) = self.infer_type(type_expression.clone()) {
                    if let Some(scope) = self.current_scope.as_ref() {
                        scope.borrow_mut().set_type(name.value.clone(), ty);
                    }
                }
            }
            Statement::Guard {
                condition,
                else_body,
                ..
            } => {
                let _cond_ty = self.infer_type(condition.clone());
                let binding_types = {
                    let cond = condition.borrow();
                    if let Expression::Case {
                        enum_type,
                        case_name,
                        bindings,
                        expression,
                        ..
                    } = &*cond
                    {
                        if !bindings.is_empty() {
                            let enum_name = enum_type.as_ref().map(|t| t.value.as_str());
                            if let Some(expr_ty) = self.infer_type(expression.clone()) {
                                self.resolve_enum_case_from_type(
                                    &expr_ty, enum_name, case_name, bindings,
                                )
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(ref param_types) = binding_types {
                    if let Expression::Case { bindings, .. } = &*condition.borrow() {
                        let current_scope = self.current_scope.clone();
                        if let Some(scope) = current_scope {
                            Self::set_binding_types(bindings, param_types, &scope);
                        }
                    }
                }
                self.resolve_block_expression(else_body);
            }
            Statement::Fallthrough { .. } | Statement::Break { .. } => {}
            Statement::Defer { body, .. } => {
                self.resolve_block_expression(body);
            }
            Statement::Throw { exception, .. } => {
                self.infer_type(exception.clone());
            }
            Statement::SubscriptDecl {
                parameters,
                return_type_expression,
                accessors,
                ..
            } => {
                let ret_type = self
                    .infer_type(return_type_expression.clone())
                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
                let saved_return_type = self.current_return_type.clone();
                for accessor in accessors {
                    let acc_scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
                    self.enter_scope(acc_scope.clone());
                    for param in parameters.iter() {
                        let param_name = param.borrow().name.value.clone();
                        if param_name != "_" {
                            if let Some(param_ty) = param.borrow().ty.clone() {
                                acc_scope.borrow_mut().set_type(param_name, param_ty);
                            }
                        }
                    }
                    match accessor.kind {
                        AccessorKind::Get => {
                            self.current_return_type = Some(ret_type.clone());
                        }
                        AccessorKind::Set | AccessorKind::WillSet | AccessorKind::DidSet => {
                            self.current_return_type = Some(Rc::new(RefCell::new(Type::Void)));
                            let param_name = accessor
                                .parameter
                                .as_ref()
                                .map(|t| t.value.clone())
                                .unwrap_or_else(|| match accessor.kind {
                                    AccessorKind::DidSet => "oldValue".to_string(),
                                    _ => "newValue".to_string(),
                                });
                            acc_scope
                                .borrow_mut()
                                .set_type(param_name, ret_type.clone());
                        }
                    }
                    for s in &accessor.body {
                        self.resolve_statement(s.clone());
                    }
                    self.leave_scope();
                }
                self.current_return_type = saved_return_type;
            }
            Statement::ModuleDecl { body, scope, .. } => {
                if let Some(s) = scope.clone() {
                    self.enter_scope(s);
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                    self.leave_scope();
                }
            }
            Statement::ConditionalBlock { clauses } => {
                for clause in clauses {
                    for stmt in &clause.body {
                        self.resolve_statement(stmt.clone());
                    }
                }
            }
            Statement::PragmaError { .. } | Statement::PragmaWarning { .. } => {}
            Statement::AsmBlock {
                outputs, inputs, ..
            } => {
                for operand in outputs.iter().chain(inputs.iter()) {
                    self.infer_type(operand.expression.clone());
                }
            }
            _ => {}
        }
    }

    fn resolve_block_expression(&mut self, body: &[Rc<RefCell<Statement>>]) {
        for stmt in body {
            self.resolve_statement(stmt.clone());
        }
    }

    fn get_block_type(&mut self, body: &[Rc<RefCell<Statement>>]) -> Option<Rc<RefCell<Type>>> {
        let mut last_ty = Rc::new(RefCell::new(Type::Void));
        for stmt in body.iter() {
            if let Some(ty) = self.infer_statement_type(stmt.clone()) {
                last_ty = ty;
            }
        }
        Some(last_ty)
    }

    fn find_max_shorthand(&self, body: &[Rc<RefCell<Statement>>]) -> Option<u32> {
        let mut max: Option<u32> = None;
        for stmt in body {
            match &*stmt.borrow() {
                Statement::ExpressionStatement { expression } => {
                    self.find_shorthand_in_expr(expression, &mut max);
                }
                Statement::Return {
                    value: Some(val), ..
                } => {
                    self.find_shorthand_in_expr(val, &mut max);
                }
                _ => {}
            }
        }
        max
    }

    fn find_shorthand_in_expr(&self, expr: &Rc<RefCell<Expression>>, max: &mut Option<u32>) {
        match &*expr.borrow() {
            Expression::ShorthandArgument { index, .. } => match max {
                Some(m) if *index > *m => *max = Some(*index),
                None => *max = Some(*index),
                _ => {}
            },
            Expression::Binary { left, right, .. } => {
                self.find_shorthand_in_expr(left, max);
                self.find_shorthand_in_expr(right, max);
            }
            Expression::Unary { expression, .. } => {
                self.find_shorthand_in_expr(expression, max);
            }
            Expression::Call { parameters, .. } => {
                for param in parameters {
                    self.find_shorthand_in_expr(&param.expression, max);
                }
            }
            _ => {}
        }
    }

    fn resolve_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) {
        match &mut *body.borrow_mut() {
            FunctionBody::Statements(statements) => {
                for stmt in statements.iter() {
                    self.resolve_statement(stmt.clone());
                }
                if let Some(expected) = self.current_return_type.clone() {
                    if !matches!(&*expected.borrow(), Type::Void) {
                        if let Some(last_stmt) = statements.last() {
                            if let Statement::ExpressionStatement { expression } =
                                &*last_stmt.borrow()
                            {
                                let token = &expression.borrow().token();
                                self.check_type_with_expected(expression.clone(), expected, token);
                            }
                        }
                    }
                }
            }
            FunctionBody::Expression(expression) => {
                if let Some(expected) = self.current_return_type.clone() {
                    let token = &expression.borrow().token();
                    self.check_type_with_expected(expression.clone(), expected, token);
                }
            }
            FunctionBody::None => {}
        }
    }

    fn resolve_type_name(&self, name: &str, token: &Token) -> Option<Rc<RefCell<Type>>> {
        match name {
            "Int8" => Some(Rc::new(RefCell::new(Type::Int8))),
            "Int16" => Some(Rc::new(RefCell::new(Type::Int16))),
            "Int32" => Some(Rc::new(RefCell::new(Type::Int32))),
            "Int64" => Some(Rc::new(RefCell::new(Type::Int64))),
            "Int128" => Some(Rc::new(RefCell::new(Type::Int128))),
            "UInt8" => Some(Rc::new(RefCell::new(Type::UInt8))),
            "UInt16" => Some(Rc::new(RefCell::new(Type::UInt16))),
            "UInt32" => Some(Rc::new(RefCell::new(Type::UInt32))),
            "UInt64" => Some(Rc::new(RefCell::new(Type::UInt64))),
            "UInt128" => Some(Rc::new(RefCell::new(Type::UInt128))),
            "Float32" => Some(Rc::new(RefCell::new(Type::Float32))),
            "Float64" => Some(Rc::new(RefCell::new(Type::Float64))),
            "Bool" => Some(Rc::new(RefCell::new(Type::Bool))),
            "Void" => Some(Rc::new(RefCell::new(Type::Void))),
            "Char" => Some(Rc::new(RefCell::new(Type::Char))),
            "Never" => Some(Rc::new(RefCell::new(Type::Never))),
            "Pointer" => Some(Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                Type::Void,
            )))))),
            _ => {
                if let Some(current_scope) = &self.current_scope
                    && let Some(ty) = current_scope.borrow().get_type(name)
                {
                    return Some(ty);
                }
                self.emit_error(
                    TrussDiagnosticCode::UnknownType,
                    format!("Unknown type '{}'", name),
                    token,
                );
                None
            }
        }
    }

    #[allow(dead_code)]
    fn lookup_primary_type(&self, name: &str) -> Option<Rc<RefCell<Type>>> {
        match name {
            "Int8" => Some(Rc::new(RefCell::new(Type::Int8))),
            "Int16" => Some(Rc::new(RefCell::new(Type::Int16))),
            "Int32" => Some(Rc::new(RefCell::new(Type::Int32))),
            "Int64" => Some(Rc::new(RefCell::new(Type::Int64))),
            "Int128" => Some(Rc::new(RefCell::new(Type::Int128))),
            "UInt8" => Some(Rc::new(RefCell::new(Type::UInt8))),
            "UInt16" => Some(Rc::new(RefCell::new(Type::UInt16))),
            "UInt32" => Some(Rc::new(RefCell::new(Type::UInt32))),
            "UInt64" => Some(Rc::new(RefCell::new(Type::UInt64))),
            "UInt128" => Some(Rc::new(RefCell::new(Type::UInt128))),
            "Float32" => Some(Rc::new(RefCell::new(Type::Float32))),
            "Float64" => Some(Rc::new(RefCell::new(Type::Float64))),
            "Bool" => Some(Rc::new(RefCell::new(Type::Bool))),
            "Void" => Some(Rc::new(RefCell::new(Type::Void))),
            "Char" => Some(Rc::new(RefCell::new(Type::Char))),
            "Never" => Some(Rc::new(RefCell::new(Type::Never))),
            "Pointer" => Some(Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                Type::Void,
            )))))),
            _ => self
                .current_scope
                .as_ref()
                .and_then(|scope| scope.borrow().get_type(name)),
        }
    }

    fn create_parameterized_type_from_truss(
        &self,
        type_name: &str,
        inner_ty: Rc<RefCell<Type>>,
    ) -> Option<Rc<RefCell<Type>>> {
        let truss_type = self.lookup_type_from_truss_package(type_name);
        let (name, symbol) = if let Some(ty) = truss_type {
            match &*ty.borrow() {
                Type::Enum(n, s, ..) | Type::Struct(n, s, ..) | Type::Class(n, s, ..) => {
                    (n.clone(), s.clone())
                }
                _ => (type_name.to_string(), WeakSymbol(std::rc::Weak::new())),
            }
        } else {
            (type_name.to_string(), WeakSymbol(std::rc::Weak::new()))
        };
        let variant = if type_name == "Optional" {
            Type::Enum(name, symbol, vec![inner_ty])
        } else if type_name == "Array" {
            Type::Struct(name, symbol, vec![inner_ty])
        } else {
            Type::Struct(name, symbol, vec![inner_ty])
        };
        Some(Rc::new(RefCell::new(variant)))
    }

    fn lookup_type_from_truss_package(&self, type_name: &str) -> Option<Rc<RefCell<Type>>> {
        if let Some(truss_pkg) = self.packages.get("Truss") {
            if let Some(truss_module) = truss_pkg.borrow().modules.get("Truss") {
                if let Some(scope) = &truss_module.borrow().scope {
                    return scope.borrow().get_type(type_name);
                }
            }
        }
        None
    }

    fn infer_if_type(&mut self, expression: Rc<RefCell<Expression>>) -> Option<Rc<RefCell<Type>>> {
        let condition;
        let then: Vec<Rc<RefCell<Statement>>>;
        let else_: Option<ElseBranch>;
        {
            let expr = expression.borrow();
            let Expression::If {
                condition: cond,
                then: t,
                else_: e,
                ..
            } = &*expr
            else {
                return None;
            };
            condition = cond.clone();
            then = t.clone();
            else_ = e.clone();
        }

        let cond_ty = self.infer_type(condition.clone())?;

        let binding_types = {
            let cond = condition.borrow();
            if let Expression::Case {
                enum_type,
                case_name,
                bindings,
                expression,
                ..
            } = &*cond
            {
                if !bindings.is_empty() {
                    if let Some(type_name) = enum_type.as_ref() {
                        self.get_enum_case_parameter_types(&type_name.value, &case_name.value)
                    } else if let Some(expr_ty) = self.infer_type(expression.clone()) {
                        self.resolve_enum_case_from_type(&expr_ty, None, case_name, bindings)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(ref param_types) = binding_types {
            if let Expression::Case { bindings, .. } = &*condition.borrow() {
                let current_scope = self.current_scope.clone();
                if let Some(scope) = current_scope {
                    Self::set_binding_types(bindings, param_types, &scope);
                }
            }
        }

        if *cond_ty.borrow() != Type::Bool && binding_types.is_none() {
            self.emit_error(
                TrussDiagnosticCode::InvalidConditionType,
                format!("If condition must be Bool, found {}", cond_ty.borrow()),
                &condition.borrow().token(),
            );
        }

        self.yield_context_depth += 1;
        let then_ty = self.get_block_type(&then)?;
        self.yield_context_depth -= 1;
        if let Some(else_branch) = else_ {
            let else_ty = match else_branch {
                ElseBranch::Block(body) => {
                    self.yield_context_depth += 1;
                    let ty = self.get_block_type(&body)?;
                    self.yield_context_depth -= 1;
                    ty
                }
                ElseBranch::If(if_expr) => self.infer_if_type(if_expr)?,
            };
            if then_ty.borrow().clone() != else_ty.borrow().clone() {
                self.emit_error(
                    TrussDiagnosticCode::BranchTypeMismatch,
                    format!(
                        "If branches have different types: {} vs {}",
                        then_ty.borrow(),
                        else_ty.borrow()
                    ),
                    &condition.borrow().token(),
                );
            }
        }
        if let Expression::If { ty, .. } = &mut *expression.borrow_mut() {
            *ty = Some(then_ty.clone());
        }
        Some(then_ty)
    }

    fn infer_type(&mut self, expression: Rc<RefCell<Expression>>) -> Option<Rc<RefCell<Type>>> {
        if matches!(&*expression.borrow(), Expression::If { .. }) {
            return self.infer_if_type(expression);
        }
        let result = match &mut *expression.borrow_mut() {
            Expression::IntegerLiteral { ty, .. } => {
                if ty.is_none() {
                    *ty = Some(Rc::new(RefCell::new(Type::Int32)));
                }
                ty.clone().unwrap()
            }
            Expression::DecimalLiteral { ty, .. } => {
                if ty.is_none() {
                    *ty = Some(Rc::new(RefCell::new(Type::Float64)));
                }
                ty.clone().unwrap()
            }
            Expression::BooleanLiteral { .. } => Rc::new(RefCell::new(Type::Bool)),
            Expression::StringLiteral { ty, .. } => {
                if let Some(t) = ty.as_ref() {
                    t.clone()
                } else if let Some(current_scope) = &self.current_scope {
                    if let Some(t) = current_scope.borrow().get_type("String") {
                        *ty = Some(t.clone());
                        t
                    } else {
                        let t = Rc::new(RefCell::new(Type::Struct(
                            "String".to_string(),
                            WeakSymbol(std::rc::Weak::new()),
                            vec![],
                        )));
                        *ty = Some(t.clone());
                        t
                    }
                } else {
                    let t = Rc::new(RefCell::new(Type::Struct(
                        "String".to_string(),
                        WeakSymbol(std::rc::Weak::new()),
                        vec![],
                    )));
                    *ty = Some(t.clone());
                    t
                }
            }
            Expression::Variable { name, ty, .. } => {
                let t = self
                    .current_scope
                    .as_ref()
                    .ok_or_else(|| {
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            "No type environment available",
                            name.as_ref(),
                        );
                    })
                    .ok()?
                    .borrow()
                    .get_type(&name.value);
                let t = t.or_else(|| {
                    let scope = self.current_scope.as_ref()?;
                    let sym = scope.borrow().get_symbol(&name.value)?;
                    let binding = sym.borrow();
                    let decl = binding.get_decl().ok().flatten()?;
                    drop(binding);
                    let decl_ref = decl.borrow();
                    match &*decl_ref {
                        Statement::FunctionDecl { ty: fn_ty, .. } => fn_ty.clone(),
                        Statement::VariableDecl { ty: var_ty, .. } => var_ty.clone(),
                        _ => None,
                    }
                });
                let t = t?;
                *ty = Some(t.clone());
                t
            }
            Expression::Type { name, ty, .. } => {
                let t = self.resolve_type_name(&name.value, name.as_ref())?;
                *ty = Some(t.clone());
                t
            }
            Expression::Binary {
                left,
                operator,
                right,
                overloads,
                selected_index,
                ..
            } => {
                let left_ty = self.infer_type(left.clone())?;

                {
                    let mut right_mut = right.borrow_mut();
                    match &mut *right_mut {
                        Expression::IntegerLiteral { ty, .. }
                            if ty.is_none() && Self::is_integer_type(&left_ty.borrow()) =>
                        {
                            *ty = Some(left_ty.clone());
                        }
                        Expression::DecimalLiteral { ty, .. }
                            if ty.is_none() && Self::is_float_type(&left_ty.borrow()) =>
                        {
                            *ty = Some(left_ty.clone());
                        }
                        _ => {}
                    }
                }

                let right_ty = self.infer_type(right.clone())?;
                if let Some(result) =
                    self.check_binary(*operator, left_ty.clone(), right_ty.clone())
                {
                    result
                } else if let Some(result) = self.try_resolve_binary_operator(
                    *operator,
                    left.clone(),
                    left_ty.clone(),
                    right.clone(),
                    right_ty.clone(),
                    overloads,
                    selected_index,
                ) {
                    result
                } else {
                    let token = left.borrow().token();
                    self.emit_error(
                        TrussDiagnosticCode::InvalidOperand,
                        format!(
                            "Invalid operands for binary operator: {} and {}",
                            left_ty.borrow().clone(),
                            right_ty.borrow().clone()
                        ),
                        &token,
                    );
                    return None;
                }
            }
            Expression::Unary {
                expression,
                operator,
                overloads,
                selected_index,
                ..
            } => {
                let operand_ty = self.infer_type(expression.clone())?;
                if let Some(result) = self.check_unary(*operator, operand_ty.clone()) {
                    if matches!(operator, UnaryOperator::Inc | UnaryOperator::Dec) {
                        self.check_writable(expression.clone());
                    }
                    result
                } else if let Some(result) = self.try_resolve_unary_operator(
                    *operator,
                    expression.clone(),
                    operand_ty.clone(),
                    overloads,
                    selected_index,
                ) {
                    result
                } else {
                    let token = expression.borrow().token();
                    self.emit_error(
                        TrussDiagnosticCode::InvalidOperand,
                        format!(
                            "Invalid operand for unary operator: {}",
                            operand_ty.borrow().clone()
                        ),
                        &token,
                    );
                    return None;
                }
            }
            Expression::Call {
                callee,
                type_parameters,
                parameters,
                overloads,
                selected_index,
                ..
            } => {
                let callee_type = self.infer_type(callee.clone());

                let callee_type = callee_type.or_else(|| {
                    if let Expression::Variable { name, .. } = &*callee.borrow() {
                        self.resolve_type_name(&name.value, name.as_ref())
                    } else {
                        None
                    }
                });

                let callee_type = callee_type?;

                if !overloads.is_empty() {
                    if let Some(best) = self.resolve_overloaded_call(
                        callee.clone(),
                        parameters,
                        overloads,
                        selected_index,
                    ) {
                        return Some(best);
                    }
                    return None;
                }

                if overloads.is_empty()
                    && let Expression::MemberAccess { object, member, .. } = &*callee.borrow()
                {
                    let member_name = member.value.clone();
                    let object_clone = object.clone();
                    if let Some(methods) = self.collect_method_overloads(object_clone, &member_name)
                    {
                        if methods.len() > 1 {
                            *overloads = methods;
                            if let Some(best) = self.resolve_overloaded_call(
                                callee.clone(),
                                parameters,
                                overloads,
                                selected_index,
                            ) {
                                return Some(best);
                            }
                            return None;
                        }
                    }
                }

                match &*callee_type.borrow() {
                    Type::Function(param_tys, ret_ty, is_vararg, _) => {
                        if let Some(decl) = self
                            .get_function_decl_from_callee(callee.clone())
                            .or_else(|| {
                                if let Expression::Variable { name, .. } = &*callee.borrow() {
                                    let scope_ref = self.current_scope.as_ref()?.borrow();
                                    let sym = scope_ref.get_symbol(&name.value)?;
                                    sym.borrow().get_decl().ok().flatten()
                                } else {
                                    None
                                }
                            })
                        {
                            if let Statement::FunctionDecl { attributes, .. } = &*decl.borrow() {
                                if attributes.iter().any(|a| a.name == "internalUsed") {
                                    self.emit_error(
                                        TrussDiagnosticCode::InternalUsedReferenced,
                                        "Referencing an internal-used declaration which is intended for internal use only",
                                        &callee.borrow().token(),
                                    );
                                }
                            }
                            let callee_token = callee.borrow().token();
                            let decl_params = match &*decl.borrow() {
                                Statement::FunctionDecl { parameters, .. } => parameters.clone(),
                                _ => vec![],
                            };
                            if !decl_params.is_empty() {
                                self.reorder_call_parameters(
                                    parameters,
                                    &decl_params,
                                    &callee_token,
                                );
                            }
                        }

                        if !*is_vararg && parameters.len() != param_tys.len() {
                            let token = &callee.borrow().token();
                            self.emit_error(
                                TrussDiagnosticCode::ArgumentCountMismatch,
                                format!(
                                    "Expected {} arguments but found {}",
                                    param_tys.len(),
                                    parameters.len()
                                ),
                                token,
                            );
                        } else if *is_vararg && parameters.len() < param_tys.len() {
                            let token = &callee.borrow().token();
                            self.emit_error(
                                TrussDiagnosticCode::ArgumentCountMismatch,
                                format!(
                                    "Expected at least {} arguments but found {}",
                                    param_tys.len(),
                                    parameters.len()
                                ),
                                token,
                            );
                        }

                        let generic_mapping = {
                            let mut mapping: std::collections::HashMap<String, Rc<RefCell<Type>>> =
                                std::collections::HashMap::new();
                            let has_generic_param = param_tys
                                .iter()
                                .any(|pt| matches!(&*pt.borrow(), Type::GenericParam(_)));
                            if has_generic_param
                                && type_parameters.is_some()
                                && !type_parameters.as_ref().unwrap().is_empty()
                            {
                                let func_decl_opt =
                                    self.get_function_decl_from_callee(callee.clone());
                                let func_decl = func_decl_opt.or_else(|| {
                                    if let Expression::Variable { name, .. } = &*callee.borrow() {
                                        let scope_ref = self.current_scope.as_ref()?.borrow();
                                        let sym = scope_ref.get_symbol(&name.value)?;
                                        sym.borrow().get_decl().ok().flatten()
                                    } else {
                                        None
                                    }
                                });
                                if let Some(ref decl) = func_decl {
                                    let tp = type_parameters.clone();
                                    self.infer_generic_params_from_call(
                                        decl, &tp, param_tys, parameters,
                                    )
                                } else {
                                    None
                                }
                            } else if has_generic_param {
                                for (i, param) in parameters.iter().enumerate() {
                                    if i >= param_tys.len() {
                                        break;
                                    }
                                    let param_ty = &param_tys[i];
                                    let arg_ty = self.infer_type(param.expression.clone())?;
                                    Self::collect_generic_mappings(
                                        param_ty.clone(),
                                        arg_ty,
                                        &mut mapping,
                                    );
                                }
                                if mapping.is_empty() {
                                    None
                                } else {
                                    Some(mapping)
                                }
                            } else {
                                None
                            }
                        };

                        let resolved_ret_ty = if let Some(ref mapping) = generic_mapping {
                            Self::substitute_generic_params(ret_ty.clone(), mapping)
                        } else {
                            ret_ty.clone()
                        };

                        for (i, param) in parameters.iter().enumerate() {
                            if i < param_tys.len() {
                                let expected_ty = if let Some(ref mapping) = generic_mapping {
                                    Self::substitute_generic_params(param_tys[i].clone(), mapping)
                                } else {
                                    param_tys[i].clone()
                                };
                                self.infer_expression_type(param.expression.clone(), expected_ty);

                                if let Some(ref decl) = {
                                    self.get_function_decl_from_callee(callee.clone()).or_else(
                                        || {
                                            if let Expression::Variable { name, .. } =
                                                &*callee.borrow()
                                            {
                                                let scope_ref =
                                                    self.current_scope.as_ref()?.borrow();
                                                let sym = scope_ref.get_symbol(&name.value)?;
                                                sym.borrow().get_decl().ok().flatten()
                                            } else {
                                                None
                                            }
                                        },
                                    )
                                } {
                                    self.check_parameter_label(param, decl, i);
                                }
                            }
                        }
                        resolved_ret_ty
                    }
                    Type::Struct(struct_name, ..) => {
                        let (init_params_info, is_failable_init) = {
                            let scope = self.current_scope.as_ref().unwrap().borrow();
                            if let Some(symbol) = scope.get_symbol(struct_name)
                                && let Symbol::Struct { constructors, .. } = &*symbol.borrow()
                            {
                                let mut found_failable = false;
                                let result = constructors.iter().find_map(|constructor| {
                                    if let Ok(Some(decl)) = constructor.borrow().get_decl()
                                        && let Statement::InitDecl {
                                            ty: Some(init_ty),
                                            is_failable,
                                            ..
                                        } = &*decl.borrow()
                                        && let Type::Function(param_tys, _, is_vararg, None) =
                                            &*init_ty.borrow()
                                    {
                                        found_failable = *is_failable;
                                        Some((decl.clone(), param_tys.clone(), *is_vararg))
                                    } else {
                                        None
                                    }
                                });
                                (result, found_failable)
                            } else {
                                (None, false)
                            }
                        };
                        if let Some((decl, param_tys, is_vararg)) = init_params_info {
                            let callee_token = callee.borrow().token();
                            let decl_params = match &*decl.borrow() {
                                Statement::InitDecl { parameters, .. } => parameters.clone(),
                                _ => vec![],
                            };
                            if !decl_params.is_empty() {
                                self.reorder_call_parameters(
                                    parameters,
                                    &decl_params,
                                    &callee_token,
                                );
                            }
                            if !is_vararg && parameters.len() != param_tys.len() {
                                self.emit_error(
                                    TrussDiagnosticCode::ArgumentCountMismatch,
                                    format!(
                                        "Expected {} arguments but found {}",
                                        param_tys.len(),
                                        parameters.len()
                                    ),
                                    &callee.borrow().token(),
                                );
                            } else if is_vararg && parameters.len() < param_tys.len() {
                                self.emit_error(
                                    TrussDiagnosticCode::ArgumentCountMismatch,
                                    format!(
                                        "Expected at least {} arguments but found {}",
                                        param_tys.len(),
                                        parameters.len()
                                    ),
                                    &callee.borrow().token(),
                                );
                            }
                            for (i, param) in parameters.iter().enumerate() {
                                if i < param_tys.len() {
                                    let expected_ty = param_tys[i].clone();
                                    self.infer_expression_type(
                                        param.expression.clone(),
                                        expected_ty,
                                    );
                                    self.check_parameter_label(param, &decl, i);
                                }
                            }
                        }
                        if is_failable_init {
                            self.current_scope
                                .as_ref()
                                .and_then(|s| s.borrow().get_type("Optional"))
                                .unwrap_or(callee_type.clone())
                        } else {
                            callee_type.clone()
                        }
                    }
                    Type::Class(class_name, ..) => {
                        let (init_params_info, is_failable_init) = {
                            let scope = self.current_scope.as_ref().unwrap().borrow();
                            if let Some(symbol) = scope.get_symbol(class_name) {
                                let constructors = match &*symbol.borrow() {
                                    Symbol::Struct { constructors, .. }
                                    | Symbol::Class { constructors, .. } => constructors.clone(),
                                    _ => return Some(callee_type.clone()),
                                };
                                let mut found_failable = false;
                                let result = constructors.iter().find_map(|constructor| {
                                    if let Ok(Some(decl)) = constructor.borrow().get_decl()
                                        && let Statement::InitDecl {
                                            ty: Some(init_ty),
                                            is_failable,
                                            ..
                                        } = &*decl.borrow()
                                        && let Type::Function(param_tys, _, is_vararg, None) =
                                            &*init_ty.borrow()
                                    {
                                        found_failable = *is_failable;
                                        Some((decl.clone(), param_tys.clone(), *is_vararg))
                                    } else {
                                        None
                                    }
                                });
                                (result, found_failable)
                            } else {
                                (None, false)
                            }
                        };
                        if let Some((decl, param_tys, is_vararg)) = init_params_info {
                            let callee_token = callee.borrow().token();
                            let decl_params = match &*decl.borrow() {
                                Statement::InitDecl { parameters, .. } => parameters.clone(),
                                _ => vec![],
                            };
                            if !decl_params.is_empty() {
                                self.reorder_call_parameters(
                                    parameters,
                                    &decl_params,
                                    &callee_token,
                                );
                            }
                            if !is_vararg && parameters.len() != param_tys.len() {
                                self.emit_error(
                                    TrussDiagnosticCode::ArgumentCountMismatch,
                                    format!(
                                        "Expected {} arguments but found {}",
                                        param_tys.len(),
                                        parameters.len()
                                    ),
                                    &callee.borrow().token(),
                                );
                            } else if is_vararg && parameters.len() < param_tys.len() {
                                self.emit_error(
                                    TrussDiagnosticCode::ArgumentCountMismatch,
                                    format!(
                                        "Expected at least {} arguments but found {}",
                                        param_tys.len(),
                                        parameters.len()
                                    ),
                                    &callee.borrow().token(),
                                );
                            }
                            for (i, param) in parameters.iter().enumerate() {
                                if i < param_tys.len() {
                                    let expected_ty = param_tys[i].clone();
                                    self.infer_expression_type(
                                        param.expression.clone(),
                                        expected_ty,
                                    );
                                    self.check_parameter_label(param, &decl, i);
                                }
                            }
                        }
                        if is_failable_init {
                            self.current_scope
                                .as_ref()
                                .and_then(|s| s.borrow().get_type("Optional"))
                                .unwrap_or(callee_type.clone())
                        } else {
                            callee_type.clone()
                        }
                    }
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::CallingNonFunction,
                            format!("Cannot call non-function type {}", callee_type.borrow()),
                            &callee.borrow().token(),
                        );
                        return None;
                    }
                }
            }
            Expression::Assignment { left, right, .. } => {
                let left_ty = self.infer_type(left.clone())?;

                self.check_writable(left.clone());

                let right_ty = self
                    .infer_expression_type(right.clone(), left_ty.clone())
                    .or_else(|| self.infer_type(right.clone()))?;
                if left_ty.borrow().clone() != right_ty.borrow().clone() {
                    let expected_msg = format!("expected {}", left_ty.borrow());
                    let found_msg = format!("found {}", right_ty.borrow());
                    self.emit_error_with_labels(
                        TrussDiagnosticCode::TypeMismatch,
                        format!(
                            "Type mismatch in assignment: {} vs {}",
                            left_ty.borrow(),
                            right_ty.borrow()
                        ),
                        primary_label_from_token(&left.borrow().token(), &expected_msg),
                        secondary_label_from_token(&right.borrow().token(), &found_msg),
                    );
                }
                left_ty
            }
            Expression::If { .. } => {
                unreachable!()
            }
            Expression::Case {
                enum_type: enum_type_opt,
                case_name,
                expression,
                ty,
                ..
            } => {
                let expr_ty = self.infer_type(expression.clone());

                if let Some(current_scope) = &self.current_scope {
                    if let Some(enum_type) = enum_type_opt.as_ref() {
                        let scope = current_scope.borrow();
                        if let Some(symbol) = scope.get_symbol(&enum_type.value) {
                            if let Symbol::Enum { cases, .. } = &*symbol.borrow() {
                                let found = cases.iter().any(|c| {
                                    c.borrow().name().as_ref().ok() == Some(&case_name.value)
                                });
                                if !found {
                                    self.emit_error(
                                        TrussDiagnosticCode::FieldNotFound,
                                        format!(
                                            "Enum '{}' has no case '{}'",
                                            enum_type.value, case_name.value
                                        ),
                                        case_name.as_ref(),
                                    );
                                }
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::TypeError,
                                    format!("'{}' is not an enum type", enum_type.value),
                                    enum_type.as_ref(),
                                );
                            }
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UnknownType,
                                format!("Unknown type '{}'", enum_type.value),
                                enum_type.as_ref(),
                            );
                        }
                    } else if let Some(expr_ty) = expr_ty.as_ref() {
                        if let Type::Enum(enum_name, ..) = &*expr_ty.borrow() {
                            let scope = current_scope.borrow();
                            if let Some(symbol) = scope.get_symbol(enum_name) {
                                if let Symbol::Enum { cases, .. } = &*symbol.borrow() {
                                    let found = cases.iter().any(|c| {
                                        c.borrow().name().as_ref().ok() == Some(&case_name.value)
                                    });
                                    if !found {
                                        self.emit_error(
                                            TrussDiagnosticCode::FieldNotFound,
                                            format!(
                                                "Enum '{}' has no case '{}'",
                                                enum_name, case_name.value
                                            ),
                                            case_name.as_ref(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                let case_ty = Rc::new(RefCell::new(Type::Bool));
                *ty = Some(case_ty.clone());
                case_ty
            }
            Expression::VoidLiteral { .. } => Rc::new(RefCell::new(Type::Void)),
            Expression::NullLiteral { ty, .. } => {
                if let Some(t) = ty.as_ref() {
                    t.clone()
                } else {
                    Rc::new(RefCell::new(Type::Void))
                }
            }
            Expression::NullptrLiteral { ty, .. } => {
                if let Some(existing_ty) = ty {
                    existing_ty.clone()
                } else {
                    let ptr_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    *ty = Some(ptr_ty.clone());
                    ptr_ty
                }
            }
            Expression::CharLiteral { .. } => Rc::new(RefCell::new(Type::Char)),
            Expression::PointerType { base, ty, non_null } => {
                if let Some(existing_ty) = ty.as_ref() {
                    return Some(existing_ty.clone());
                }
                let base_ty = self.infer_type(*base.clone())?;
                let pointer_ty = if *non_null {
                    Rc::new(RefCell::new(Type::NonNullPointer(base_ty)))
                } else {
                    Rc::new(RefCell::new(Type::Pointer(base_ty)))
                };
                *ty = Some(pointer_ty.clone());
                pointer_ty
            }
            Expression::AnyType { inner, ty } => {
                let inner_ty = self.infer_type(inner.clone())?;
                match &*inner_ty.borrow() {
                    Type::Protocol(..) => {}
                    Type::Compound(types) => {
                        for t in types {
                            if !matches!(&*t.borrow(), Type::Protocol(..)) {
                                let token = inner.borrow().token();
                                self.emit_error(
                                    TrussDiagnosticCode::TypeError,
                                    format!("'any' must be used with a protocol type, but '{}' is not a protocol", inner_ty.borrow()),
                                    &token,
                                );
                                return None;
                            }
                        }
                    }
                    _ => {
                        let token = inner.borrow().token();
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            format!("'any' must be used with a protocol type, but '{}' is not a protocol", inner_ty.borrow()),
                            &token,
                        );
                        return None;
                    }
                }
                *ty = Some(inner_ty.clone());
                inner_ty
            }
            Expression::SomeType { inner, ty } => {
                let inner_ty = self.infer_type(inner.clone())?;
                match &*inner_ty.borrow() {
                    Type::Protocol(..) => {}
                    Type::Compound(types) => {
                        for t in types {
                            if !matches!(&*t.borrow(), Type::Protocol(..)) {
                                let token = inner.borrow().token();
                                self.emit_error(
                                    TrussDiagnosticCode::TypeError,
                                    format!("'some' must be used with a protocol type, but '{}' is not a protocol", inner_ty.borrow()),
                                    &token,
                                );
                                return None;
                            }
                        }
                    }
                    _ => {
                        let token = inner.borrow().token();
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            format!("'some' must be used with a protocol type, but '{}' is not a protocol", inner_ty.borrow()),
                            &token,
                        );
                        return None;
                    }
                }
                *ty = Some(inner_ty.clone());
                inner_ty
            }
            Expression::CompoundType { types, ty } => {
                let mut resolved = Vec::new();
                for t in types {
                    if let Some(t_ty) = self.infer_type(t.clone()) {
                        resolved.push(t_ty);
                    } else {
                        return None;
                    }
                }
                let compound = Rc::new(RefCell::new(Type::Compound(resolved)));
                *ty = Some(compound.clone());
                compound
            }
            Expression::Match {
                value, cases, ty, ..
            } => {
                let subject_ty = self.infer_type(value.clone());
                let mut match_ty = Rc::new(RefCell::new(Type::Void));

                for case in cases {
                    let case_scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
                    self.enter_scope(case_scope.clone());

                    for pattern in &case.patterns {
                        if let Pattern::EnumCase {
                            case_name,
                            bindings,
                            ..
                        } = pattern.as_ref()
                        {
                            if !bindings.is_empty() {
                                if let Some(ref subject_ty) = subject_ty {
                                    if let Type::Enum(enum_name, ..) = &*subject_ty.borrow() {
                                        let param_types = self.get_enum_case_parameter_types(
                                            enum_name,
                                            &case_name.value,
                                        );
                                        if let Some(ref param_types) = param_types {
                                            Self::set_binding_types(
                                                bindings,
                                                param_types,
                                                &case_scope,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        if let Pattern::ValueBinding(inner) = pattern.as_ref() {
                            if let Pattern::Identifier(name) = inner.as_ref() {
                                if let Some(ref subject_ty) = subject_ty {
                                    case_scope
                                        .borrow_mut()
                                        .set_type(name.value.clone(), subject_ty.clone());
                                }
                            }
                        }
                    }

                    if let Some(guard) = &case.guard {
                        self.infer_type(guard.clone());
                    }

                    let body_ty = self
                        .get_block_type(&case.body)
                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));

                    if *match_ty.borrow() == Type::Void {
                        match_ty = body_ty;
                    } else if match_ty.borrow().clone() != body_ty.borrow().clone() {
                        self.emit_error(
                            TrussDiagnosticCode::BranchTypeMismatch,
                            format!(
                                "Match branches have different types: {} vs {}",
                                match_ty.borrow(),
                                body_ty.borrow()
                            ),
                            &case.token,
                        );
                    }

                    self.leave_scope();
                }

                *ty = Some(match_ty.clone());
                match_ty
            }
            Expression::Cast {
                expression,
                target_type,
                ty,
                kind,
                ..
            } => {
                let source_ty = self.infer_type(expression.clone())?;
                let target_ty = self.infer_type(target_type.clone())?;
                let token = expression.borrow().token();

                match kind {
                    CastKind::ForceBitcast => {
                        if !Self::check_cast_bitcast(&source_ty.borrow(), &target_ty.borrow()) {
                            self.emit_error(
                                TrussDiagnosticCode::TypeMismatch,
                                format!(
                                    "Cannot bitcast between types of different sizes: '{}' ({} bits) to '{}' ({} bits)",
                                    source_ty.borrow(),
                                    Self::get_type_size_bits(&source_ty.borrow()).unwrap_or(0),
                                    target_ty.borrow(),
                                    Self::get_type_size_bits(&target_ty.borrow()).unwrap_or(0)
                                ),
                                &token,
                            );
                        }
                    }
                    _ => {
                        if !Self::check_cast(&source_ty.borrow(), &target_ty.borrow()) {
                            self.emit_error(
                                TrussDiagnosticCode::TypeMismatch,
                                format!(
                                    "Cannot cast from '{}' to '{}'",
                                    source_ty.borrow(),
                                    target_ty.borrow()
                                ),
                                &token,
                            );
                        }
                    }
                }
                *ty = Some(target_ty.clone());
                target_ty
            }
            Expression::MemberAccess { object, member, ty } => {
                if let Some(result) = self.try_module_member_access(object.clone(), member, ty) {
                    return Some(result);
                }
                let object_ty = self.infer_type(object.clone())?;
                match &*object_ty.borrow() {
                    Type::Struct(struct_name, ..) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        let symbol_opt = scope.get_symbol(struct_name);
                        drop(scope);
                        let Some(symbol) = symbol_opt else {
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!("Struct symbol '{}' not found", struct_name),
                                member,
                            );
                            return None;
                        };

                        if !self.is_member_accessible(symbol.clone(), member) {
                            self.emit_error(
                                TrussDiagnosticCode::InaccessibleMember,
                                format!(
                                    "'{}' is inaccessible due to '{}' level",
                                    member.value,
                                    symbol
                                        .borrow()
                                        .get_decl()
                                        .unwrap()
                                        .unwrap()
                                        .borrow()
                                        .access_modifier()
                                        .map_or(String::from("internal"), |m| m
                                            .map(|m| m.token.value.clone())
                                            .unwrap_or(String::from("internal")))
                                ),
                                member,
                            );
                            return None;
                        }

                        let binding = symbol.borrow();
                        let Symbol::Struct {
                            properties,
                            methods,
                            ..
                        } = &*binding
                        else {
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!("Struct '{}' has unexpected symbol type", struct_name),
                                member,
                            );
                            return None;
                        };
                        for field in properties {
                            if field.borrow().name().as_ref().ok() == Some(&member.value)
                                && let Some(decl) = field.borrow().get_decl().ok().flatten()
                                && let Statement::VariableDecl { ty: field_ty, .. } =
                                    &*decl.borrow()
                                && let Some(t) = field_ty
                            {
                                if !self.is_member_symbol_accessible(field.clone(), member) {
                                    self.emit_error(
                                        TrussDiagnosticCode::InaccessibleMember,
                                        format!(
                                            "'{}' is inaccessible due to '{}' level",
                                            member.value,
                                            field
                                                .borrow()
                                                .get_decl()
                                                .ok()
                                                .flatten()
                                                .map(|d| {
                                                    d.borrow().access_modifier().map_or(
                                                        String::from("internal"),
                                                        |m| {
                                                            m.map(|m| m.token.value.clone())
                                                                .unwrap_or(String::from("internal"))
                                                        },
                                                    )
                                                })
                                                .unwrap_or(String::from("internal"))
                                        ),
                                        member,
                                    );
                                    return None;
                                }
                                *ty = Some(t.clone());
                                return Some(t.clone());
                            }
                        }
                        for method in methods {
                            if method.borrow().name().as_ref().ok() == Some(&member.value)
                                && let Some(decl) = method.borrow().get_decl().ok().flatten()
                            {
                                if !self.is_member_symbol_accessible(method.clone(), member) {
                                    self.emit_error(
                                        TrussDiagnosticCode::InaccessibleMember,
                                        format!(
                                            "'{}' is inaccessible due to '{}' level",
                                            member.value,
                                            method
                                                .borrow()
                                                .get_decl()
                                                .ok()
                                                .flatten()
                                                .map(|d| {
                                                    d.borrow().access_modifier().map_or(
                                                        String::from("internal"),
                                                        |m| {
                                                            m.map(|m| m.token.value.clone())
                                                                .unwrap_or(String::from("internal"))
                                                        },
                                                    )
                                                })
                                                .unwrap_or(String::from("internal"))
                                        ),
                                        member,
                                    );
                                    return None;
                                }
                                let method_ty = {
                                    let decl_ref = decl.borrow();
                                    if let Statement::FunctionDecl { ty, .. } = &*decl_ref {
                                        ty.clone()
                                    } else if let Statement::InitDecl { ty, .. } = &*decl_ref {
                                        ty.clone()
                                    } else if let Statement::DeinitDecl { ty, .. } = &*decl_ref {
                                        ty.clone()
                                    } else {
                                        continue;
                                    }
                                };
                                if let Some(t) = method_ty {
                                    *ty = Some(t.clone());
                                    return Some(t.clone());
                                }
                            }
                        }
                        let token = &*member;
                        self.emit_error(
                            TrussDiagnosticCode::FieldNotFound,
                            format!(
                                "Field '{}' not found on struct '{}'",
                                member.value, struct_name
                            ),
                            token,
                        );
                        return None;
                    }
                    Type::Class(class_name, ..) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        let symbol_opt = scope.get_symbol(class_name);
                        drop(scope);
                        let Some(symbol) = symbol_opt else {
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!("Class '{}' not found", class_name),
                                member,
                            );
                            return None;
                        };

                        if !self.is_member_accessible(symbol.clone(), member) {
                            self.emit_error(
                                TrussDiagnosticCode::InaccessibleMember,
                                format!(
                                    "'{}' is inaccessible due to '{}' level",
                                    member.value,
                                    symbol
                                        .borrow()
                                        .get_decl()
                                        .unwrap()
                                        .unwrap()
                                        .borrow()
                                        .access_modifier()
                                        .map_or(String::from("internal"), |m| m
                                            .map(|m| m.token.value.clone())
                                            .unwrap_or(String::from("internal")))
                                ),
                                member,
                            );
                            return None;
                        }

                        let binding = symbol.borrow();
                        let (properties, methods) = match &*binding {
                            Symbol::Struct {
                                properties,
                                methods,
                                ..
                            }
                            | Symbol::Class {
                                properties,
                                methods,
                                ..
                            } => (properties, methods),
                            _ => {
                                self.emit_error(
                                    TrussDiagnosticCode::FieldNotFound,
                                    format!("Class symbol '{}' has unexpected type", class_name),
                                    member,
                                );
                                return None;
                            }
                        };
                        for field in properties {
                            if field.borrow().name().as_ref().ok() == Some(&member.value)
                                && let Some(decl) = field.borrow().get_decl().ok().flatten()
                                && let Statement::VariableDecl { ty: field_ty, .. } =
                                    &*decl.borrow()
                                && let Some(t) = field_ty
                            {
                                if !self.is_member_symbol_accessible(field.clone(), member) {
                                    self.emit_error(
                                        TrussDiagnosticCode::InaccessibleMember,
                                        format!(
                                            "'{}' is inaccessible due to '{}' level",
                                            member.value,
                                            field
                                                .borrow()
                                                .get_decl()
                                                .ok()
                                                .flatten()
                                                .map(|d| {
                                                    d.borrow().access_modifier().map_or(
                                                        String::from("internal"),
                                                        |m| {
                                                            m.map(|m| m.token.value.clone())
                                                                .unwrap_or(String::from("internal"))
                                                        },
                                                    )
                                                })
                                                .unwrap_or(String::from("internal"))
                                        ),
                                        member,
                                    );
                                    return None;
                                }
                                *ty = Some(t.clone());
                                return Some(t.clone());
                            }
                        }
                        for method in methods {
                            if method.borrow().name().as_ref().ok() == Some(&member.value)
                                && let Some(decl) = method.borrow().get_decl().ok().flatten()
                            {
                                if !self.is_member_symbol_accessible(method.clone(), member) {
                                    self.emit_error(
                                        TrussDiagnosticCode::InaccessibleMember,
                                        format!(
                                            "'{}' is inaccessible due to '{}' level",
                                            member.value,
                                            method
                                                .borrow()
                                                .get_decl()
                                                .ok()
                                                .flatten()
                                                .map(|d| {
                                                    d.borrow().access_modifier().map_or(
                                                        String::from("internal"),
                                                        |m| {
                                                            m.map(|m| m.token.value.clone())
                                                                .unwrap_or(String::from("internal"))
                                                        },
                                                    )
                                                })
                                                .unwrap_or(String::from("internal"))
                                        ),
                                        member,
                                    );
                                    return None;
                                }
                                let method_ty = {
                                    let decl_ref = decl.borrow();
                                    if let Statement::FunctionDecl { ty, .. } = &*decl_ref {
                                        ty.clone()
                                    } else if let Statement::InitDecl { ty, .. } = &*decl_ref {
                                        ty.clone()
                                    } else if let Statement::DeinitDecl { ty, .. } = &*decl_ref {
                                        ty.clone()
                                    } else {
                                        continue;
                                    }
                                };
                                if let Some(t) = method_ty {
                                    *ty = Some(t.clone());
                                    return Some(t.clone());
                                }
                            }
                        }

                        let decl = symbol.borrow().get_decl().ok().flatten();
                        let super_info = decl.as_ref().and_then(|decl| {
                            if let Statement::ClassDecl {
                                superclass: Some(super_expr),
                                ..
                            } = &*decl.borrow()
                            {
                                if let Expression::Type {
                                    name: super_name, ..
                                } = &*super_expr.borrow()
                                {
                                    return Some((
                                        super_name.value.clone(),
                                        super_name.position.clone(),
                                        super_name.file.clone(),
                                    ));
                                }
                            }
                            None
                        });

                        if let Some((super_name, pos, file)) = super_info {
                            let super_object = Rc::new(RefCell::new(Expression::Variable {
                                name: Box::new(Token::new(
                                    super_name,
                                    TokenType::Identifier,
                                    pos,
                                    file,
                                )),
                                ty: None,
                                symbol: None,
                            }));
                            let member_expr = Rc::new(RefCell::new(Expression::MemberAccess {
                                object: super_object,
                                member: Box::new(member.as_ref().clone()),
                                ty: None,
                            }));
                            return self.infer_type(member_expr);
                        }

                        let token = &*member;
                        self.emit_error(
                            TrussDiagnosticCode::FieldNotFound,
                            format!(
                                "Field '{}' not found on class '{}'",
                                member.value, class_name
                            ),
                            token,
                        );
                        return None;
                    }
                    Type::Enum(enum_name, ..) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        if let Some(symbol) = scope.get_symbol(enum_name)
                            && let Symbol::Enum { cases, .. } = &*symbol.borrow()
                        {
                            for case in cases {
                                if case.borrow().name().as_ref().ok() == Some(&member.value) {
                                    if let Symbol::EnumCase {
                                        parameter_types, ..
                                    } = &*case.borrow()
                                    {
                                        if parameter_types.is_empty() {
                                            *ty = Some(object_ty.clone());
                                            return Some(object_ty.clone());
                                        } else {
                                            let case_fn_type =
                                                Rc::new(RefCell::new(Type::Function(
                                                    parameter_types.clone(),
                                                    object_ty.clone(),
                                                    false,
                                                    None,
                                                )));
                                            *ty = Some(case_fn_type.clone());
                                            return Some(case_fn_type);
                                        }
                                    }
                                }
                            }
                            let token = &*member;
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!(
                                    "Case '{}' not found on enum '{}'",
                                    member.value, enum_name
                                ),
                                token,
                            );
                            return None;
                        } else {
                            let token = &*member;
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!("Enum symbol '{}' not found", enum_name),
                                token,
                            );
                            return None;
                        }
                    }
                    Type::Tuple(elements) => {
                        let member_name = &member.value;
                        if let Some((_, element_ty)) =
                            elements.iter().enumerate().find_map(|(i, (n, t))| {
                                n.as_ref().and_then(|name| {
                                    if name == member_name {
                                        Some((i, t.clone()))
                                    } else {
                                        None
                                    }
                                })
                            })
                        {
                            *ty = Some(element_ty.clone());
                            return Some(element_ty.clone());
                        }
                        if let Ok(idx) = member_name.parse::<usize>() {
                            if idx < elements.len() {
                                let element_ty = elements[idx].1.clone();
                                *ty = Some(element_ty.clone());
                                return Some(element_ty);
                            }
                        }
                        let token = &*member;
                        self.emit_error(
                            TrussDiagnosticCode::FieldNotFound,
                            format!(
                                "Field '{}' not found on tuple type '{}'",
                                member.value,
                                object_ty.borrow()
                            ),
                            token,
                        );
                        return None;
                    }
                    Type::Protocol(protocol_name, ..) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        let protocol_name = protocol_name.clone();
                        if let Some(symbol) = scope.get_symbol(&protocol_name)
                            && let Symbol::Protocol { methods, .. } = &*symbol.borrow()
                        {
                            for method in methods {
                                if method.borrow().name().as_ref().ok() == Some(&member.value)
                                    && let Some(decl) = method.borrow().get_decl().ok().flatten()
                                {
                                    let method_ty = {
                                        let decl_ref = decl.borrow();
                                        if let Statement::FunctionDecl { ty, .. } = &*decl_ref {
                                            ty.clone()
                                        } else {
                                            continue;
                                        }
                                    };
                                    if let Some(t) = method_ty {
                                        *ty = Some(t.clone());
                                        return Some(t.clone());
                                    }
                                }
                            }
                            let token = &*member;
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!(
                                    "Member '{}' not found on protocol '{}'",
                                    member.value, protocol_name
                                ),
                                token,
                            );
                            return None;
                        } else {
                            let token = &*member;
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!("Protocol symbol '{}' not found", protocol_name),
                                token,
                            );
                            return None;
                        }
                    }
                    Type::Compound(types) => {
                        let protocol_names: Vec<String> = types
                            .iter()
                            .filter_map(|t| {
                                if let Type::Protocol(name, ..) = &*t.borrow() {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        for protocol_name in &protocol_names {
                            let scope = self.current_scope.as_ref().unwrap().borrow();
                            if let Some(symbol) = scope.get_symbol(protocol_name)
                                && let Symbol::Protocol {
                                    methods,
                                    properties,
                                    ..
                                } = &*symbol.borrow()
                            {
                                for method in methods {
                                    if method.borrow().name().as_ref().ok() == Some(&member.value)
                                        && let Some(decl) =
                                            method.borrow().get_decl().ok().flatten()
                                    {
                                        let method_ty = {
                                            let decl_ref = decl.borrow();
                                            if let Statement::FunctionDecl { ty, .. } = &*decl_ref {
                                                ty.clone()
                                            } else {
                                                continue;
                                            }
                                        };
                                        if let Some(t) = method_ty {
                                            *ty = Some(t.clone());
                                            return Some(t.clone());
                                        }
                                    }
                                }
                                for prop in properties {
                                    if prop.borrow().name().as_ref().ok() == Some(&member.value)
                                        && let Some(decl) = prop.borrow().get_decl().ok().flatten()
                                        && let Statement::VariableDecl { ty: prop_ty, .. } =
                                            &*decl.borrow()
                                        && let Some(t) = prop_ty
                                    {
                                        *ty = Some(t.clone());
                                        return Some(t.clone());
                                    }
                                }
                            }
                            drop(scope);
                        }
                        let token = &*member;
                        self.emit_error(
                            TrussDiagnosticCode::FieldNotFound,
                            format!(
                                "Member '{}' not found on compound protocol type '{}'",
                                member.value,
                                object_ty.borrow()
                            ),
                            token,
                        );
                        return None;
                    }
                    Type::Inline(inner, _) => {
                        let inner_borrow = inner.borrow();
                        match &*inner_borrow {
                            Type::Class(class_name, ..) => {
                                let scope = self.current_scope.as_ref().unwrap().borrow();
                                if let Some(symbol) = scope.get_symbol(class_name) {
                                    let binding = symbol.borrow();
                                    let (properties, methods) = match &*binding {
                                        Symbol::Struct {
                                            properties,
                                            methods,
                                            ..
                                        }
                                        | Symbol::Class {
                                            properties,
                                            methods,
                                            ..
                                        } => (properties, methods),
                                        _ => {
                                            self.emit_error(
                                                TrussDiagnosticCode::FieldNotFound,
                                                format!(
                                                    "Class symbol '{}' has unexpected type",
                                                    class_name
                                                ),
                                                member,
                                            );
                                            return None;
                                        }
                                    };
                                    for field in properties {
                                        if field.borrow().name().as_ref().ok()
                                            == Some(&member.value)
                                            && let Some(decl) =
                                                field.borrow().get_decl().ok().flatten()
                                            && let Statement::VariableDecl { ty: field_ty, .. } =
                                                &*decl.borrow()
                                            && let Some(t) = field_ty
                                        {
                                            *ty = Some(t.clone());
                                            return Some(t.clone());
                                        }
                                    }
                                    for method in methods {
                                        if method.borrow().name().as_ref().ok()
                                            == Some(&member.value)
                                            && let Some(decl) =
                                                method.borrow().get_decl().ok().flatten()
                                        {
                                            let method_ty = {
                                                let decl_ref = decl.borrow();
                                                if let Statement::FunctionDecl { ty, .. } =
                                                    &*decl_ref
                                                {
                                                    ty.clone()
                                                } else if let Statement::InitDecl { ty, .. } =
                                                    &*decl_ref
                                                {
                                                    ty.clone()
                                                } else if let Statement::DeinitDecl { ty, .. } =
                                                    &*decl_ref
                                                {
                                                    ty.clone()
                                                } else {
                                                    continue;
                                                }
                                            };
                                            if let Some(t) = method_ty {
                                                *ty = Some(t.clone());
                                                return Some(t.clone());
                                            }
                                        }
                                    }
                                    self.emit_error(
                                        TrussDiagnosticCode::FieldNotFound,
                                        format!(
                                            "Member '{}' not found on inline class '{}'",
                                            member.value, class_name
                                        ),
                                        member,
                                    );
                                    return None;
                                } else {
                                    self.emit_error(
                                        TrussDiagnosticCode::FieldNotFound,
                                        format!("Class symbol '{}' not found", class_name),
                                        member,
                                    );
                                    return None;
                                }
                            }
                            _ => {
                                let token = &*member;
                                self.emit_error(
                                    TrussDiagnosticCode::FieldNotFound,
                                    format!(
                                        "Cannot access member '{}' of inline non-class type '{}'",
                                        member.value,
                                        object_ty.borrow()
                                    ),
                                    token,
                                );
                                return None;
                            }
                        }
                    }
                    _ => {
                        let token = &*member;
                        self.emit_error(
                            TrussDiagnosticCode::FieldNotFound,
                            format!(
                                "Cannot access member '{}' of non-struct/enum type '{}'",
                                member.value,
                                object_ty.borrow()
                            ),
                            token,
                        );
                        return None;
                    }
                }
            }
            Expression::TupleLiteral { elements, ty, .. } => {
                let mut element_types = Vec::new();
                for (name, elem) in elements {
                    if let Some(t) = self.infer_type(elem.clone()) {
                        element_types.push((name.clone(), t));
                    }
                }
                let tuple_ty = Rc::new(RefCell::new(Type::Tuple(element_types)));
                *ty = Some(tuple_ty.clone());
                tuple_ty
            }
            Expression::TupleType { elements, .. } => {
                let mut element_types = Vec::new();
                for (name, elem) in elements {
                    if let Some(t) = self.infer_type(elem.clone()) {
                        element_types.push((name.clone(), t));
                    }
                }
                Rc::new(RefCell::new(Type::Tuple(element_types)))
            }
            Expression::TupleIndexAccess {
                object,
                index,
                index_value,
                ty,
            } => {
                let object_ty = self.infer_type(object.clone())?;
                match &*object_ty.borrow() {
                    Type::Tuple(elements) => {
                        let idx = *index_value as usize;
                        if idx < elements.len() {
                            let element_ty = elements[idx].1.clone();
                            *ty = Some(element_ty.clone());
                            element_ty
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::TypeError,
                                format!(
                                    "Index {} out of bounds for tuple of length {}",
                                    idx,
                                    elements.len()
                                ),
                                index.as_ref(),
                            );
                            return None;
                        }
                    }
                    other => {
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            format!(
                                "Cannot index into non-tuple type '{}' with .{}",
                                other, index_value
                            ),
                            index.as_ref(),
                        );
                        return None;
                    }
                }
            }
            Expression::SelfKeyword { token, ty, .. } => {
                let t = self
                    .current_scope
                    .as_ref()
                    .ok_or_else(|| {
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            "No type environment available",
                            token.as_ref(),
                        );
                    })
                    .ok()?
                    .borrow()
                    .get_type("self");
                let t = t?;
                *ty = Some(t.clone());
                t
            }
            Expression::SuperKeyword { token, ty, .. } => {
                let self_ty = self
                    .current_scope
                    .as_ref()
                    .ok_or_else(|| {
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            "No type environment available",
                            token.as_ref(),
                        );
                    })
                    .ok()?
                    .borrow()
                    .get_type("self");
                let self_ty = self_ty?;
                let class_name = self_ty.borrow().to_string();
                let class_name = class_name
                    .strip_prefix("Class(")
                    .and_then(|s| s.strip_suffix(')'))
                    .or(Some(class_name.as_str()))
                    .map(|s| s.to_string());

                let super_ty = class_name
                    .as_ref()
                    .and_then(|name| self.superclass_map.get(name).cloned());

                if let Some(super_ty) = super_ty {
                    *ty = Some(super_ty.clone());
                    super_ty
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::TypeError,
                        "'super' is only available in classes with a superclass",
                        token.as_ref(),
                    );
                    return None;
                }
            }
            Expression::SelfType { ty, .. } => {
                let t = self.current_scope.as_ref()?.borrow().get_type("Self");
                if let Some(t) = t {
                    *ty = Some(t.clone());
                    t
                } else {
                    return None;
                }
            }
            Expression::AssociatedTypeAccess { object, member, ty } => {
                let object_ty = self.infer_type(object.clone());
                let object_ty = match object_ty {
                    Some(t) => t,
                    None => return None,
                };
                let result = match &*object_ty.borrow() {
                    Type::Protocol(protocol_name, weak_sym, ..) => {
                        if let Some(sym) = weak_sym.0.upgrade() {
                            if let Ok(Some(decl)) = sym.borrow().get_decl() {
                                if let Statement::ProtocolDecl { scope, .. } = &*decl.borrow() {
                                    if let Some(protocol_scope) = scope {
                                        let scope_ref = protocol_scope.borrow();
                                        if let Some(found) = scope_ref.get_type(&member.value) {
                                            let found_ty = found.borrow().clone();
                                            match &found_ty {
                                                Type::GenericParam(_) => {
                                                    Rc::new(RefCell::new(Type::AssociatedType(
                                                        object_ty.clone(),
                                                        member.value.clone(),
                                                    )))
                                                }
                                                _ => found.clone(),
                                            }
                                        } else {
                                            self.emit_error(
                                                TrussDiagnosticCode::UnknownType,
                                                format!("Associated type '{}' not found in protocol '{}'", member.value, protocol_name),
                                                member,
                                            );
                                            return None;
                                        }
                                    } else {
                                        self.emit_error(
                                            TrussDiagnosticCode::UnknownType,
                                            format!("Protocol '{}' has no scope", protocol_name),
                                            member,
                                        );
                                        return None;
                                    }
                                } else {
                                    return None;
                                }
                            } else {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                    Type::Compound(types) => {
                        let mut result = None;
                        for t in types {
                            if let Type::Protocol(_name, weak_sym, ..) = &*t.borrow() {
                                if let Some(sym) = weak_sym.0.upgrade() {
                                    if let Ok(Some(decl)) = sym.borrow().get_decl() {
                                        if let Statement::ProtocolDecl { scope, .. } =
                                            &*decl.borrow()
                                        {
                                            if let Some(protocol_scope) = scope {
                                                let scope_ref = protocol_scope.borrow();
                                                if let Some(found) =
                                                    scope_ref.get_type(&member.value)
                                                {
                                                    let found_ty = found.borrow().clone();
                                                    result = Some(match &found_ty {
                                                        Type::GenericParam(_) => Rc::new(
                                                            RefCell::new(Type::AssociatedType(
                                                                object_ty.clone(),
                                                                member.value.clone(),
                                                            )),
                                                        ),
                                                        _ => found.clone(),
                                                    });
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        match result {
                            Some(t) => t,
                            None => {
                                self.emit_error(
                                    TrussDiagnosticCode::UnknownType,
                                    format!(
                                        "Associated type '{}' not found in compound protocol",
                                        member.value
                                    ),
                                    member,
                                );
                                return None;
                            }
                        }
                    }
                    Type::GenericParam(_) => Rc::new(RefCell::new(Type::AssociatedType(
                        object_ty.clone(),
                        member.value.clone(),
                    ))),
                    Type::Struct(struct_name, ..) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        if let Some(symbol) = scope.get_symbol(struct_name) {
                            if let Ok(Some(decl)) = symbol.borrow().get_decl() {
                                if !self.is_member_accessible(symbol.clone(), member) {
                                    self.emit_error(
                                        TrussDiagnosticCode::InaccessibleMember,
                                        format!(
                                            "'{}' is inaccessible due to access level",
                                            member.value
                                        ),
                                        member,
                                    );
                                    return None;
                                }
                                if let Statement::StructDecl {
                                    scope: struct_scope,
                                    ..
                                } = &*decl.borrow()
                                {
                                    if let Some(s) = struct_scope {
                                        if let Some(found) = s.borrow().get_type(&member.value) {
                                            found
                                        } else {
                                            self.emit_error(
                                                TrussDiagnosticCode::UnknownType,
                                                format!(
                                                    "Type '{}' not found in struct '{}'",
                                                    member.value, struct_name
                                                ),
                                                member,
                                            );
                                            return None;
                                        }
                                    } else {
                                        self.emit_error(
                                            TrussDiagnosticCode::UnknownType,
                                            format!("Struct '{}' has no scope", struct_name),
                                            member,
                                        );
                                        return None;
                                    }
                                } else {
                                    return None;
                                }
                            } else {
                                return None;
                            }
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UnknownType,
                                format!("Struct '{}' not found in current scope", struct_name),
                                member,
                            );
                            return None;
                        }
                    }
                    Type::Class(class_name, ..) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        if let Some(symbol) = scope.get_symbol(class_name) {
                            if let Ok(Some(decl)) = symbol.borrow().get_decl() {
                                if !self.is_member_accessible(symbol.clone(), member) {
                                    self.emit_error(
                                        TrussDiagnosticCode::InaccessibleMember,
                                        format!(
                                            "'{}' is inaccessible due to access level",
                                            member.value
                                        ),
                                        member,
                                    );
                                    return None;
                                }
                                if let Statement::ClassDecl {
                                    scope: class_scope, ..
                                } = &*decl.borrow()
                                {
                                    if let Some(s) = class_scope {
                                        if let Some(found) = s.borrow().get_type(&member.value) {
                                            found
                                        } else {
                                            self.emit_error(
                                                TrussDiagnosticCode::UnknownType,
                                                format!(
                                                    "Type '{}' not found in class '{}'",
                                                    member.value, class_name
                                                ),
                                                member,
                                            );
                                            return None;
                                        }
                                    } else {
                                        self.emit_error(
                                            TrussDiagnosticCode::UnknownType,
                                            format!("Class '{}' has no scope", class_name),
                                            member,
                                        );
                                        return None;
                                    }
                                } else {
                                    return None;
                                }
                            } else {
                                return None;
                            }
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UnknownType,
                                format!("Class '{}' not found in current scope", class_name),
                                member,
                            );
                            return None;
                        }
                    }
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::UnknownType,
                            format!("Cannot access associated type on non-protocol type"),
                            member,
                        );
                        return None;
                    }
                };
                *ty = Some(result.clone());
                result
            }
            Expression::Closure {
                parameters,
                return_type,
                body,
                scope,
                ty,
                ..
            } => {
                let (mut ret_type, ret_type_from_expected) = return_type
                    .as_ref()
                    .and_then(|rt| self.infer_type(rt.clone()))
                    .map(|t| (t, false))
                    .unwrap_or_else(|| (Rc::new(RefCell::new(Type::Void)), true));
                let mut param_types = Vec::new();
                for param in parameters.iter() {
                    let param_type = param
                        .borrow()
                        .type_annotation
                        .as_ref()
                        .and_then(|ta| self.infer_type(ta.clone()))
                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
                    param_types.push(param_type);
                }

                let max_shorthand = self.find_max_shorthand(body);
                let shorthand_start = parameters.len();
                if let Some(max_idx) = max_shorthand {
                    let required_params = max_idx as usize + 1;
                    if shorthand_start == 0 && self.closure_expected_type.is_none() {
                        let first_token = body.first().and_then(|s| match &*s.borrow() {
                            Statement::ExpressionStatement { expression } => {
                                Some(expression.borrow().token())
                            }
                            Statement::Return {
                                value: Some(val), ..
                            } => Some(val.borrow().token()),
                            _ => None,
                        });
                        if let Some(tok) = first_token {
                            self.emit_error(
                                TrussDiagnosticCode::UnknownType,
                                "Cannot infer closure parameter types from shorthand arguments without type context; add explicit type annotation",
                                &tok,
                            );
                        }
                    } else if shorthand_start == 0 {
                        if let Some(expected_ty) = &self.closure_expected_type {
                            if let Type::Function(expected_params, expected_ret, _, None) =
                                &*expected_ty.borrow()
                            {
                                for idx in 0..required_params {
                                    let pt = expected_params
                                        .get(idx)
                                        .cloned()
                                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32)));
                                    param_types.push(pt);
                                }
                                if ret_type_from_expected {
                                    let ret =
                                        Rc::new(RefCell::new(Type::clone(&*expected_ret.borrow())));
                                    let _ = std::mem::replace(&mut ret_type, ret);
                                }
                            }
                        }
                    } else if required_params > shorthand_start {
                        for _ in shorthand_start..required_params {
                            param_types.push(Rc::new(RefCell::new(Type::Int32)));
                        }
                    }
                }

                let fn_type = Rc::new(RefCell::new(Type::Function(
                    param_types.clone(),
                    ret_type,
                    false,
                    None,
                )));
                *ty = Some(fn_type.clone());

                if let Some(sc) = scope {
                    self.enter_scope(sc.clone());
                    for (i, param) in parameters.iter().enumerate() {
                        let p = param.borrow();
                        let param_type = if i < param_types.len() {
                            param_types[i].clone()
                        } else {
                            Rc::new(RefCell::new(Type::Void))
                        };
                        self.current_scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(p.name.value.clone(), param_type);
                    }
                    if let Some(max_idx) = max_shorthand {
                        for idx in 0..=max_idx {
                            let name = format!("${}", idx);
                            let param_type = param_types
                                .get(idx as usize)
                                .cloned()
                                .unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32)));
                            self.current_scope
                                .as_ref()
                                .unwrap()
                                .borrow_mut()
                                .set_type(name, param_type);
                        }
                    }
                    for stmt in body.iter() {
                        let s = stmt.borrow();
                        if let Statement::ExpressionStatement { expression } = &*s {
                            self.infer_type(expression.clone());
                        } else if let Statement::Return {
                            value: Some(value), ..
                        } = &*s
                        {
                            self.infer_type(value.clone());
                        } else {
                            drop(s);
                            self.process_decl(stmt.clone());
                        }
                    }
                    self.leave_scope();
                }

                fn_type
            }
            Expression::FunctionType {
                param_types,
                return_type,
                ty,
                ..
            } => {
                let params: Vec<Rc<RefCell<Type>>> = param_types
                    .iter()
                    .filter_map(|pt| self.infer_type(pt.clone()))
                    .collect();
                let ret = self
                    .infer_type(return_type.clone())
                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
                let fn_type = Rc::new(RefCell::new(Type::Function(params, ret, false, None)));
                *ty = Some(fn_type.clone());
                fn_type
            }
            Expression::ShorthandArgument { index, ty } => {
                if ty.is_none() {
                    let name = format!("${}", index);
                    let found = self
                        .current_scope
                        .as_ref()
                        .and_then(|s| s.borrow().get_type(&name));
                    *ty = Some(found.unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32))));
                }
                ty.clone().unwrap()
            }
            Expression::SubscriptAccess {
                object,
                parameters,
                ty,
            } => {
                if let Some(t) = ty {
                    t.clone()
                } else {
                    let object_ty = self.infer_type(object.clone());
                    let result = object_ty
                        .clone()
                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
                    if let Some(ref object_t) = object_ty {
                        let object_t_clone = object_t.borrow().clone();
                        let lookup_result = match &object_t_clone {
                            Type::Struct(struct_name, ..) | Type::Class(struct_name, ..) => {
                                let struct_name = struct_name.clone();
                                let token = object.borrow().token();
                                let scope = self.current_scope.as_ref().unwrap().borrow();
                                scope.get_symbol(&struct_name).and_then(|symbol| {
                                    if !self.is_member_accessible(symbol.clone(), &token) {
                                        self.emit_error(
                                            TrussDiagnosticCode::InaccessibleMember,
                                            format!(
                                                "'{}' is inaccessible due to '{}' level",
                                                struct_name,
                                                symbol
                                                    .borrow()
                                                    .get_decl()
                                                    .unwrap()
                                                    .unwrap()
                                                    .borrow()
                                                    .access_modifier()
                                                    .map_or(String::from("internal"), |m| m
                                                        .map(|m| m.token.value.clone())
                                                        .unwrap_or(String::from("internal")))
                                            ),
                                            &token,
                                        );
                                        return None;
                                    }
                                    let subscripts = match &*symbol.borrow() {
                                        Symbol::Struct { subscripts, .. }
                                        | Symbol::Class { subscripts, .. } => subscripts.clone(),
                                        _ => vec![],
                                    };
                                    drop(scope);
                                    for sub in subscripts {
                                        if let Some(decl) = sub.borrow().get_decl().ok().flatten()
                                            && let Statement::SubscriptDecl { ty: sub_ty, .. } =
                                                &*decl.borrow()
                                            && let Some(t) = sub_ty
                                        {
                                            if !self.is_member_symbol_accessible(
                                                sub.clone(),
                                                &token,
                                            ) {
                                                self.emit_error(
                                                    TrussDiagnosticCode::InaccessibleMember,
                                                    format!(
                                                        "subscript is inaccessible due to '{}' level",
                                                        sub
                                                            .borrow()
                                                            .get_decl()
                                                            .ok()
                                                            .flatten()
                                                            .map(|d| {
                                                                d.borrow()
                                                                    .access_modifier()
                                                                    .map_or(
                                                                        String::from("internal"),
                                                                        |m| {
                                                                            m.map(|m| {
                                                                                m.token.value.clone()
                                                                            })
                                                                            .unwrap_or(String::from(
                                                                                "internal",
                                                                            ))
                                                                        },
                                                                    )
                                                            })
                                                            .unwrap_or(String::from("internal"))
                                                    ),
                                                    &token,
                                                );
                                                return None;
                                            }
                                            if let Type::Function(_, ret_type, _, None) = &*t.borrow() {
                                                return Some(ret_type.clone());
                                            }
                                        }
                                    }
                                    None
                                })
                            }
                            _ => None,
                        };
                        if let Some(ret_type) = lookup_result {
                            for p in parameters.iter() {
                                self.infer_type(p.expression.clone());
                            }
                            *ty = Some(ret_type.clone());
                            return Some(ret_type);
                        }
                    }
                    for p in parameters.iter() {
                        self.infer_type(p.expression.clone());
                    }
                    *ty = Some(result.clone());
                    result
                }
            }
            Expression::MacroInvocation { ty, .. } => ty
                .clone()
                .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void))),
            Expression::SizeOf { argument, ty, .. } => {
                self.infer_type(argument.clone());
                let result = Rc::new(RefCell::new(Type::UInt64));
                *ty = Some(result.clone());
                result
            }
            Expression::Do {
                body,
                catch_clauses,
                finally_body,
                ty,
                scope,
                ..
            } => {
                if let Some(sc) = scope {
                    self.enter_scope(sc.clone());
                    self.yield_context_depth += 1;
                    self.resolve_block_expression(body);
                    for clause in catch_clauses {
                        if let Some(guard) = &clause.guard {
                            self.infer_type(guard.clone());
                        }
                        self.resolve_block_expression(&clause.body);
                    }
                    self.resolve_block_expression(finally_body);
                    let block_ty = self.get_block_type(body)?;
                    *ty = Some(block_ty.clone());
                    self.yield_context_depth -= 1;
                    self.leave_scope();
                    block_ty
                } else {
                    self.yield_context_depth += 1;
                    let block_ty = self.get_block_type(body)?;
                    *ty = Some(block_ty.clone());
                    for clause in catch_clauses {
                        if let Some(guard) = &clause.guard {
                            self.infer_type(guard.clone());
                        }
                        self.resolve_block_expression(&clause.body);
                    }
                    self.resolve_block_expression(finally_body);
                    self.yield_context_depth -= 1;
                    block_ty
                }
            }
            Expression::ArrayLiteral { elements, ty, .. } => {
                for element in elements {
                    self.infer_type(element.clone());
                }
                if let Some(t) = ty.as_ref() {
                    t.clone()
                } else if let Some(current_scope) = &self.current_scope {
                    if let Some(t) = current_scope.borrow().get_type("Array") {
                        *ty = Some(t.clone());
                        t
                    } else {
                        let array_ty = Rc::new(RefCell::new(Type::Struct(
                            "Array".to_string(),
                            WeakSymbol(std::rc::Weak::new()),
                            vec![],
                        )));
                        *ty = Some(array_ty.clone());
                        array_ty
                    }
                } else {
                    let array_ty = Rc::new(RefCell::new(Type::Struct(
                        "Array".to_string(),
                        WeakSymbol(std::rc::Weak::new()),
                        vec![],
                    )));
                    *ty = Some(array_ty.clone());
                    array_ty
                }
            }
            Expression::InlineType {
                base, ty, token, ..
            } => {
                let base_ty = self.infer_type(base.clone())?;
                match &*base_ty.borrow() {
                    Type::Class(_, ..) => {}
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            format!("'inline' requires a class type, got '{}'", base_ty.borrow()),
                            token.as_ref(),
                        );
                        return None;
                    }
                }
                *ty = Some(Rc::new(RefCell::new(Type::Inline(base_ty, None))));
                ty.clone().unwrap()
            }
            Expression::OptionalType { inner, ty } => {
                let inner_ty = self.infer_type(inner.clone())?;
                *ty = self.create_parameterized_type_from_truss("Optional", inner_ty.clone());
                ty.clone().unwrap()
            }
            Expression::ArrayType { inner, ty } => {
                let inner_ty = self.infer_type(inner.clone())?;
                *ty = self.create_parameterized_type_from_truss("Array", inner_ty.clone());
                ty.clone().unwrap()
            }
            Expression::Try {
                kind, expression, ty, ..
            } => {
                let inner_ty = self.infer_type(expression.clone())?;
                match kind {
                    TryKind::Optional => {
                        let opt_ty =
                            self.create_parameterized_type_from_truss("Optional", inner_ty)?;
                        *ty = Some(opt_ty.clone());
                        opt_ty
                    }
                    _ => {
                        *ty = Some(inner_ty.clone());
                        inner_ty
                    }
                }
            }
        };
        Some(result)
    }

    fn infer_statement_type(
        &mut self,
        statement: Rc<RefCell<Statement>>,
    ) -> Option<Rc<RefCell<Type>>> {
        match &*statement.borrow() {
            Statement::ExpressionStatement { expression } => self.infer_type(expression.clone()),
            Statement::Return {
                value: Some(value), ..
            } => self.infer_type(value.clone()),
            Statement::Return { value: None, .. } => Some(Rc::new(RefCell::new(Type::Void))),
            Statement::Yield {
                value: Some(value), ..
            } => {
                let token = value.borrow().token();
                if let Some(expected) = self.current_return_type.clone() {
                    self.check_type_with_expected(value.clone(), expected, &token);
                } else if self.yield_context_depth == 0 {
                    self.emit_error(
                        TrussDiagnosticCode::YieldNotAllowedHere,
                        "Yield statement outside function or expression context",
                        &token,
                    );
                }
                self.infer_type(value.clone())
            }
            Statement::Yield { value: None, .. } => Some(Rc::new(RefCell::new(Type::Void))),
            Statement::VariableDecl { ty, .. } => {
                Some(ty.clone().unwrap_or(Rc::new(RefCell::new(Type::Void))))
            }
            _ => Some(Rc::new(RefCell::new(Type::Void))),
        }
    }

    fn check_type_with_expected(
        &mut self,
        expression: Rc<RefCell<Expression>>,
        expected: Rc<RefCell<Type>>,
        token: &Token,
    ) {
        let is_int_literal = matches!(&*expression.borrow(), Expression::IntegerLiteral { .. });
        let is_float_literal = matches!(&*expression.borrow(), Expression::DecimalLiteral { .. });
        let is_nullptr = matches!(&*expression.borrow(), Expression::NullptrLiteral { .. });
        let is_null = matches!(&*expression.borrow(), Expression::NullLiteral { .. });
        let is_array_literal = matches!(&*expression.borrow(), Expression::ArrayLiteral { .. });

        if is_int_literal {
            let is_expected_optional =
                matches!(&*expected.borrow(), Type::Enum(name, ..) if name == "Optional");
            if Self::is_integer_type(&expected.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::IntegerLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else if is_expected_optional {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::IntegerLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {}, found integer literal",
                        expected.borrow()
                    ),
                    token,
                );
            }
        } else if is_float_literal {
            let is_expected_optional =
                matches!(&*expected.borrow(), Type::Enum(name, ..) if name == "Optional");
            if Self::is_float_type(&expected.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::DecimalLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else if is_expected_optional {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::DecimalLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {}, found float literal",
                        expected.borrow()
                    ),
                    token,
                );
            }
        } else if is_nullptr {
            if matches!(&*expected.borrow(), Type::NonNullPointer(_)) {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    "Cannot assign nullptr to non-nullable pointer type",
                    token,
                );
                return;
            }
            let mut expr_mut = expression.borrow_mut();
            if let Expression::NullptrLiteral { ty, .. } = &mut *expr_mut {
                *ty = Some(expected.clone());
            }
            drop(expr_mut);
        } else if is_null {
            let is_expected_optional = match &*expected.borrow() {
                Type::Enum(name, ..) | Type::Struct(name, ..) | Type::Class(name, ..) => {
                    name == "Optional"
                }
                _ => false,
            };
            if is_expected_optional {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::NullLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else if !matches!(&*expected.borrow(), Type::Void) {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!("Type mismatch: expected {}, found null", expected.borrow()),
                    token,
                );
            }
        } else if is_array_literal {
            let is_expected_array = match &*expected.borrow() {
                Type::Class(name, ..) | Type::Struct(name, ..) => name == "Array",
                _ => false,
            };
            if is_expected_array {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::ArrayLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {}, found array literal",
                        expected.borrow()
                    ),
                    token,
                );
            }
        } else {
            self.closure_expected_type = Some(expected.clone());
            let inferred = self.infer_type(expression.clone());
            self.closure_expected_type = None;
            if let Some(inferred) = inferred {
                let inferred_clone = inferred.borrow().clone();
                let expected_clone = expected.borrow().clone();
                let is_protocol_compat =
                    matches!(&expected_clone, Type::Protocol(..) | Type::Compound(..));

                let is_optional_box = if let Type::Enum(name, ..) = &expected_clone {
                    name == "Optional"
                } else {
                    false
                };

                if is_optional_box {
                    let mut expr_mut = expression.borrow_mut();
                    if let Ok(ty) = expr_mut.get_ty_mut_ref() {
                        *ty = Some(expected.clone());
                    }
                    drop(expr_mut);
                } else if !is_protocol_compat
                    && inferred_clone != expected_clone
                    && !Self::types_are_type_compatible(&inferred_clone, &expected_clone)
                {
                    self.emit_error(
                        TrussDiagnosticCode::TypeMismatch,
                        format!(
                            "Type mismatch: expected {}, found {}",
                            expected_clone, inferred_clone
                        ),
                        token,
                    );
                }
            } else {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!("Type mismatch: expected {}", expected.borrow()),
                    token,
                );
            }
        }
    }

    fn is_integer_type(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Int128
                | Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::UInt128
        )
    }

    fn is_float_type(ty: &Type) -> bool {
        matches!(ty, Type::Float32 | Type::Float64)
    }

    fn is_numeric_type(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Int128
                | Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::UInt128
                | Type::Float32
                | Type::Float64
        )
    }

    fn substitute_generic_params(
        ty: Rc<RefCell<Type>>,
        mapping: &std::collections::HashMap<String, Rc<RefCell<Type>>>,
    ) -> Rc<RefCell<Type>> {
        let borrowed = ty.borrow();
        match borrowed.clone() {
            Type::GenericParam(name) => {
                drop(borrowed);
                if let Some(concrete) = mapping.get(&name) {
                    concrete.clone()
                } else {
                    ty.clone()
                }
            }
            Type::Function(param_tys, ret_ty, is_vararg, None) => {
                drop(borrowed);
                let new_params: Vec<Rc<RefCell<Type>>> = param_tys
                    .iter()
                    .map(|p| Self::substitute_generic_params(p.clone(), mapping))
                    .collect();
                let new_ret = Self::substitute_generic_params(ret_ty.clone(), mapping);
                Rc::new(RefCell::new(Type::Function(new_params, new_ret, is_vararg, None)))
            }
            Type::Pointer(base) => {
                drop(borrowed);
                let new_base = Self::substitute_generic_params(base.clone(), mapping);
                Rc::new(RefCell::new(Type::Pointer(new_base)))
            }
            Type::NonNullPointer(base) => {
                drop(borrowed);
                let new_base = Self::substitute_generic_params(base.clone(), mapping);
                Rc::new(RefCell::new(Type::NonNullPointer(new_base)))
            }
            Type::Tuple(elements) => {
                drop(borrowed);
                let new_elements: Vec<(Option<String>, Rc<RefCell<Type>>)> = elements
                    .into_iter()
                    .map(|(name, t)| (name, Self::substitute_generic_params(t, mapping)))
                    .collect();
                Rc::new(RefCell::new(Type::Tuple(new_elements)))
            }
            Type::Compound(types) => {
                drop(borrowed);
                let new_types: Vec<Rc<RefCell<Type>>> = types
                    .into_iter()
                    .map(|t| Self::substitute_generic_params(t, mapping))
                    .collect();
                Rc::new(RefCell::new(Type::Compound(new_types)))
            }
            Type::AssociatedType(base, name) => {
                drop(borrowed);
                let new_base = Self::substitute_generic_params(base.clone(), mapping);
                Rc::new(RefCell::new(Type::AssociatedType(new_base, name)))
            }
            other => {
                drop(borrowed);
                Rc::new(RefCell::new(other))
            }
        }
    }

    fn infer_generic_params_from_call(
        &mut self,
        func_decl: &Rc<RefCell<Statement>>,
        type_parameters: &Option<Vec<Rc<RefCell<Expression>>>>,
        param_tys: &[Rc<RefCell<Type>>],
        parameters: &[CallParameter],
    ) -> Option<std::collections::HashMap<String, Rc<RefCell<Type>>>> {
        let decl = func_decl.borrow();
        let generic_params = match &*decl {
            Statement::FunctionDecl {
                generic_parameters, ..
            } => generic_parameters,
            _ => return None,
        };

        if generic_params.is_empty() {
            return None;
        }

        let mut mapping: std::collections::HashMap<String, Rc<RefCell<Type>>> =
            std::collections::HashMap::new();

        if let Some(explicit_tps) = type_parameters {
            for (i, tp_expr) in explicit_tps.iter().enumerate() {
                if i >= generic_params.len() {
                    break;
                }
                let tp_ty = self.infer_type(tp_expr.clone())?;
                mapping.insert(generic_params[i].name.value.clone(), tp_ty);
            }
            return Some(mapping);
        }

        for (i, param) in parameters.iter().enumerate() {
            if i >= param_tys.len() {
                break;
            }
            let param_ty = &param_tys[i];
            let arg_ty = self.infer_type(param.expression.clone())?;

            Self::collect_generic_mappings(param_ty.clone(), arg_ty, &mut mapping);
        }

        if mapping.is_empty() {
            return None;
        }
        Some(mapping)
    }

    fn collect_generic_mappings(
        param_ty: Rc<RefCell<Type>>,
        arg_ty: Rc<RefCell<Type>>,
        mapping: &mut std::collections::HashMap<String, Rc<RefCell<Type>>>,
    ) {
        let p_clone = param_ty.borrow().clone();
        let a_clone = arg_ty.borrow().clone();
        match (&p_clone, &a_clone) {
            (Type::GenericParam(name), _) => {
                if !mapping.contains_key(name) {
                    mapping.insert(name.clone(), arg_ty.clone());
                }
            }
            (Type::Function(p1, r1, _, None), Type::Function(p2, r2, _, None)) => {
                for (pt, at) in p1.iter().zip(p2.iter()) {
                    Self::collect_generic_mappings(pt.clone(), at.clone(), mapping);
                }
                Self::collect_generic_mappings(r1.clone(), r2.clone(), mapping);
            }
            (Type::Pointer(b1), Type::Pointer(b2)) => {
                Self::collect_generic_mappings(b1.clone(), b2.clone(), mapping);
            }
            (Type::NonNullPointer(b1), Type::NonNullPointer(b2)) => {
                Self::collect_generic_mappings(b1.clone(), b2.clone(), mapping);
            }
            (Type::Tuple(e1), Type::Tuple(e2)) => {
                for ((_, t1), (_, t2)) in e1.iter().zip(e2.iter()) {
                    Self::collect_generic_mappings(t1.clone(), t2.clone(), mapping);
                }
            }
            (Type::Compound(t1), Type::Compound(t2)) => {
                for (t1, t2) in t1.iter().zip(t2.iter()) {
                    Self::collect_generic_mappings(t1.clone(), t2.clone(), mapping);
                }
            }
            _ => {}
        }
    }

    fn infer_expression_type(
        &mut self,
        expression: Rc<RefCell<Expression>>,
        expected_type: Rc<RefCell<Type>>,
    ) -> Option<Rc<RefCell<Type>>> {
        let expr_ref = expression.borrow();
        let is_int_literal = matches!(&*expr_ref, Expression::IntegerLiteral { .. });
        let is_float_literal = matches!(&*expr_ref, Expression::DecimalLiteral { .. });
        drop(expr_ref);

        if is_int_literal {
            if Self::is_integer_type(&expected_type.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::IntegerLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected_type.clone());
                }
                return Some(expected_type);
            } else {
                let token = expression.borrow().token();
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {}, found integer literal",
                        expected_type.borrow()
                    ),
                    &token,
                );
                return None;
            }
        }

        if is_float_literal {
            if Self::is_float_type(&expected_type.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::DecimalLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected_type.clone());
                }
                return Some(expected_type);
            } else {
                let token = expression.borrow().token();
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {}, found float literal",
                        expected_type.borrow()
                    ),
                    &token,
                );
                return None;
            }
        }

        if let Expression::NullptrLiteral { ty, .. } = &*expression.borrow() {
            if ty.is_none() {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::NullptrLiteral { ty: nullptr_ty, .. } = &mut *expr_mut {
                    *nullptr_ty = Some(expected_type.clone());
                }
            }
            return Some(expected_type);
        }

        self.infer_type(expression)
    }

    fn check_binary(
        &self,
        operator: BinaryOperator,
        left: Rc<RefCell<Type>>,
        right: Rc<RefCell<Type>>,
    ) -> Option<Rc<RefCell<Type>>> {
        match operator {
            BinaryOperator::Plus
            | BinaryOperator::Minus
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulus => {
                let left_ty = left.borrow().clone();
                let right_ty = right.borrow().clone();

                if !Self::is_numeric_type(&left_ty) {
                    return None;
                }
                if left_ty != right_ty {
                    return None;
                }
                Some(Rc::new(RefCell::new(left_ty)))
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                if left.borrow().clone() != right.borrow().clone() {
                    return None;
                }
                Some(Rc::new(RefCell::new(Type::Bool)))
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if *left.borrow() != Type::Bool {
                    return None;
                }
                if *right.borrow() != Type::Bool {
                    return None;
                }
                Some(Rc::new(RefCell::new(Type::Bool)))
            }
            _ => None,
        }
    }

    fn check_cast(source: &Type, target: &Type) -> bool {
        if *source == *target {
            return true;
        }
        match (source, target) {
            (Type::Never, _) => true,
            (Type::Pointer(_), Type::Pointer(_)) => true,
            (Type::NonNullPointer(_), Type::Pointer(_)) => true,
            (Type::NonNullPointer(_), Type::NonNullPointer(_)) => true,
            (s, t) if Self::is_numeric_type(s) && Self::is_numeric_type(t) => true,
            (Type::Bool, t) if Self::is_integer_type(t) => true,
            (s, Type::Bool) if Self::is_integer_type(s) => true,
            (Type::Bool, t) if Self::is_float_type(t) => false,
            (s, Type::Bool) if Self::is_float_type(s) => false,
            (Type::Char, t) if Self::is_integer_type(t) => true,
            (s, Type::Char) if Self::is_integer_type(s) => true,
            _ => false,
        }
    }

    fn get_type_size_bits(ty: &Type) -> Option<u32> {
        match ty {
            Type::Int8 | Type::UInt8 => Some(8),
            Type::Int16 | Type::UInt16 => Some(16),
            Type::Int32 | Type::UInt32 | Type::Float32 => Some(32),
            Type::Int64 | Type::UInt64 | Type::Float64 => Some(64),
            Type::Int128 | Type::UInt128 => Some(128),
            Type::Bool | Type::Char => Some(8),
            Type::Pointer(_) => Some(64),
            Type::NonNullPointer(_) => Some(64),
            _ => None,
        }
    }

    fn check_cast_bitcast(source: &Type, target: &Type) -> bool {
        let source_size = Self::get_type_size_bits(source);
        let target_size = Self::get_type_size_bits(target);

        match (source_size, target_size) {
            (Some(s), Some(t)) => {
                if s != t {
                    return false;
                }
                true
            }
            _ => true,
        }
    }

    fn check_unary(
        &self,
        operator: UnaryOperator,
        operand: Rc<RefCell<Type>>,
    ) -> Option<Rc<RefCell<Type>>> {
        match operator {
            UnaryOperator::Plus | UnaryOperator::Minus => {
                let op_ty = operand.borrow().clone();
                if !Self::is_numeric_type(&op_ty) {
                    return None;
                }
                Some(Rc::new(RefCell::new(op_ty)))
            }
            UnaryOperator::Inc | UnaryOperator::Dec => {
                let op_ty = operand.borrow().clone();
                if !Self::is_numeric_type(&op_ty) {
                    return None;
                }
                Some(Rc::new(RefCell::new(op_ty)))
            }
            UnaryOperator::BitNot => {
                let op_ty = operand.borrow().clone();
                if !Self::is_integer_type(&op_ty) {
                    return None;
                }
                Some(Rc::new(RefCell::new(op_ty)))
            }
            UnaryOperator::Deref => {
                let op_ty = operand.borrow().clone();
                if let Type::Pointer(inner_ty) = op_ty {
                    Some(inner_ty)
                } else if let Type::NonNullPointer(inner_ty) = operand.borrow().clone() {
                    Some(inner_ty)
                } else {
                    None
                }
            }
            UnaryOperator::AddressOf => Some(Rc::new(RefCell::new(Type::Pointer(operand)))),
            _ => None,
        }
    }

    fn collect_method_overloads(
        &mut self,
        object: Rc<RefCell<Expression>>,
        member_name: &str,
    ) -> Option<Vec<Rc<RefCell<Symbol>>>> {
        let object_ty = self.infer_type(object)?;
        let type_name = match &*object_ty.borrow() {
            Type::Struct(n, ..) | Type::Class(n, ..) | Type::Enum(n, ..) => n.clone(),
            _ => return None,
        };
        let scope = self.current_scope.as_ref().unwrap().borrow();
        let symbol = scope.get_symbol(&type_name)?;
        let methods = match &*symbol.borrow() {
            Symbol::Struct { methods, .. }
            | Symbol::Class { methods, .. }
            | Symbol::Enum { methods, .. } => methods.clone(),
            _ => return None,
        };
        let matching: Vec<Rc<RefCell<Symbol>>> = methods
            .iter()
            .filter(|m| m.borrow().name().as_ref().ok() == Some(&member_name.to_string()))
            .cloned()
            .collect();
        if matching.is_empty() {
            None
        } else {
            Some(matching)
        }
    }

    fn try_resolve_binary_operator(
        &mut self,
        operator: BinaryOperator,
        left: Rc<RefCell<Expression>>,
        left_ty: Rc<RefCell<Type>>,
        right: Rc<RefCell<Expression>>,
        _right_ty: Rc<RefCell<Type>>,
        bin_overloads: &mut Vec<Rc<RefCell<Symbol>>>,
        bin_selected: &mut Option<usize>,
    ) -> Option<Rc<RefCell<Type>>> {
        let type_name = match &*left_ty.borrow() {
            Type::Struct(n, ..) | Type::Class(n, ..) | Type::Enum(n, ..) => n.clone(),
            _ => return None,
        };
        let matching = {
            let scope = self.current_scope.as_ref().unwrap().borrow();
            let sym = scope.get_symbol(&type_name)?;
            let methods = match &*sym.borrow() {
                Symbol::Struct { methods, .. }
                | Symbol::Class { methods, .. }
                | Symbol::Enum { methods, .. } => methods.clone(),
                _ => return None,
            };
            let op_name = operator.operator_name().to_string();
            let filtered: Vec<Rc<RefCell<Symbol>>> = methods
                .iter()
                .filter(|m| {
                    let b = m.borrow();
                    if b.name().as_ref().ok() != Some(&op_name) {
                        return false;
                    }
                    if let Some(decl) = b.get_decl().ok() {
                        if let Some(d) = decl {
                            if let Statement::FunctionDecl {
                                operator_fixity, ..
                            } = &*d.borrow()
                            {
                                return operator_fixity.is_none();
                            }
                        }
                    }
                    false
                })
                .cloned()
                .collect();
            filtered
        };
        if matching.is_empty() {
            return None;
        }
        let (static_matches, member_matches): (Vec<_>, Vec<_>) =
            matching.into_iter().partition(|m| {
                m.borrow()
                    .get_decl()
                    .ok()
                    .and_then(|d| {
                        d.and_then(|decl| {
                            if let Statement::FunctionDecl { static_method, .. } = &*decl.borrow() {
                                Some(*static_method)
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(false)
            });
        if !member_matches.is_empty() {
            let right_param = CallParameter {
                label: None,
                expression: right.clone(),
            };
            let params = vec![right_param];
            let mut sel = None;
            if let Some(ret_ty) = self.resolve_operator_overload(&params, &member_matches, &mut sel)
            {
                if let Some(idx) = sel {
                    *bin_overloads = member_matches;
                    *bin_selected = Some(idx);
                }
                return Some(ret_ty);
            }
        }
        if !static_matches.is_empty() {
            let left_param = CallParameter {
                label: None,
                expression: left,
            };
            let right_param = CallParameter {
                label: None,
                expression: right,
            };
            let params = vec![left_param, right_param];
            let mut sel = None;
            if let Some(ret_ty) = self.resolve_operator_overload(&params, &static_matches, &mut sel)
            {
                if let Some(idx) = sel {
                    *bin_overloads = static_matches;
                    *bin_selected = Some(idx);
                }
                return Some(ret_ty);
            }
        }
        None
    }

    fn try_resolve_unary_operator(
        &mut self,
        operator: UnaryOperator,
        operand: Rc<RefCell<Expression>>,
        operand_ty: Rc<RefCell<Type>>,
        un_overloads: &mut Vec<Rc<RefCell<Symbol>>>,
        un_selected: &mut Option<usize>,
    ) -> Option<Rc<RefCell<Type>>> {
        let type_name = match &*operand_ty.borrow() {
            Type::Struct(n, ..) | Type::Class(n, ..) | Type::Enum(n, ..) => n.clone(),
            _ => return None,
        };
        let matching = {
            let scope = self.current_scope.as_ref().unwrap().borrow();
            let sym = scope.get_symbol(&type_name)?;
            let methods = match &*sym.borrow() {
                Symbol::Struct { methods, .. }
                | Symbol::Class { methods, .. }
                | Symbol::Enum { methods, .. } => methods.clone(),
                _ => return None,
            };
            let op_name = operator.operator_name().to_string();
            let filtered: Vec<Rc<RefCell<Symbol>>> = methods
                .iter()
                .filter(|m| {
                    let b = m.borrow();
                    if b.name().as_ref().ok() != Some(&op_name) {
                        return false;
                    }
                    if let Some(decl) = b.get_decl().ok() {
                        if let Some(d) = decl {
                            if let Statement::FunctionDecl {
                                operator_fixity, ..
                            } = &*d.borrow()
                            {
                                return *operator_fixity == Some(OperatorFixity::Prefix);
                            }
                        }
                    }
                    false
                })
                .cloned()
                .collect();
            filtered
        };
        if matching.is_empty() {
            return None;
        }
        let (static_matches, member_matches): (Vec<_>, Vec<_>) =
            matching.into_iter().partition(|m| {
                m.borrow()
                    .get_decl()
                    .ok()
                    .and_then(|d| {
                        d.and_then(|decl| {
                            if let Statement::FunctionDecl { static_method, .. } = &*decl.borrow() {
                                Some(*static_method)
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(false)
            });
        if !member_matches.is_empty() {
            let params = vec![];
            let mut sel = None;
            if let Some(ret_ty) = self.resolve_operator_overload(&params, &member_matches, &mut sel)
            {
                if let Some(idx) = sel {
                    *un_overloads = member_matches;
                    *un_selected = Some(idx);
                }
                return Some(ret_ty);
            }
        }
        if !static_matches.is_empty() {
            let operand_param = CallParameter {
                label: None,
                expression: operand,
            };
            let params = vec![operand_param];
            let mut sel = None;
            if let Some(ret_ty) = self.resolve_operator_overload(&params, &static_matches, &mut sel)
            {
                return Some(ret_ty);
            }
        }
        None
    }

    fn resolve_operator_overload(
        &mut self,
        parameters: &[CallParameter],
        overloads: &[Rc<RefCell<Symbol>>],
        selected_index: &mut Option<usize>,
    ) -> Option<Rc<RefCell<Type>>> {
        for (i, sym) in overloads.iter().enumerate() {
            let Some((param_tys, ret_ty, is_vararg, _decl)) =
                self.get_fn_info_from_symbol(sym.clone())
            else {
                continue;
            };
            if !is_vararg && parameters.len() != param_tys.len() {
                continue;
            }
            if is_vararg && parameters.len() < param_tys.len() {
                continue;
            }
            let mut all_match = true;
            for (j, call_param) in parameters.iter().enumerate() {
                if j >= param_tys.len() {
                    break;
                }
                let Some(arg_ty) = self.infer_type(call_param.expression.clone()) else {
                    all_match = false;
                    break;
                };
                if !Self::types_are_type_compatible(&arg_ty.borrow(), &param_tys[j].borrow()) {
                    all_match = false;
                    break;
                }
            }
            if all_match {
                *selected_index = Some(i);
                return Some(ret_ty);
            }
        }
        None
    }

    fn get_fn_info_from_symbol(
        &self,
        sym: Rc<RefCell<Symbol>>,
    ) -> Option<(
        Vec<Rc<RefCell<Type>>>,
        Rc<RefCell<Type>>,
        bool,
        Rc<RefCell<Statement>>,
    )> {
        let decl = sym.borrow().get_decl().ok()??;
        let decl_borrow = decl.borrow();
        if let Statement::FunctionDecl {
            ty: Some(fn_type), ..
        } = &*decl_borrow
        {
            let fn_borrow = fn_type.borrow();
            if let Type::Function(param_tys, ret_ty, is_vararg, None) = &*fn_borrow {
                return Some((param_tys.clone(), ret_ty.clone(), *is_vararg, decl.clone()));
            }
        }
        None
    }

    fn resolve_overloaded_call(
        &mut self,
        callee: Rc<RefCell<Expression>>,
        parameters: &[CallParameter],
        overloads: &[Rc<RefCell<Symbol>>],
        selected_index: &mut Option<usize>,
    ) -> Option<Rc<RefCell<Type>>> {
        let mut best_idx: Option<usize> = None;
        let mut best_info: Option<(
            Vec<Rc<RefCell<Type>>>,
            Rc<RefCell<Type>>,
            bool,
            Rc<RefCell<Statement>>,
        )> = None;

        for (i, sym) in overloads.iter().enumerate() {
            let Some((param_tys, ret_ty, is_vararg, decl)) =
                self.get_fn_info_from_symbol(sym.clone())
            else {
                continue;
            };

            if !is_vararg && parameters.len() != param_tys.len() {
                continue;
            }
            if is_vararg && parameters.len() < param_tys.len() {
                continue;
            }

            let mut all_match = true;
            for (j, call_param) in parameters.iter().enumerate() {
                if j >= param_tys.len() {
                    break;
                }
                let inferred = self.infer_type(call_param.expression.clone());
                let Some(arg_ty) = inferred else {
                    all_match = false;
                    break;
                };
                let expected_ty = &param_tys[j];
                if !Self::types_are_type_compatible(&arg_ty.borrow(), &expected_ty.borrow()) {
                    all_match = false;
                    break;
                }
            }

            if !all_match {
                continue;
            }

            if best_idx.is_some() {
                let token = &callee.borrow().token();
                self.emit_error(
                    TrussDiagnosticCode::AmbiguousOverload,
                    "Call is ambiguous: multiple overloads match the provided arguments",
                    token,
                );
                return None;
            }

            best_idx = Some(i);
            best_info = Some((param_tys, ret_ty, is_vararg, decl));
        }

        if let Some(idx) = best_idx {
            *selected_index = Some(idx);
            if let Some((param_tys, ret_ty, _is_vararg, decl)) = best_info {
                for (i, param) in parameters.iter().enumerate() {
                    if i < param_tys.len() {
                        let expected_ty = param_tys[i].clone();
                        self.infer_expression_type(param.expression.clone(), expected_ty);
                        self.check_parameter_label(param, &decl, i);
                    }
                }
                Some(ret_ty)
            } else {
                None
            }
        } else {
            let token = &callee.borrow().token();
            self.emit_error(
                TrussDiagnosticCode::NoMatchingOverload,
                "No matching overload found for the provided arguments",
                token,
            );
            None
        }
    }

    fn types_are_type_compatible(a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Int8, Type::Int8)
            | (Type::Int16, Type::Int16)
            | (Type::Int32, Type::Int32)
            | (Type::Int64, Type::Int64)
            | (Type::Int128, Type::Int128)
            | (Type::UInt8, Type::UInt8)
            | (Type::UInt16, Type::UInt16)
            | (Type::UInt32, Type::UInt32)
            | (Type::UInt64, Type::UInt64)
            | (Type::UInt128, Type::UInt128)
            | (Type::Float32, Type::Float32)
            | (Type::Float64, Type::Float64)
            | (Type::Bool, Type::Bool)
            | (Type::Char, Type::Char)
            | (Type::Void, Type::Void)
            | (Type::Never, Type::Never) => true,
            (Type::Struct(n1, ..), Type::Struct(n2, ..))
            | (Type::Class(n1, ..), Type::Class(n2, ..))
            | (Type::Enum(n1, ..), Type::Enum(n2, ..))
            | (Type::Protocol(n1, ..), Type::Protocol(n2, ..)) => n1 == n2,
            (Type::Pointer(t1), Type::Pointer(t2)) => {
                Self::types_are_type_compatible(&t1.borrow(), &t2.borrow())
            }
            (Type::NonNullPointer(t1), Type::NonNullPointer(t2)) => {
                Self::types_are_type_compatible(&t1.borrow(), &t2.borrow())
            }
            (Type::Tuple(e1), Type::Tuple(e2)) => {
                e1.len() == e2.len()
                    && e1.iter().zip(e2.iter()).all(|((_, t1), (_, t2))| {
                        Self::types_are_type_compatible(&t1.borrow(), &t2.borrow())
                    })
            }
            (Type::Function(p1, r1, v1, None), Type::Function(p2, r2, v2, None)) => {
                v1 == v2
                    && p1.len() == p2.len()
                    && p1
                        .iter()
                        .zip(p2.iter())
                        .all(|(t1, t2)| Self::types_are_type_compatible(&t1.borrow(), &t2.borrow()))
                    && Self::types_are_type_compatible(&r1.borrow(), &r2.borrow())
            }
            (Type::GenericParam(n1), Type::GenericParam(n2)) => n1 == n2,
            (Type::AssociatedType(t1, n1), Type::AssociatedType(t2, n2)) => {
                n1 == n2 && Self::types_are_type_compatible(&t1.borrow(), &t2.borrow())
            }
            (Type::Compound(t1), Type::Compound(t2)) => {
                t1.len() == t2.len()
                    && t1
                        .iter()
                        .zip(t2.iter())
                        .all(|(t1, t2)| Self::types_are_type_compatible(&t1.borrow(), &t2.borrow()))
            }
            (Type::Inline(a_inner, _), Type::Inline(b_inner, _)) => {
                Self::types_are_type_compatible(&a_inner.borrow(), &b_inner.borrow())
            }
            (Type::Inline(a_inner, _), b) => Self::types_are_type_compatible(&a_inner.borrow(), b),
            (a, Type::Inline(b_inner, _)) => Self::types_are_type_compatible(a, &b_inner.borrow()),
            _ => false,
        }
    }

    fn get_function_decl_from_callee(
        &self,
        callee: Rc<RefCell<Expression>>,
    ) -> Option<Rc<RefCell<Statement>>> {
        if let Expression::Variable { symbol, .. } = &*callee.borrow()
            && let Some(sym) = symbol
            && let Some(sym) = sym.0.upgrade()
        {
            sym.borrow().get_decl().unwrap()
        } else {
            None
        }
    }

    fn get_effective_label(param: &Parameter) -> Option<String> {
        match &param.label {
            Some(label_token) if label_token.value == "_" => None,
            Some(label_token) => Some(label_token.value.clone()),
            None => Some(param.name.value.clone()),
        }
    }

    fn reorder_call_parameters(
        &self,
        parameters: &mut Vec<CallParameter>,
        decl_params: &[Rc<RefCell<Parameter>>],
        callee_token: &Token,
    ) {
        let mut label_to_idx: HashMap<String, usize> = HashMap::new();
        let mut unlabeled_indices: Vec<usize> = Vec::new();

        for (i, dp) in decl_params.iter().enumerate() {
            match Self::get_effective_label(&dp.borrow()) {
                Some(label) => {
                    label_to_idx.entry(label).or_insert(i);
                }
                None => {
                    unlabeled_indices.push(i);
                }
            }
        }

        let old_params = std::mem::take(parameters);
        let mut new_params: Vec<Option<CallParameter>> = vec![None; decl_params.len()];
        let mut matched = vec![false; decl_params.len()];
        let mut next_pos = 0;

        let mut has_errors = false;

        for cp in old_params {
            let is_underscore_label = cp.label.as_ref().is_some_and(|l| l.value == "_");
            if let Some(label_token) = &cp.label
                && !is_underscore_label
            {
                if let Some(&idx) = label_to_idx.get(&label_token.value) {
                    if matched[idx] {
                        has_errors = true;
                        self.emit_error(
                            TrussDiagnosticCode::ArgumentLabelMismatch,
                            format!(
                                "Duplicate argument for parameter '{}'",
                                decl_params[idx].borrow().name.value
                            ),
                            label_token,
                        );
                    } else {
                        matched[idx] = true;
                        new_params[idx] = Some(cp);
                    }
                } else {
                    has_errors = true;
                    let decl_names: Vec<String> = decl_params
                        .iter()
                        .map(|dp| {
                            let b = dp.borrow();
                            match &b.label {
                                Some(l) if l.value != "_" => l.value.clone(),
                                None => b.name.value.clone(),
                                _ => String::new(),
                            }
                        })
                        .filter(|n| !n.is_empty())
                        .collect();
                    self.emit_error(
                        TrussDiagnosticCode::ArgumentLabelMismatch,
                        format!(
                            "Expected argument label '{}' but found '{}'",
                            decl_names.first().map_or("", |s| s),
                            label_token.value
                        ),
                        label_token,
                    );
                }
            } else {
                while next_pos < decl_params.len() && matched[next_pos] {
                    next_pos += 1;
                }
                if next_pos < decl_params.len() {
                    let pos = next_pos;
                    matched[pos] = true;
                    new_params[pos] = Some(cp);
                    next_pos += 1;
                } else {
                    has_errors = true;
                    self.emit_error(
                        TrussDiagnosticCode::ArgumentCountMismatch,
                        "Too many arguments".to_string(),
                        callee_token,
                    );
                }
            }
        }

        for (i, dp) in decl_params.iter().enumerate() {
            if !matched[i] {
                if let Some(default_value) = &dp.borrow().default_value {
                    new_params[i] = Some(CallParameter {
                        label: None,
                        expression: default_value.clone(),
                    });
                } else if !has_errors {
                    let label_info = match &dp.borrow().label {
                        Some(l) if l.value != "_" => format!(" '{}'", l.value),
                        None => format!(" '{}'", dp.borrow().name.value),
                        _ => String::new(),
                    };
                    self.emit_error(
                        TrussDiagnosticCode::ArgumentCountMismatch,
                        format!("Missing required argument{}", label_info),
                        callee_token,
                    );
                }
            }
        }

        *parameters = new_params.into_iter().flatten().collect();
    }

    fn check_parameter_label(
        &self,
        call_param: &CallParameter,
        func_decl: &Rc<RefCell<Statement>>,
        param_index: usize,
    ) {
        let decl_borrow = func_decl.borrow();
        let parameters = match &*decl_borrow {
            Statement::FunctionDecl { parameters, .. } => parameters,
            Statement::InitDecl { parameters, .. } => parameters,
            _ => return,
        };
        if param_index >= parameters.len() {
            return;
        }

        let decl_param = &parameters[param_index];
        let decl_param_label = &decl_param.borrow().label;
        let decl_param_name = &decl_param.borrow().name;
        let provided_label = &call_param.label;

        let expected_label: Option<&Token>;

        if let Some(label) = decl_param_label {
            if label.value == "_" {
                expected_label = None;
            } else {
                expected_label = Some(label);
            }
        } else {
            expected_label = Some(decl_param_name);
        }

        match (expected_label, provided_label) {
            (Some(expected), Some(provided)) => {
                if expected.value != provided.value {
                    self.emit_error(
                        TrussDiagnosticCode::ArgumentLabelMismatch,
                        format!(
                            "Expected argument label '{}' but found '{}'",
                            expected.value, provided.value
                        ),
                        provided,
                    );
                }
            }
            (Some(expected), None) => {
                if decl_param.borrow().default_value.is_none() {
                    let token = call_param.expression.borrow().token();
                    self.emit_error(
                        TrussDiagnosticCode::MissingArgumentLabel,
                        format!("Missing argument label '{}' in call", expected.value),
                        &token,
                    );
                }
            }
            (None, Some(provided)) => {
                if provided.value != "_" {
                    self.emit_error(
                        TrussDiagnosticCode::ArgumentLabelMismatch,
                        format!(
                            "Argument should not have a label, but found '{}'",
                            provided.value
                        ),
                        provided,
                    );
                }
            }
            (None, None) => {}
        }
    }

    fn is_member_accessible(&self, container: Rc<RefCell<Symbol>>, member_token: &Token) -> bool {
        let (access_modifier, decl_file) = {
            let symbol = container.borrow();
            let Some(decl_rc) = symbol.get_decl().ok().flatten() else {
                return true;
            };
            let Ok(decl) = decl_rc.try_borrow() else {
                return true;
            };
            let modifiers = decl.modifiers().unwrap();
            let access = modifiers.iter().find_map(|m| {
                if let ModifierType::Access(access) = &m.ty {
                    Some(*access)
                } else {
                    None
                }
            });
            let file = decl.token().file.clone();
            (access, file)
        };
        let Some(access) = access_modifier else {
            return true;
        };
        match access {
            AccessModifier::Open | AccessModifier::Public | AccessModifier::Internal => true,
            AccessModifier::Fileprivate => member_token.file == decl_file,
            AccessModifier::Private => self
                .current_owner
                .as_ref()
                .map_or(false, |owner| Rc::ptr_eq(owner, &container)),
            AccessModifier::Package => {
                // TODO: implement package access check
                true
            }
        }
    }

    fn is_member_symbol_accessible(
        &self,
        member: Rc<RefCell<Symbol>>,
        member_token: &Token,
    ) -> bool {
        let access_modifier = {
            let symbol = member.borrow();
            let Some(m) = symbol.get_decl().ok().flatten().and_then(|decl| {
                decl.borrow()
                    .modifiers()
                    .unwrap()
                    .iter()
                    .find(|m| matches!(m.ty, ModifierType::Access(_)))
                    .cloned()
            }) else {
                return true;
            };
            m.ty
        };
        let ModifierType::Access(access) = &access_modifier else {
            return true;
        };
        match access {
            AccessModifier::Open | AccessModifier::Public | AccessModifier::Internal => true,
            AccessModifier::Fileprivate => {
                let symbol = member.borrow();
                if let Some(decl) = symbol.get_decl().ok().flatten() {
                    let decl_file = decl.borrow().token().file;
                    member_token.file == decl_file
                } else {
                    true
                }
            }
            AccessModifier::Private => {
                let parent = member.borrow().parent();
                parent.is_some()
                    && self
                        .current_owner
                        .as_ref()
                        .map_or(false, |owner| Rc::ptr_eq(owner, &parent.unwrap()))
            }
            AccessModifier::Package => {
                // TODO: implement package access check
                true
            }
        }
    }

    fn get_enum_case_parameter_types(
        &self,
        enum_name: &str,
        case_name: &str,
    ) -> Option<Vec<Rc<RefCell<Type>>>> {
        let scope = self.current_scope.as_ref()?;
        let scope_ref = scope.borrow();
        let symbol = scope_ref.get_symbol(enum_name)?;
        let symbol_ref = symbol.borrow();
        if let Symbol::Enum { cases, .. } = &*symbol_ref {
            for case in cases {
                if case.borrow().name().as_ref().ok() == Some(&case_name.to_string()) {
                    if let Symbol::EnumCase {
                        parameter_types, ..
                    } = &*case.borrow()
                    {
                        return Some(parameter_types.clone());
                    }
                }
            }
        }
        None
    }

    fn resolve_enum_case_from_type(
        &self,
        expr_ty: &Rc<RefCell<Type>>,
        enum_name: Option<&str>,
        case_name: &Token,
        _bindings: &[Pattern],
    ) -> Option<Vec<Rc<RefCell<Type>>>> {
        if let Some(name) = enum_name {
            self.get_enum_case_parameter_types(name, &case_name.value)
        } else if let Type::Enum(enum_name, ..) = &*expr_ty.borrow() {
            self.get_enum_case_parameter_types(enum_name, &case_name.value)
        } else {
            None
        }
    }

    fn set_binding_types(
        bindings: &[Pattern],
        param_types: &[Rc<RefCell<Type>>],
        block_scope: &Rc<RefCell<Scope>>,
    ) {
        let mut scope_ref = block_scope.borrow_mut();
        for (i, binding) in bindings.iter().enumerate() {
            if i >= param_types.len() {
                break;
            }
            match binding {
                Pattern::Identifier(name) => {
                    if name.value != "_" {
                        scope_ref.set_type(name.value.clone(), param_types[i].clone());
                    }
                }
                Pattern::ValueBinding(inner) => {
                    if let Pattern::Identifier(name) = inner.as_ref() {
                        if name.value != "_" {
                            scope_ref.set_type(name.value.clone(), param_types[i].clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn enter_scope(&mut self, scope: Rc<RefCell<Scope>>) {
        self.current_scope = Some(scope);
    }

    fn leave_scope(&mut self) {
        self.current_scope = self.current_scope.clone().unwrap().borrow().parent.clone();
    }

    fn access_level_value(access: &AccessModifier) -> u8 {
        match access {
            AccessModifier::Open => 6,
            AccessModifier::Public => 5,
            AccessModifier::Package => 4,
            AccessModifier::Internal => 3,
            AccessModifier::Fileprivate => 2,
            AccessModifier::Private => 1,
        }
    }

    fn get_access_modifier(stmt: &Statement) -> Option<AccessModifier> {
        stmt.modifiers().ok().and_then(|modifiers| {
            modifiers.iter().find_map(|m| {
                if let ModifierType::Access(ref access) = m.ty {
                    Some(*access)
                } else {
                    None
                }
            })
        })
    }

    fn get_set_access_modifier(stmt: &Statement) -> Option<AccessModifier> {
        stmt.modifiers().ok().and_then(|modifiers| {
            modifiers.iter().find_map(|m| {
                if let ModifierType::AccessSet(ref access) = m.ty {
                    Some(*access)
                } else {
                    None
                }
            })
        })
    }

    fn is_setter_accessible(
        &self,
        set_access: &AccessModifier,
        member: Rc<RefCell<Symbol>>,
        member_token: &Token,
    ) -> bool {
        match set_access {
            AccessModifier::Open | AccessModifier::Public | AccessModifier::Internal => true,
            AccessModifier::Fileprivate => {
                let symbol = member.borrow();
                if let Some(decl) = symbol.get_decl().ok().flatten() {
                    let decl_file = decl.borrow().token().file;
                    member_token.file == decl_file
                } else {
                    true
                }
            }
            AccessModifier::Private => member
                .borrow()
                .parent()
                .and_then(|p| {
                    self.current_owner
                        .as_ref()
                        .map(|owner| Rc::ptr_eq(owner, &p))
                })
                .unwrap_or(false),
            AccessModifier::Package => true,
        }
    }

    fn validate_class_member_overrides(
        &self,
        _class_name: &Token,
        superclass: &Option<Rc<RefCell<Expression>>>,
        body: &[Rc<RefCell<Statement>>],
    ) {
        let superclass_symbol = superclass.as_ref().and_then(|super_expr| {
            if let Expression::Type {
                name: super_name, ..
            } = &*super_expr.borrow()
            {
                self.current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().get_symbol(&super_name.value))
            } else {
                None
            }
        });

        let (super_method_names, super_property_names, _is_super_abstract) = superclass_symbol
            .as_ref()
            .map(|sym| {
                let binding = sym.borrow();
                if let Symbol::Class {
                    methods,
                    properties,
                    is_abstract,
                    ..
                } = &*binding
                {
                    let method_names: Vec<String> = methods
                        .iter()
                        .filter_map(|m| {
                            let mb = m.borrow();
                            if let Symbol::ClassMethod { name, .. } = &*mb {
                                Some(name.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    let property_names: Vec<String> = properties
                        .iter()
                        .filter_map(|p| {
                            let pb = p.borrow();
                            if let Symbol::ClassProperty { name, .. } = &*pb {
                                Some(name.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    (method_names, property_names, *is_abstract)
                } else {
                    (vec![], vec![], false)
                }
            })
            .unwrap_or_default();

        for member in body {
            let member_ref = member.borrow();
            match &*member_ref {
                Statement::FunctionDecl {
                    name: method_name,
                    modifiers,
                    ..
                } => {
                    let is_override = modifiers.iter().any(|m| m.ty == ModifierType::Override);
                    let exists_in_super = super_method_names.contains(&method_name.value);

                    if exists_in_super {
                        if let Some(ref super_sym) = superclass_symbol {
                            let binding = super_sym.borrow();
                            if let Symbol::Class { methods, .. } = &*binding {
                                if let Some(super_method) = methods.iter().find(|m| {
                                    if let Ok(n) = m.borrow().name() {
                                        n == method_name.value
                                    } else {
                                        false
                                    }
                                }) {
                                    let smb = super_method.borrow();
                                    if let Symbol::ClassMethod { is_final, .. } = &*smb {
                                        if *is_final {
                                            let token = modifiers
                                                .iter()
                                                .find(|m| m.ty == ModifierType::Override)
                                                .map(|m| &*m.token)
                                                .unwrap_or(method_name);
                                            self.emit_error(
                                                TrussDiagnosticCode::CannotOverrideFinal,
                                                format!(
                                                    "Cannot override final method '{}'",
                                                    method_name.value
                                                ),
                                                token,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        if !is_override {
                            self.emit_error(
                                TrussDiagnosticCode::MissingOverrideModifier,
                                format!(
                                    "Method '{}' overrides a superclass method but is missing 'override' modifier",
                                    method_name.value
                                ),
                                method_name,
                            );
                        }
                    } else if is_override {
                        self.emit_error(
                            TrussDiagnosticCode::OverrideWithoutOverride,
                            format!(
                                "Method '{}' marked with 'override' but does not override anything",
                                method_name.value
                            ),
                            method_name,
                        );
                    }
                }
                Statement::VariableDecl {
                    name: var_name,
                    modifiers,
                    accessors,
                    ..
                } => {
                    let has_override = modifiers.iter().any(|m| m.ty == ModifierType::Override);
                    let is_computed = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
                    if is_computed {
                        let exists_in_super = super_property_names.contains(&var_name.value);

                        if exists_in_super {
                            if !has_override {
                                self.emit_error(
                                    TrussDiagnosticCode::MissingOverrideModifier,
                                    format!(
                                        "Property '{}' overrides a superclass property but is missing 'override' modifier",
                                        var_name.value
                                    ),
                                    var_name,
                                );
                            }
                        } else if has_override {
                            self.emit_error(
                                TrussDiagnosticCode::OverrideWithoutOverride,
                                format!(
                                    "Property '{}' marked with 'override' but does not override anything",
                                    var_name.value
                                ),
                                var_name,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn validate_member_access_levels(
        &self,
        container_access: Option<AccessModifier>,
        is_class: bool,
        body: &[Rc<RefCell<Statement>>],
    ) {
        let container_value = container_access
            .as_ref()
            .map(Self::access_level_value)
            .unwrap_or(3);

        for member in body {
            let member_ref = member.borrow();
            let Some(member_access) = Self::get_access_modifier(&member_ref) else {
                continue;
            };

            if member_access == AccessModifier::Open && !is_class {
                let token = member_ref.token();
                self.emit_error(
                    TrussDiagnosticCode::OpenOnlyOnClass,
                    "'open' is only allowed on class declarations and class members",
                    &token,
                );
                continue;
            }

            let member_value = Self::access_level_value(&member_access);
            if member_value > container_value {
                if let Some(container_access) = &container_access {
                    let token = member_ref.token();
                    self.emit_error(
                        TrussDiagnosticCode::InvalidMemberAccessLevel,
                        format!(
                            "Member's access level '{:?}' cannot be higher than container's access level '{:?}'",
                            member_access, container_access
                        ),
                        &token,
                    );
                }
            }
        }
    }

    fn validate_setter_access_conflicts(&self, body: &[Rc<RefCell<Statement>>]) {
        for member in body {
            let member_ref = member.borrow();
            let (decl_mod, inline_mod) = match &*member_ref {
                Statement::VariableDecl {
                    modifiers,
                    accessors,
                    ..
                }
                | Statement::SubscriptDecl {
                    modifiers,
                    accessors,
                    ..
                } => {
                    let decl_mod = modifiers.iter().find_map(|m| {
                        if let ModifierType::AccessSet(ref access) = m.ty {
                            Some(*access)
                        } else {
                            None
                        }
                    });
                    let inline_mod = accessors
                        .iter()
                        .find(|a| a.kind == AccessorKind::Set)
                        .and_then(|a| a.set_access_modifier);
                    (decl_mod, inline_mod)
                }
                _ => (None, None),
            };
            if let (Some(decl), Some(inline)) = (decl_mod, inline_mod) {
                if decl != inline {
                    let token = member_ref.token();
                    let msg = format!(
                        "Conflicting setter access: declaration specifies '{:?}', but accessor specifies '{:?}'",
                        decl, inline
                    );
                    self.emit_error(TrussDiagnosticCode::ConflictingSetterAccess, msg, &token);
                }
            }
        }
    }

    fn validate_where_clause(&mut self, where_clause: &[WhereRequirement]) {
        for req in where_clause {
            match &req.kind {
                WhereRequirementKind::Conformance {
                    type_expr,
                    constraint,
                } => {
                    self.infer_type(type_expr.clone());
                    let constraint_ty = self.infer_type(constraint.clone());
                    if let Some(ty) = constraint_ty {
                        if !matches!(&*ty.borrow(), Type::Protocol(..)) {
                            let token = constraint.borrow().token();
                            self.emit_error(
                                TrussDiagnosticCode::TypeError,
                                format!(
                                    "Constraint '{}' is not a protocol type",
                                    ty.borrow()
                                ),
                                &token,
                            );
                        }
                    }
                }
                WhereRequirementKind::Equality { left, right } => {
                    self.infer_type(left.clone());
                    self.infer_type(right.clone());
                }
            }
        }
    }

    fn check_protocol_conformances(
        &mut self,
        type_name: &str,
        type_token: &Token,
        conformances: &[Rc<RefCell<Expression>>],
        is_class: bool,
    ) {
        for conformance in conformances {
            let (protocol_name, protocol_type_params) = {
                let expr = conformance.borrow();
                match &*expr {
                    Expression::Type {
                        name,
                        type_parameters,
                        ty,
                    } => {
                        if let Some(ty) = ty {
                            if matches!(&*ty.borrow(), Type::Protocol(..)) {
                                (Some(name.value.clone()), type_parameters.clone())
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        }
                    }
                    _ => (None, None),
                }
            };
            let Some(ref protocol_name) = protocol_name else {
                continue;
            };

            let Some(protocol_symbol) = self
                .current_scope
                .as_ref()
                .and_then(|scope| scope.borrow().get_symbol(protocol_name))
            else {
                continue;
            };

            let protocol_generic_params: Vec<String> = protocol_symbol
                .borrow()
                .get_decl()
                .ok()
                .flatten()
                .map(|d| {
                    let stmt = d.borrow();
                    match &*stmt {
                        Statement::ProtocolDecl {
                            generic_parameters, ..
                        } => generic_parameters
                            .iter()
                            .filter_map(|gp| match &gp.kind {
                                GenericParameterKind::Type { .. } => {
                                    Some(gp.name.value.clone())
                                }
                                _ => None,
                            })
                            .collect(),
                        _ => vec![],
                    }
                })
                .unwrap_or_default();

            let inferred_params = if protocol_type_params.is_some() {
                protocol_type_params.clone()
            } else if !protocol_generic_params.is_empty() {
                self.infer_protocol_generic_params(
                    protocol_name,
                    &protocol_symbol,
                    &protocol_generic_params,
                    type_name,
                )
            } else {
                None
            };

            // Set the inferred type params on the conformance expression
            let protocol_name_owned = protocol_name.clone();
            if let Some(params) = inferred_params.as_ref() {
                let mut expr = conformance.borrow_mut();
                if let Expression::Type {
                    ty,
                    type_parameters,
                    ..
                } = &mut *expr
                {
                    *type_parameters = Some(params.clone());
                    *ty = Some(Rc::new(RefCell::new(Type::Protocol(
                        protocol_name_owned,
                        WeakSymbol(Rc::downgrade(&protocol_symbol)),
                        params.iter().filter_map(|p| {
                            let pe = p.borrow();
                            match &*pe {
                                Expression::Type { ty, .. }
                                | Expression::Variable { ty, .. } => ty.clone(),
                                _ => None,
                            }
                        }).collect(),
                    ))));
                }
                drop(expr);
            }

            let protocol_name = protocol_name.clone();
            let required_methods: Vec<String> = {
                let sym = protocol_symbol.borrow();
                let Symbol::Protocol { methods, .. } = &*sym else {
                    continue;
                };
                methods
                    .iter()
                    .filter(|m| {
                        m.borrow()
                            .get_decl()
                            .ok()
                            .flatten()
                            .map(|decl| {
                                let d = decl.borrow();
                                match &*d {
                                    Statement::FunctionDecl { body, .. } => {
                                        matches!(&*body.borrow(), FunctionBody::None)
                                    }
                                    _ => true,
                                }
                            })
                            .unwrap_or(true)
                    })
                    .filter_map(|m| m.borrow().name().ok())
                    .collect()
            };

            let required_properties: Vec<String> = {
                let sym = protocol_symbol.borrow();
                let Symbol::Protocol { properties, .. } = &*sym else {
                    continue;
                };
                properties
                    .iter()
                    .filter_map(|p| p.borrow().name().ok())
                    .collect()
            };

            let Some(type_symbol) = self
                .current_scope
                .as_ref()
                .and_then(|scope| scope.borrow().get_symbol(type_name))
            else {
                continue;
            };

            let (type_methods, type_properties): (Vec<String>, Vec<String>) = {
                let type_sym = type_symbol.borrow();
                match &*type_sym {
                    Symbol::Struct {
                        methods,
                        properties,
                        ..
                    }
                    | Symbol::Class {
                        methods,
                        properties,
                        ..
                    } => (
                        methods
                            .iter()
                            .filter_map(|m| m.borrow().name().ok())
                            .collect(),
                        properties
                            .iter()
                            .filter_map(|f| f.borrow().name().ok())
                            .collect(),
                    ),
                    _ => (vec![], vec![]),
                }
            };

            for req_method in &required_methods {
                if !type_methods.contains(req_method) {
                    let is_autowired = {
                        let sym = protocol_symbol.borrow();
                        match &*sym {
                            Symbol::Protocol { methods, .. } => methods.iter().any(|m| {
                                let mb = m.borrow();
                                matches!(&*mb, Symbol::ProtocolMethod { name, is_autowired: true, .. } if name == req_method)
                            }),
                            _ => false,
                        }
                    };
                    if is_autowired {
                        if is_class {
                            self.emit_error(
                                TrussDiagnosticCode::TypeError,
                                format!(
                                    "Class '{}' cannot conform to protocol '{}' because it has compiler-provided requirements",
                                    type_name, protocol_name
                                ),
                                type_token,
                            );
                        } else if protocol_name != "Copyable" || req_method != "copy" {
                            self.emit_error(
                                TrussDiagnosticCode::TypeError,
                                format!(
                                    "Compiler does not support autowired requirement 'func {}()' in protocol '{}'",
                                    req_method, protocol_name
                                ),
                                type_token,
                            );
                        }
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::ProtocolRequirementNotImplemented,
                            format!(
                                "Type '{}' does not implement protocol '{}' requirement: 'func {}()'",
                                type_name, protocol_name, req_method
                            ),
                            type_token,
                        );
                    }
                }
            }

            for req_prop in &required_properties {
                if !type_properties.contains(req_prop) {
                    self.emit_error(
                        TrussDiagnosticCode::ProtocolRequirementNotImplemented,
                        format!(
                            "Type '{}' does not implement protocol '{}' requirement: 'var {}'",
                            type_name, protocol_name, req_prop
                        ),
                        type_token,
                    );
                }
            }
        }
    }

    fn infer_protocol_generic_params(
        &mut self,
        _protocol_name: &str,
        protocol_symbol: &Rc<RefCell<Symbol>>,
        generic_param_names: &[String],
        type_name: &str,
    ) -> Option<Vec<Rc<RefCell<Expression>>>> {
        let sym = protocol_symbol.borrow();
        let Symbol::Protocol { methods, .. } = &*sym else {
            return None;
        };

        let mut mapping: HashMap<String, Rc<RefCell<Type>>> = HashMap::new();

        for method in methods {
            let Ok(method_name) = method.borrow().name() else {
                continue;
            };
            let Some(decl) = method.borrow().get_decl().ok().flatten() else {
                continue;
            };
            let decl_ref = decl.borrow();
            let Statement::FunctionDecl {
                ty: proto_fn_ty, ..
            } = &*decl_ref
            else {
                continue;
            };
            let Some(proto_fn_ty) = proto_fn_ty else {
                continue;
            };
            let (proto_param_tys, proto_ret_ty) = {
                let ty = proto_fn_ty.borrow();
                let Type::Function(params, ret, _, None) = &*ty else {
                    continue;
                };
                (params.clone(), ret.clone())
            };
            drop(decl_ref);

            let Some(type_sym) = self
                .current_scope
                .as_ref()
                .and_then(|scope| scope.borrow().get_symbol(type_name))
            else {
                continue;
            };
            let type_methods = match &*type_sym.borrow() {
                Symbol::Struct { methods, .. }
                | Symbol::Class { methods, .. } => methods.clone(),
                _ => continue,
            };
            drop(type_sym);

            let type_method = type_methods.iter().find(|m| {
                m.borrow().name().as_ref().ok() == Some(&method_name)
            })?;

            let Some(type_decl) = type_method.borrow().get_decl().ok().flatten() else {
                continue;
            };
            let type_decl_ref = type_decl.borrow();
            let Statement::FunctionDecl {
                ty: type_fn_ty, ..
            } = &*type_decl_ref
            else {
                continue;
            };
            let Some(type_fn_ty) = type_fn_ty else {
                continue;
            };
            let (type_param_tys, type_ret_ty) = {
                let ty = type_fn_ty.borrow();
                let Type::Function(params, ret, _, None) = &*ty else {
                    continue;
                };
                (params.clone(), ret.clone())
            };
            drop(type_decl_ref);

            for (proto_ty, type_ty) in proto_param_tys.iter().zip(type_param_tys.iter()) {
                Self::collect_generic_mappings(proto_ty.clone(), type_ty.clone(), &mut mapping);
            }
            Self::collect_generic_mappings(proto_ret_ty.clone(), type_ret_ty.clone(), &mut mapping);
        }

        // Check that all generic params have been inferred
        for name in generic_param_names {
            if !mapping.contains_key(name) {
                return None;
            }
        }

        let params: Vec<Rc<RefCell<Expression>>> = generic_param_names
            .iter()
            .filter_map(|name| {
                let ty = mapping.get(name)?;
                let token = Token::new(
                    name.clone(),
                    TokenType::Identifier,
                    Position { pos: 0, line: 0, col: 0, len: 0 },
                    Rc::new("".to_string()),
                );
                let expr = Expression::Type {
                    name: Box::new(token),
                    type_parameters: None,
                    ty: Some(ty.clone()),
                };
                Some(Rc::new(RefCell::new(expr)))
            })
            .collect();

        if params.is_empty() { None } else { Some(params) }
    }

    fn find_module_from_expr(&self, expr: &Expression) -> Option<Rc<RefCell<Module>>> {
        match expr {
            Expression::Variable { symbol, .. } => {
                let ws = symbol.as_ref()?;
                let sym = ws.0.upgrade()?;
                let binding = sym.borrow();
                if let Symbol::Module { module, .. } = &*binding {
                    module.clone()
                } else {
                    None
                }
            }
            Expression::MemberAccess { object, member, .. } => {
                let obj = object.borrow();
                let module = self.find_module_from_expr(&obj)?;
                let scope = module.borrow().scope.clone()?;
                let sym = scope.borrow().get_symbol(&member.value)?;
                let binding = sym.borrow();
                if let Symbol::Module { module, .. } = &*binding {
                    module.clone()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn try_module_member_access(
        &mut self,
        object: Rc<RefCell<Expression>>,
        member: &Token,
        ty: &mut Option<Rc<RefCell<Type>>>,
    ) -> Option<Rc<RefCell<Type>>> {
        let module = self.find_module_from_expr(&object.borrow())?;
        let scope = module.borrow().scope.clone()?;
        let sym = scope.borrow().get_symbol(&member.value)?;
        let binding = sym.borrow();

        if let Symbol::Module { .. } = &*binding {
            return None;
        }

        let decl = binding.get_decl().ok()??;
        drop(binding);
        drop(scope);

        let member_type = {
            let decl_ref = decl.borrow();
            match &*decl_ref {
                Statement::FunctionDecl { ty: fn_ty, .. } => fn_ty.clone(),
                Statement::VariableDecl { ty: var_ty, .. } => var_ty.clone(),
                _ => None,
            }
        };

        if let Some(t) = member_type {
            *ty = Some(t.clone());
            Some(t)
        } else {
            None
        }
    }

    fn check_writable(&mut self, expr: Rc<RefCell<Expression>>) {
        let token = expr.borrow().token();
        match &*expr.borrow() {
            Expression::Variable { symbol, name, .. } => {
                if let Some(ws) = symbol {
                    let sym = ws.0.upgrade();
                    if let Some(sym) = sym {
                        let binding = sym.borrow();
                        if let Symbol::Variable { is_var, .. } = &*binding {
                            if *is_var {
                                return;
                            }
                            if let Some(decl) = binding.get_decl().ok().flatten() {
                                let has_initializer = {
                                    let decl_ref = decl.borrow();
                                    matches!(
                                        &*decl_ref,
                                        Statement::VariableDecl {
                                            initializer: Some(_),
                                            ..
                                        }
                                    )
                                };
                                if has_initializer {
                                    self.emit_error(
                                        TrussDiagnosticCode::AssignToImmutable,
                                        format!("Cannot assign to 'let' variable '{}'", name.value),
                                        &token,
                                    );
                                    return;
                                }
                                if self
                                    .initialized_lets
                                    .last()
                                    .map_or(false, |s| s.contains(&name.value))
                                {
                                    self.emit_error(
                                        TrussDiagnosticCode::AssignToImmutable,
                                        format!(
                                            "Cannot assign to 'let' variable '{}' again",
                                            name.value
                                        ),
                                        &token,
                                    );
                                    return;
                                }
                                if let Some(scope) = &self.current_scope {
                                    scope.borrow().get_symbol(&name.value);
                                }
                                if let Some(set) = self.initialized_lets.last_mut() {
                                    set.insert(name.value.clone());
                                }
                            }
                        }
                    }
                }
            }
            Expression::MemberAccess { object, member, .. } => {
                let object_ty = self.infer_type(object.clone());
                if let Some(object_ty) = object_ty {
                    let type_name = match &*object_ty.borrow() {
                        Type::Struct(n, ..) | Type::Class(n, ..) => Some(n.clone()),
                        _ => None,
                    };
                    if let Some(type_name) = type_name {
                        if let Some(scope) = &self.current_scope {
                            let scope_ref = scope.borrow();
                            if let Some(symbol) = scope_ref.get_symbol(&type_name) {
                                let binding = symbol.borrow();
                                let properties = match &*binding {
                                    Symbol::Struct { properties, .. }
                                    | Symbol::Class { properties, .. } => properties.clone(),
                                    _ => vec![],
                                };
                                drop(binding);
                                for prop in &properties {
                                    let prop_binding = prop.borrow();
                                    if let Symbol::StructProperty { name, is_var, .. }
                                    | Symbol::ClassProperty { name, is_var, .. } = &*prop_binding
                                    {
                                        if name == &member.value {
                                            if *is_var {
                                                let prop_clone = prop.clone();
                                                let set_access = {
                                                    let d = prop_clone.borrow();
                                                    d.get_decl().ok().flatten().and_then(|decl| {
                                                        let decl_ref = decl.borrow();
                                                        if let Statement::VariableDecl {
                                                            accessors,
                                                            ..
                                                        } = &*decl_ref
                                                        {
                                                            if let Some(set_acc) =
                                                                accessors.iter().find(|a| {
                                                                    a.kind == AccessorKind::Set
                                                                })
                                                            {
                                                                if set_acc
                                                                    .set_access_modifier
                                                                    .is_some()
                                                                {
                                                                    return set_acc
                                                                        .set_access_modifier;
                                                                }
                                                            }
                                                        }
                                                        drop(decl_ref);
                                                        Self::get_set_access_modifier(
                                                            &*decl.borrow(),
                                                        )
                                                    })
                                                };
                                                if let Some(ref set_access) = set_access {
                                                    if !self.is_setter_accessible(
                                                        set_access, prop_clone, &token,
                                                    ) {
                                                        self.emit_error(
                                                            TrussDiagnosticCode::InvalidMemberAccessLevel,
                                                            format!("Cannot assign to property '{}' due to setter access level", name),
                                                            &token,
                                                        );
                                                        return;
                                                    }
                                                }
                                                return;
                                            }
                                            if self.is_in_init {
                                                if self.initialized_properties.contains(name) {
                                                    self.emit_error(
                                                        TrussDiagnosticCode::AssignToImmutable,
                                                        format!("Cannot assign to 'let' property '{}' again", name),
                                                        &token,
                                                    );
                                                } else {
                                                    self.initialized_properties
                                                        .insert(name.clone());
                                                }
                                            } else {
                                                self.emit_error(
                                                    TrussDiagnosticCode::AssignToImmutable,
                                                    format!(
                                                        "Cannot assign to 'let' property '{}'",
                                                        name
                                                    ),
                                                    &token,
                                                );
                                            }
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Expression::SubscriptAccess { object, .. } => {
                let object_ty = self.infer_type(object.clone());
                if let Some(object_ty) = object_ty {
                    let type_name = match &*object_ty.borrow() {
                        Type::Struct(n, ..) | Type::Class(n, ..) => Some(n.clone()),
                        _ => None,
                    };
                    if let Some(type_name) = type_name {
                        if let Some(scope) = &self.current_scope {
                            let scope_ref = scope.borrow();
                            if let Some(symbol) = scope_ref.get_symbol(&type_name) {
                                let binding = symbol.borrow();
                                let subscripts = match &*binding {
                                    Symbol::Struct { subscripts, .. }
                                    | Symbol::Class { subscripts, .. } => subscripts.clone(),
                                    _ => vec![],
                                };
                                drop(binding);
                                drop(scope_ref);
                                for sub in &subscripts {
                                    let sub_binding = sub.borrow();
                                    if let Some(decl) = sub_binding.get_decl().ok().flatten() {
                                        let decl_ref = decl.borrow();
                                        if let Statement::SubscriptDecl { accessors, .. } =
                                            &*decl_ref
                                        {
                                            let inline_mod = accessors
                                                .iter()
                                                .find(|a| a.kind == AccessorKind::Set)
                                                .and_then(|a| a.set_access_modifier);
                                            let set_mod = inline_mod.or_else(|| {
                                                Self::get_set_access_modifier(&*decl_ref)
                                            });
                                            if let Some(ref set_mod) = set_mod {
                                                if !self.is_setter_accessible(
                                                    set_mod,
                                                    sub.clone(),
                                                    &token,
                                                ) {
                                                    self.emit_error(
                                                        TrussDiagnosticCode::InvalidMemberAccessLevel,
                                                        "Cannot assign to subscript due to setter access level",
                                                        &token,
                                                    );
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                    drop(sub_binding);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>, token: &Token) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg));
        self.engine.borrow_mut().emit(diag);
    }

    fn emit_error_with_labels(
        &self,
        code: TrussDiagnosticCode,
        message: impl Into<String>,
        primary: duck_diagnostic::Label,
        secondary: duck_diagnostic::Label,
    ) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg)
            .with_label(primary)
            .with_label(secondary);
        self.engine.borrow_mut().emit(diag);
    }
}
