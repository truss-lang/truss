use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        expression::{BinaryOperator, CallParameter, CastKind, Expression, UnaryOperator},
        node::Program,
        statement::{AccessModifier, AccessorKind, FunctionBody, ModifierType, Pattern, ProtocolMember, Statement, VariadicKind},
    },
    diag::{
        TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token,
        secondary_label_from_token,
    },
    krate::{Crate, Module},
    lexer::token::{Token, TokenType},
    scope::Scope,
    symbol::{Symbol, WeakSymbol},
    types::Type,
};

type MethodInfo = Option<(String, Rc<RefCell<Type>>, Vec<Rc<RefCell<Type>>>)>;

#[derive(Debug)]
pub struct TypeResolver {
    pub krate: Rc<RefCell<Crate>>,
    current_module: Option<Rc<RefCell<Module>>>,
    current_return_type: Option<Rc<RefCell<Type>>>,
    current_scope: Option<Rc<RefCell<Scope>>>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
}

impl TypeResolver {
    pub fn new(krate: Rc<RefCell<Crate>>, engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        Self {
            krate,
            current_module: None,
            current_return_type: None,
            current_scope: None,
            engine,
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
        match &mut *statement.borrow_mut() {
            Statement::FunctionDecl {
                name,
                parameters,
                return_type,
                body,
                scope,
                ty,
                ..
            } => {
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
                    }
                    if param.borrow().variadic_kind != VariadicKind::NotVariadic {
                        is_vararg = true;
                    }
                }

                let fn_type = Rc::new(RefCell::new(Type::Function(
                    parameter_types,
                    ret_type,
                    is_vararg,
                )));
                *ty = Some(fn_type.clone());

                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), fn_type);

                self.enter_scope(scope.as_ref().unwrap().clone());

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
                name, body, scope, ..
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
                )));
                let self_ty = struct_ty.clone();
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), struct_ty);

                self.enter_scope(scope.as_ref().unwrap().clone());
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type("self".to_string(), self_ty);
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
            }
            Statement::ClassDecl {
                name, body, scope, superclass, conformances, ..
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
                )));
                let self_ty = class_ty.clone();
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), class_ty);

                if let Some(superclass_expr) = superclass {
                    self.infer_type(superclass_expr.clone());
                }
                for conformance in conformances {
                    self.infer_type(conformance.clone());
                }

                self.enter_scope(scope.as_ref().unwrap().clone());
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type("self".to_string(), self_ty);
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
            }
            Statement::EnumDecl {
                name, cases: ast_cases, body, scope, ..
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
                )));
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), enum_ty);

                if let Symbol::Enum { cases, .. } = &mut *symbol.borrow_mut() {
                    for (case_symbol, ast_case) in cases.iter().zip(ast_cases.iter()) {
                        let mut parameter_types = Vec::new();
                        for param in &ast_case.parameters {
                            let param_type = self.infer_type(param.type_expression.clone());
                            if let Some(ref param_type) = param_type {
                                parameter_types.push(param_type.clone());
                            }
                        }
                        if let Symbol::EnumCase { parameter_types: param_tys, .. } = &mut *case_symbol.borrow_mut() {
                            *param_tys = parameter_types;
                        }
                    }
                }

                self.enter_scope(scope.as_ref().unwrap().clone());
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
            }
            Statement::InitDecl {
                parameters,
                body,
                scope,
                ty,
                ..
            } => {
                let ret_type = Rc::new(RefCell::new(Type::Void));
                let mut parameter_types = Vec::new();
                for param in parameters.iter() {
                    let param_type = self.infer_type(param.borrow().type_expression.clone());
                    if let Some(ref param_type) = param_type {
                        param.borrow_mut().ty = Some(param_type.clone());
                        parameter_types.push(param_type.clone());
                    }
                }
                let fn_type = Rc::new(RefCell::new(Type::Function(
                    parameter_types,
                    ret_type,
                    false,
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
                )));
                self.current_scope
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .set_type(name.value.clone(), protocol_ty);

                for conformance in conformances {
                    self.infer_type(conformance.clone());
                }

                self.enter_scope(scope.as_ref().unwrap().clone());
                for member in members {
                    match member {
                        ProtocolMember::Method { decl, .. } => {
                            if let Statement::FunctionDecl {
                                name: _method_name,
                                parameters,
                                return_type,
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

                                let fn_type = Rc::new(RefCell::new(Type::Function(
                                    parameter_types,
                                    ret_type,
                                    is_vararg,
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
                    }
                }
                self.leave_scope();
            }
            _ => {}
        }
    }

    fn process_function_decl_in_expr(&mut self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Block { statements, .. } = &*expr.borrow() {
            for stmt in statements {
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
                            AccessorKind::Set
                            | AccessorKind::WillSet
                            | AccessorKind::DidSet => {
                                self.current_return_type =
                                    Some(Rc::new(RefCell::new(Type::Void)));
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
                    )))
                };

                let ret_type = if let Type::Function(_, ret, _) = &*fn_type.borrow() {
                    ret.clone()
                } else {
                    Rc::new(RefCell::new(Type::Void))
                };
                self.current_return_type = Some(ret_type.clone());
                self.enter_scope(scope.as_ref().unwrap().clone());

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

                self.leave_scope();
                self.current_return_type = last_return_type;
            }
            Statement::InitDecl {
                parameters,
                body,
                scope,
                ..
            } => {
                self.enter_scope(scope.as_ref().unwrap().clone());

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
                self.resolve_block_expression(body.clone());
            }
            Statement::Loop { body, .. } => {
                self.resolve_block_expression(body.clone());
            }
            Statement::RepeatWhile {
                body, condition, ..
            } => {
                self.resolve_block_expression(body.clone());
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
                self.resolve_block_expression(body.clone());
            }
            Statement::ExternBlock { items, .. } => {
                for item in items {
                    self.resolve_statement(item.clone());
                }
            }
            Statement::ExternDecl { statement, .. } => {
                self.resolve_statement(statement.clone());
            }
            Statement::StructDecl { body, .. } => {
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
            }
            Statement::ClassDecl { body, conformances, .. } => {
                for conformance in conformances {
                    self.infer_type(conformance.clone());
                }
                for stmt in body {
                    self.resolve_statement(stmt.clone());
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
                                    )))
                                };
                                let ret_type = if let Type::Function(_, ret, _) = &*fn_type.borrow() {
                                    ret.clone()
                                } else {
                                    Rc::new(RefCell::new(Type::Void))
                                };
                                self.current_return_type = Some(ret_type.clone());
                                self.enter_scope(fn_scope.as_ref().unwrap().clone());
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
                                self.leave_scope();
                                self.current_return_type = last_return_type;
                            }
                        }
                        ProtocolMember::Property { .. } => {}
                    }
                }
                self.leave_scope();
            }
            Statement::EnumDecl { body, .. } => {
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
            }
            _ => {}
        }
    }

    fn resolve_block_expression(&mut self, block_expr: Rc<RefCell<Expression>>) {
        if let Expression::Block { statements, .. } = &*block_expr.borrow() {
            for stmt in statements {
                self.resolve_statement(stmt.clone());
            }
        }
    }

    fn resolve_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) {
        match &mut *body.borrow_mut() {
            FunctionBody::Statements(statements) => {
                for stmt in statements {
                    self.resolve_statement(stmt.clone());
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

    fn infer_type(&mut self, expression: Rc<RefCell<Expression>>) -> Option<Rc<RefCell<Type>>> {
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
                let t = t?;
                *ty = Some(t.clone());
                t
            }
            Expression::Type { name, ty, .. } => {
                let t = self.resolve_type_name(&name.value, name.as_ref())?;
                *ty = Some(t.clone());
                t
            }
            Expression::Block { statements, .. } => {
                let mut last_ty = Rc::new(RefCell::new(Type::Void));
                for stmt in statements.iter() {
                    if let Some(ty) = self.infer_statement_type(stmt.clone()) {
                        last_ty = ty;
                    }
                }
                last_ty
            }
            Expression::Binary {
                left,
                operator,
                right,
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
                ..
            } => {
                let operand_ty = self.infer_type(expression.clone())?;
                if let Some(result) = self.check_unary(*operator, operand_ty.clone()) {
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
                callee, parameters, ..
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
                match &*callee_type.borrow() {
                    Type::Function(param_tys, ret_ty, is_vararg) => {
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

                        let func_decl = self.get_function_decl_from_callee(callee.clone());

                        for (i, param) in parameters.iter().enumerate() {
                            if i < param_tys.len() {
                                let expected_ty = param_tys[i].clone();
                                self.infer_expression_type(param.expression.clone(), expected_ty);

                                if let Some(ref decl) = func_decl {
                                    self.check_parameter_label(param, decl, i);
                                }
                            }
                        }
                        ret_ty.clone()
                    }
                    Type::Struct(struct_name, _) => {
                        let init_params_info = {
                            let scope = self.current_scope.as_ref().unwrap().borrow();
                            if let Some(symbol) = scope.get_symbol(struct_name)
                                && let Symbol::Struct { constructors, .. } = &*symbol.borrow()
                            {
                                constructors.iter().find_map(|constructor| {
                                    if let Ok(Some(decl)) = constructor.borrow().get_decl()
                                        && let Statement::InitDecl {
                                            ty: Some(init_ty), ..
                                        } = &*decl.borrow()
                                        && let Type::Function(param_tys, _, is_vararg) =
                                            &*init_ty.borrow()
                                    {
                                        Some((decl.clone(), param_tys.clone(), *is_vararg))
                                    } else {
                                        None
                                    }
                                })
                            } else {
                                None
                            }
                        };
                        if let Some((decl, param_tys, is_vararg)) = init_params_info {
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
                        callee_type.clone()
                    }
                    Type::Class(class_name, _) => {
                        let init_params_info = {
                            let scope = self.current_scope.as_ref().unwrap().borrow();
                            if let Some(symbol) = scope.get_symbol(class_name) {
                                let constructors = match &*symbol.borrow() {
                                    Symbol::Struct { constructors, .. }
                                    | Symbol::Class { constructors, .. } => constructors.clone(),
                                    _ => return Some(callee_type.clone()),
                                };
                                constructors.iter().find_map(|constructor| {
                                    if let Ok(Some(decl)) = constructor.borrow().get_decl()
                                        && let Statement::InitDecl {
                                            ty: Some(init_ty), ..
                                        } = &*decl.borrow()
                                        && let Type::Function(param_tys, _, is_vararg) =
                                            &*init_ty.borrow()
                                    {
                                        Some((decl.clone(), param_tys.clone(), *is_vararg))
                                    } else {
                                        None
                                    }
                                })
                            } else {
                                None
                            }
                        };
                        if let Some((decl, param_tys, is_vararg)) = init_params_info {
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
                        callee_type.clone()
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
            Expression::If {
                condition,
                then,
                else_,
                ..
            } => {
                let cond_ty = self.infer_type(condition.clone())?;

                let binding_types = {
                    let cond = condition.borrow();
                    if let Expression::Case { enum_type, case_name, bindings, .. } = &*cond {
                        if !bindings.is_empty() {
                            self.get_enum_case_parameter_types(
                                &enum_type.value,
                                &case_name.value,
                            )
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(ref param_types) = binding_types {
                    if let Expression::Block { scope, .. } = &mut *then.borrow_mut() {
                        if let Some(block_scope) = scope {
                            if let Expression::Case { bindings, .. } = &*condition.borrow() {
                                Self::set_binding_types(bindings, param_types, block_scope);
                            }
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

                let then_ty = self.infer_type(then.clone())?;
                if let Some(else_expr) = else_ {
                    let else_ty = self.infer_type(else_expr.clone())?;
                    if then_ty.borrow().clone() != else_ty.borrow().clone() {
                        self.emit_error(
                            TrussDiagnosticCode::BranchTypeMismatch,
                            format!(
                                "If branches have different types: {} vs {}",
                                then_ty.borrow(),
                                else_ty.borrow()
                            ),
                            &then.borrow().token(),
                        );
                    }
                }
                then_ty
            }
            Expression::Case {
                enum_type,
                case_name,
                expression,
                ty,
                ..
            } => {
                let _expr_ty = self.infer_type(expression.clone());

                if let Some(current_scope) = &self.current_scope {
                    let scope = current_scope.borrow();
                    if let Some(symbol) = scope.get_symbol(&enum_type.value) {
                        if let Symbol::Enum { cases, .. } = &*symbol.borrow() {
                            let found = cases.iter().any(|c| {
                                c.borrow().name().as_ref().ok() == Some(&case_name.value)
                            });
                            if !found {
                                self.emit_error(
                                    TrussDiagnosticCode::FieldNotFound,
                                    format!("Enum '{}' has no case '{}'", enum_type.value, case_name.value),
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
                }

                let case_ty = Rc::new(RefCell::new(Type::Bool));
                *ty = Some(case_ty.clone());
                case_ty
            }
            Expression::VoidLiteral { .. } => Rc::new(RefCell::new(Type::Void)),
            Expression::NullLiteral { .. } => Rc::new(RefCell::new(Type::Void)),
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
            Expression::PointerType { base, ty } => {
                if let Some(existing_ty) = ty.as_ref() {
                    return Some(existing_ty.clone());
                }
                let base_ty = self.infer_type(*base.clone())?;
                let pointer_ty = Rc::new(RefCell::new(Type::Pointer(base_ty)));
                *ty = Some(pointer_ty.clone());
                pointer_ty
            }
            Expression::AnyType { inner, ty } => {
                let inner_ty = self.infer_type(inner.clone())?;
                if !matches!(&*inner_ty.borrow(), Type::Protocol(..)) {
                    let token = inner.borrow().token();
                    self.emit_error(
                        TrussDiagnosticCode::TypeError,
                        format!("'any' must be used with a protocol type, but '{}' is not a protocol", inner_ty.borrow()),
                        &token,
                    );
                    return None;
                }
                *ty = Some(inner_ty.clone());
                inner_ty
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
                let object_ty = self.infer_type(object.clone())?;
                match &*object_ty.borrow() {
                    Type::Struct(struct_name, _) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        if let Some(symbol) = scope.get_symbol(struct_name)
                            && let Symbol::Struct {
                                fields, methods, ..
                            } = &*symbol.borrow()
                        {
                            if !self.is_member_accessible(&symbol.borrow(), member) {
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
                            for field in fields {
                                if field.borrow().name().as_ref().ok() == Some(&member.value)
                                    && let Some(decl) = field.borrow().get_decl().ok().flatten()
                                    && let Statement::VariableDecl { ty: field_ty, .. } =
                                        &*decl.borrow()
                                    && let Some(t) = field_ty
                                {
                                    *ty = Some(t.clone());
                                    return Some(t.clone());
                                }
                            }
                            for method in methods {
                                if method.borrow().name().as_ref().ok() == Some(&member.value)
                                    && let Some(decl) = method.borrow().get_decl().ok().flatten()
                                {
                                    let method_ty = {
                                        let decl_ref = decl.borrow();
                                        if let Statement::FunctionDecl { ty, .. } = &*decl_ref {
                                            ty.clone()
                                        } else if let Statement::InitDecl { ty, .. } = &*decl_ref {
                                            ty.clone()
                                        } else if let Statement::DeinitDecl { ty, .. } = &*decl_ref
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
                        } else {
                            let token = &*member;
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!("Struct symbol '{}' not found", struct_name),
                                token,
                            );
                            return None;
                        }
                    }
                    Type::Class(class_name, _) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        if let Some(symbol) = scope.get_symbol(class_name) {
                            let binding = symbol.borrow();
                            let (fields, methods) = match &*binding {
                                Symbol::Struct { fields, methods, .. }
                                | Symbol::Class { fields, methods, .. } => (fields, methods),
                                _ => {
                                    self.emit_error(
                                        TrussDiagnosticCode::FieldNotFound,
                                        format!("Class symbol '{}' has unexpected type", class_name),
                                        member,
                                    );
                                    return None;
                                }
                            };
                            if !self.is_member_accessible(&symbol.borrow(), member) {
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
                            for field in fields {
                                if field.borrow().name().as_ref().ok() == Some(&member.value)
                                    && let Some(decl) = field.borrow().get_decl().ok().flatten()
                                    && let Statement::VariableDecl { ty: field_ty, .. } =
                                        &*decl.borrow()
                                    && let Some(t) = field_ty
                                {
                                    *ty = Some(t.clone());
                                    return Some(t.clone());
                                }
                            }
                            for method in methods {
                                if method.borrow().name().as_ref().ok() == Some(&member.value)
                                    && let Some(decl) = method.borrow().get_decl().ok().flatten()
                                {
                                    let method_ty = {
                                        let decl_ref = decl.borrow();
                                        if let Statement::FunctionDecl { ty, .. } = &*decl_ref {
                                            ty.clone()
                                        } else if let Statement::InitDecl { ty, .. } = &*decl_ref {
                                            ty.clone()
                                        } else if let Statement::DeinitDecl { ty, .. } = &*decl_ref
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

                            let decl = symbol.borrow().get_decl().ok().flatten();
                            let super_info = decl.as_ref().and_then(|decl| {
                                if let Statement::ClassDecl { superclass: Some(super_expr), .. } = &*decl.borrow() {
                                    if let Expression::Type { name: super_name, .. } = &*super_expr.borrow() {
                                        return Some((super_name.value.clone(), super_name.position.clone(), super_name.file.clone()));
                                    }
                                }
                                None
                            });
                            drop(binding);
                            drop(scope);

                            if let Some((super_name, pos, file)) = super_info {
                                let super_object = Rc::new(RefCell::new(Expression::Variable {
                                    name: Box::new(Token::new(super_name, TokenType::Identifier, pos, file)),
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
                        } else {
                            let token = &*member;
                            self.emit_error(
                                TrussDiagnosticCode::FieldNotFound,
                                format!("Class symbol '{}' not found", class_name),
                                token,
                            );
                            return None;
                        }
                    }
                    Type::Enum(enum_name, _) => {
                        let scope = self.current_scope.as_ref().unwrap().borrow();
                        if let Some(symbol) = scope.get_symbol(enum_name)
                            && let Symbol::Enum { cases, .. } = &*symbol.borrow()
                        {
                            for case in cases {
                                if case.borrow().name().as_ref().ok() == Some(&member.value) {
                                    if let Symbol::EnumCase { parameter_types, .. } = &*case.borrow() {
                                        if parameter_types.is_empty() {
                                            *ty = Some(object_ty.clone());
                                            return Some(object_ty.clone());
                                        } else {
                                            let case_fn_type = Rc::new(RefCell::new(Type::Function(
                                                parameter_types.clone(),
                                                object_ty.clone(),
                                                false,
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
                        // Try to find by name in named tuple fields
                        if let Some((_, element_ty)) = elements.iter().enumerate().find_map(|(i, (n, t))| {
                            n.as_ref().and_then(|name| {
                                if name == member_name {
                                    Some((i, t.clone()))
                                } else {
                                    None
                                }
                            })
                        }) {
                            *ty = Some(element_ty.clone());
                            return Some(element_ty.clone());
                        }
                        // Also try numeric index via member name (e.g. "0", "1")
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
            Expression::TupleLiteral {
                elements, ty, ..
            } => {
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
                                other,
                                index_value
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

        if is_int_literal {
            if Self::is_integer_type(&expected.borrow()) {
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
            if Self::is_float_type(&expected.borrow()) {
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
            let mut expr_mut = expression.borrow_mut();
            if let Expression::NullptrLiteral { ty, .. } = &mut *expr_mut {
                *ty = Some(expected.clone());
            }
            drop(expr_mut);
        } else if let Some(inferred) = self.infer_type(expression) {
            if inferred.borrow().clone() != expected.borrow().clone() {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {}, found {}",
                        expected.borrow(),
                        inferred.borrow()
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
                } else {
                    None
                }
            }
            _ => None,
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
                let token = call_param.expression.borrow().token();
                self.emit_error(
                    TrussDiagnosticCode::MissingArgumentLabel,
                    format!("Missing argument label '{}' in call", expected.value),
                    &token,
                );
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

    fn is_member_accessible(&self, symbol: &Symbol, member_token: &Token) -> bool {
        let Some(access) = symbol.get_decl().ok().flatten().and_then(|decl| {
            decl.borrow()
                .modifiers()
                .unwrap()
                .iter()
                .find(|m| matches!(m.ty, ModifierType::Access(_)))
                .cloned()
        }) else {
            return true;
        };
        let ModifierType::Access(access) = access.ty;
        match access {
            AccessModifier::Open | AccessModifier::Public | AccessModifier::Internal => true,
            AccessModifier::Fileprivate => {
                if let Some(decl) = symbol.get_decl().ok().flatten() {
                    let decl_file = decl.borrow().token().file;
                    member_token.file == decl_file
                } else {
                    true
                }
            }
            AccessModifier::Private => {
                // self.is_within_owner_scope(symbol)
                // TODO: implement is_within_owner_scope
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
                    if let Symbol::EnumCase { parameter_types, .. } = &*case.borrow() {
                        return Some(parameter_types.clone());
                    }
                }
            }
        }
        None
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
