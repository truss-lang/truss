use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::{
        expression::{ClosureCapture, ElseBranch, Expression, UnaryOperator},
        node::Program,
        statement::{
            AccessorKind, Attribute, FunctionBody, GenericParameterKind, ImportKind, Modifier,
            ModifierType, OwnershipModifier, Pattern, ProtocolAccessorSet, ProtocolMember,
            SelectiveAlias, Statement, WhereRequirement, WhereRequirementKind,
        },
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    krate::{Module, Package},
    lexer::token::{KeywordType, Position, Token, TokenType},
    scope::Scope,
    symbol::{Symbol, WeakSymbol},
    types::Type,
};

#[derive(Debug)]
pub struct SymbolResolver {
    pub packages: HashMap<String, Rc<RefCell<Package>>>,
    current_package: String,
    current_module: Option<Rc<RefCell<Module>>>,
    current_scope: Option<Rc<RefCell<Scope>>>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
    root_module_name: String,
    closure_capture_stack: Vec<(Rc<RefCell<Scope>>, Vec<ClosureCapture>)>,
}
impl SymbolResolver {
    pub fn new(
        packages: HashMap<String, Rc<RefCell<Package>>>,
        current_package: String,
        engine: Rc<RefCell<TrussDiagnosticEngine>>,
    ) -> Self {
        Self {
            packages,
            current_package,
            current_module: None,
            current_scope: None,
            engine,
            root_module_name: String::new(),
            closure_capture_stack: Vec::new(),
        }
    }

    pub fn resolve(&mut self, program: &Program, module_name: String) -> Rc<RefCell<Module>> {
        self.root_module_name = module_name.clone();
        let module = Rc::new(RefCell::new(Module::new(module_name.clone())));
        let current_pkg = self.packages.get(&self.current_package).unwrap();
        current_pkg
            .borrow_mut()
            .modules
            .insert(module_name, module.clone());
        self.current_module = Some(module.clone());
        let scope = self.enter_scope(None);
        self.current_module.as_ref().unwrap().borrow_mut().scope = Some(scope.clone());

        let mut entries: Vec<(String, Rc<RefCell<Symbol>>)> = Vec::new();
        let mut type_entries: Vec<(String, Rc<RefCell<Type>>)> = Vec::new();
        {
            let truss_pkg = self.packages.get("Truss");
            let truss_module = truss_pkg.and_then(|p| p.borrow().modules.get("Truss").cloned());
            if let Some(m) = truss_module {
                if let Some(s) = m.borrow().scope.clone() {
                    let sb = s.borrow();
                    entries = sb
                        .name_table
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    type_entries = sb
                        .type_env
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                }
            }
        }
        let file: Rc<String> = Rc::new(
            self.packages
                .get(&self.current_package)
                .unwrap()
                .borrow()
                .name
                .clone(),
        );
        for (name, symbol) in entries {
            let name_token = Token::new(
                name,
                crate::lexer::token::TokenType::Identifier,
                crate::lexer::token::Position {
                    pos: 0,
                    line: 0,
                    col: 0,
                    len: 0,
                },
                file.clone(),
            );
            self.enter(symbol, &name_token);
        }
        for (name, ty) in type_entries {
            if let Some(current_scope) = self.current_scope.as_ref() {
                if !current_scope.borrow().type_env.contains_key(&name) {
                    current_scope.borrow_mut().type_env.insert(name, ty);
                }
            }
        }

        for stmt in &program.statements {
            self.register_symbols(stmt.clone());
        }

        self.validate_main_attribute(&program.statements);

        for stmt in &program.statements {
            self.resolve_statement(stmt.clone());
        }
        self.leave_scope();
        module
    }

    pub fn register_symbols(&mut self, stmt: Rc<RefCell<Statement>>) {
        match &mut *stmt.borrow_mut() {
            Statement::FunctionDecl {
                name,
                body,
                generic_parameters,
                scope: fn_scope,
                ..
            } => {
                let symbol = Rc::new(RefCell::new(Symbol::Function {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                }));
                self.enter(symbol, name);
                *fn_scope = Some(self.enter_scope(None));
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { .. } => {
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                Rc::new(RefCell::new(Type::Never)),
                            )))
                        }
                    };
                    fn_scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.resolve_expression(default_value.clone());
                    }
                }
                match &*body.borrow() {
                    FunctionBody::Statements(stmts) => {
                        for s in stmts {
                            self.register_symbols(s.clone());
                        }
                    }
                    FunctionBody::Expression(expr) => {
                        self.register_function_symbols_in_expr(expr.clone());
                    }
                    FunctionBody::None => {}
                }
                self.leave_scope();
            }
            Statement::StructDecl {
                name,
                body,
                scope,
                generic_parameters,
                attributes,
                ..
            } => {
                let is_builtin = attributes.iter().any(|a| a.name == "builtintype");
                let has_dml = attributes.iter().any(|a| a.name == "dynamicMemberLookup");
                let has_dcl = attributes.iter().any(|a| a.name == "dynamicCallable");
                let struct_symbol = Rc::new(RefCell::new(Symbol::Struct {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    is_builtin_type: is_builtin,
                    has_dynamic_member_lookup: has_dml,
                    has_dynamic_callable: has_dcl,
                    properties: vec![],
                    methods: vec![],
                    constructors: vec![],
                    destrcutor: None,
                    subscripts: vec![],
                }));
                self.enter(struct_symbol.clone(), name);
                if let Some(scope) = self.current_scope.as_ref() {
                    let struct_ty = Rc::new(RefCell::new(Type::Struct(
                        name.value.clone(),
                        WeakSymbol(Rc::downgrade(&struct_symbol)),
                        vec![],
                    )));
                    scope.borrow_mut().set_type(name.value.clone(), struct_ty);
                }
                let Symbol::Struct {
                    properties: fields,
                    methods,
                    constructors,
                    destrcutor,
                    subscripts: _subscripts,
                    ..
                } = &mut *struct_symbol.borrow_mut()
                else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                {
                    for gp in generic_parameters {
                        let gp_type = match &gp.kind {
                            GenericParameterKind::Type { .. } => {
                                Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                            }
                            GenericParameterKind::Const { .. } => {
                                Rc::new(RefCell::new(Type::ConstGeneric(
                                    gp.name.value.clone(),
                                    Rc::new(RefCell::new(Type::Never)),
                                )))
                            }
                        };
                        scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(gp.name.value.clone(), gp_type);
                        if let Some(default_value) = &gp.default_value {
                            self.resolve_expression(default_value.clone());
                        }
                    }
                }
                {
                    let self_sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: "self".to_string(),
                        decl: None,
                        parameter: None,
                        is_var: true,
                        ownership: OwnershipModifier::Strong,
                    }));
                    self.enter(self_sym, name);
                }
                for field_stmt in body {
                    if let Statement::VariableDecl {
                        name: field_name,
                        token: field_token,
                        ownership: field_ownership,
                        ..
                    } = &*field_stmt.borrow()
                    {
                        let is_var = field_token.value == "var";
                        let field_symbol = Rc::new(RefCell::new(Symbol::StructProperty {
                            name: field_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&struct_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_var,
                            ownership: *field_ownership,
                        }));
                        fields.push(field_symbol.clone());
                        self.enter(field_symbol, field_name);
                    } else if let Statement::FunctionDecl {
                        name: method_name, ..
                    } = &*field_stmt.borrow()
                    {
                        let method_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                            name: method_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&struct_symbol)),
                            decl: Some(field_stmt.clone()),
                        }));
                        methods.push(method_symbol.clone());
                        self.enter(method_symbol, method_name);
                        if let Statement::FunctionDecl {
                            body: method_body, ..
                        } = &*field_stmt.borrow()
                        {
                            match &*method_body.borrow() {
                                FunctionBody::Statements(stmts) => {
                                    for s in stmts {
                                        self.register_symbols(s.clone());
                                    }
                                }
                                FunctionBody::Expression(expr) => {
                                    self.register_function_symbols_in_expr(expr.clone());
                                }
                                FunctionBody::None => {}
                            }
                        }
                    } else if let Statement::InitDecl { body, .. } = &*field_stmt.borrow() {
                        let init_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                            name: "init".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&struct_symbol)),
                            decl: Some(field_stmt.clone()),
                        }));
                        constructors.push(init_symbol.clone());
                        let init_token = {
                            let stmt = field_stmt.borrow();
                            if let Statement::InitDecl { token, .. } = &*stmt {
                                Box::new(token.clone())
                            } else {
                                unreachable!()
                            }
                        };
                        self.enter(init_symbol, &init_token);
                        if let FunctionBody::Statements(stmts) = &*body.borrow() {
                            for s in stmts {
                                self.register_symbols(s.clone());
                            }
                        }
                    } else if let Statement::DeinitDecl { body, .. } = &*field_stmt.borrow() {
                        let deinit_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                            name: "deinit".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&struct_symbol)),
                            decl: Some(field_stmt.clone()),
                        }));
                        methods.push(deinit_symbol.clone());
                        let deinit_token = {
                            let stmt = field_stmt.borrow();
                            if let Statement::DeinitDecl { token, .. } = &*stmt {
                                token.as_ref().clone()
                            } else {
                                unreachable!()
                            }
                        };
                        if destrcutor.is_none() {
                            *destrcutor = Some(deinit_symbol.clone());
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::DuplicateFunction,
                                "Duplicate deinit function",
                                &deinit_token,
                            );
                        }
                        self.enter(deinit_symbol, &deinit_token);
                        if let FunctionBody::Statements(stmts) = &*body.borrow() {
                            for s in stmts {
                                self.register_symbols(s.clone());
                            }
                        }
                    } else if let Statement::SubscriptDecl { .. } = &*field_stmt.borrow() {
                        let sub_sym = Rc::new(RefCell::new(Symbol::StructSubscript {
                            name: "subscript".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&struct_symbol)),
                            decl: Some(field_stmt.clone()),
                        }));
                        _subscripts.push(sub_sym.clone());
                        {
                            let stmt = field_stmt.borrow();
                            if let Statement::SubscriptDecl {
                                parameters,
                                accessors,
                                ..
                            } = &*stmt
                            {
                                for accessor in accessors {
                                    let acc_scope = Rc::new(RefCell::new(Scope::new(
                                        self.current_scope.clone(),
                                    )));
                                    let saved = self.current_scope.clone();
                                    self.current_scope = Some(acc_scope.clone());
                                    for param in parameters {
                                        let param_name = param.borrow().name.value.clone();
                                        if param_name != "_" {
                                            let param_sym =
                                                Rc::new(RefCell::new(Symbol::Variable {
                                                    name: param_name,
                                                    decl: None,
                                                    parameter: Some(param.clone()),
                                                    is_var: true,
                                                    ownership: OwnershipModifier::Strong,
                                                }));
                                            self.enter(param_sym, &param.borrow().name);
                                        }
                                    }
                                    if let Some(p) = &accessor.parameter {
                                        let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                            name: p.value.clone(),
                                            decl: None,
                                            parameter: None,
                                            is_var: true,
                                            ownership: OwnershipModifier::Strong,
                                        }));
                                        self.enter(param_sym, p);
                                    } else {
                                        let param_name = match accessor.kind {
                                            AccessorKind::DidSet => "oldValue",
                                            _ => "newValue",
                                        };
                                        let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                            name: param_name.to_string(),
                                            decl: None,
                                            parameter: None,
                                            is_var: true,
                                            ownership: OwnershipModifier::Strong,
                                        }));
                                        if let Some(scope) = self.current_scope.clone() {
                                            scope.borrow_mut().set_symbol(param_sym);
                                        }
                                    }
                                    for s in &accessor.body {
                                        self.register_symbols(s.clone());
                                    }
                                    self.current_scope = saved;
                                }
                            }
                        }
                    } else {
                        self.register_symbols(field_stmt.clone());
                    }
                }
                self.leave_scope();
            }
            Statement::ClassDecl {
                attributes,
                name,
                body,
                scope,
                generic_parameters,
                modifiers,
                ..
            } => {
                let is_abstract = Self::has_modifier(modifiers, ModifierType::Abstract);
                let is_final = Self::has_modifier(modifiers, ModifierType::Final);
                let has_dml = attributes.iter().any(|a| a.name == "dynamicMemberLookup");
                let has_dcl = attributes.iter().any(|a| a.name == "dynamicCallable");
                let class_symbol = Rc::new(RefCell::new(Symbol::Class {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    has_dynamic_member_lookup: has_dml,
                    has_dynamic_callable: has_dcl,
                    properties: vec![],
                    methods: vec![],
                    constructors: vec![],
                    destrcutor: None,
                    superclass: None,
                    subscripts: vec![],
                    is_abstract,
                    is_final,
                }));
                self.enter(class_symbol.clone(), name);
                if let Some(scope) = self.current_scope.as_ref() {
                    let class_ty = Rc::new(RefCell::new(Type::Class(
                        name.value.clone(),
                        WeakSymbol(Rc::downgrade(&class_symbol)),
                        vec![],
                    )));
                    scope.borrow_mut().set_type(name.value.clone(), class_ty);
                }
                let Symbol::Class {
                    properties: fields,
                    methods,
                    constructors,
                    destrcutor,
                    subscripts: _subscripts,
                    ..
                } = &mut *class_symbol.borrow_mut()
                else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                {
                    for gp in generic_parameters {
                        let gp_type = match &gp.kind {
                            GenericParameterKind::Type { .. } => {
                                Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                            }
                            GenericParameterKind::Const { .. } => {
                                Rc::new(RefCell::new(Type::ConstGeneric(
                                    gp.name.value.clone(),
                                    Rc::new(RefCell::new(Type::Never)),
                                )))
                            }
                        };
                        scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(gp.name.value.clone(), gp_type);
                        if let Some(default_value) = &gp.default_value {
                            self.resolve_expression(default_value.clone());
                        }
                    }
                }
                {
                    let self_sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: "self".to_string(),
                        decl: None,
                        parameter: None,
                        is_var: true,
                        ownership: OwnershipModifier::Strong,
                    }));
                    self.enter(self_sym, name);
                }
                for field_stmt in body {
                    if let Statement::VariableDecl {
                        name: field_name,
                        token: field_token,
                        modifiers: field_mods,
                        ownership: field_ownership,
                        ..
                    } = &*field_stmt.borrow()
                    {
                        let is_var = field_token.value == "var";
                        let is_field_final = Self::has_modifier(field_mods, ModifierType::Final);
                        let is_field_override =
                            Self::has_modifier(field_mods, ModifierType::Override);
                        let field_symbol = Rc::new(RefCell::new(Symbol::ClassProperty {
                            name: field_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_var,
                            is_final: is_field_final || is_final,
                            is_override: is_field_override,
                            ownership: *field_ownership,
                        }));
                        fields.push(field_symbol.clone());
                        self.enter(field_symbol, field_name);
                    } else if let Statement::FunctionDecl {
                        name: method_name,
                        modifiers: method_mods,
                        ..
                    } = &*field_stmt.borrow()
                    {
                        let is_method_abstract =
                            Self::has_modifier(method_mods, ModifierType::Abstract);
                        let is_method_final = Self::has_modifier(method_mods, ModifierType::Final);
                        let is_method_override =
                            Self::has_modifier(method_mods, ModifierType::Override);
                        if is_method_abstract && !is_abstract {
                            self.emit_error(
                                TrussDiagnosticCode::AbstractMemberInNonAbstractClass,
                                format!(
                                    "Abstract method '{}' can only be defined in an abstract class",
                                    method_name.value
                                ),
                                method_name,
                            );
                        }
                        let method_symbol = Rc::new(RefCell::new(Symbol::ClassMethod {
                            name: method_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_abstract: is_method_abstract,
                            is_final: is_method_final || is_final,
                            is_override: is_method_override,
                        }));
                        methods.push(method_symbol.clone());
                        self.enter(method_symbol, method_name);
                        if let Statement::FunctionDecl {
                            body: method_body, ..
                        } = &*field_stmt.borrow()
                        {
                            match &*method_body.borrow() {
                                FunctionBody::Statements(stmts) => {
                                    for s in stmts {
                                        self.register_symbols(s.clone());
                                    }
                                }
                                FunctionBody::Expression(expr) => {
                                    self.register_function_symbols_in_expr(expr.clone());
                                }
                                FunctionBody::None => {}
                            }
                        }
                    } else if let Statement::InitDecl { body, .. } = &*field_stmt.borrow() {
                        let init_symbol = Rc::new(RefCell::new(Symbol::ClassMethod {
                            name: "init".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_abstract: false,
                            is_final: is_final,
                            is_override: false,
                        }));
                        constructors.push(init_symbol.clone());
                        let init_token = {
                            let stmt = field_stmt.borrow();
                            if let Statement::InitDecl { token, .. } = &*stmt {
                                Box::new(token.clone())
                            } else {
                                unreachable!()
                            }
                        };
                        self.enter(init_symbol, &init_token);
                        if let FunctionBody::Statements(stmts) = &*body.borrow() {
                            for s in stmts {
                                self.register_symbols(s.clone());
                            }
                        }
                    } else if let Statement::DeinitDecl { body, .. } = &*field_stmt.borrow() {
                        let deinit_symbol = Rc::new(RefCell::new(Symbol::ClassMethod {
                            name: "deinit".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_abstract: false,
                            is_final: is_final,
                            is_override: false,
                        }));
                        methods.push(deinit_symbol.clone());
                        let deinit_token = {
                            let stmt = field_stmt.borrow();
                            if let Statement::DeinitDecl { token, .. } = &*stmt {
                                token.as_ref().clone()
                            } else {
                                unreachable!()
                            }
                        };
                        if destrcutor.is_none() {
                            *destrcutor = Some(deinit_symbol.clone());
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::DuplicateFunction,
                                "Duplicate deinit function",
                                &deinit_token,
                            );
                        }
                        self.enter(deinit_symbol, &deinit_token);
                        if let FunctionBody::Statements(stmts) = &*body.borrow() {
                            for s in stmts {
                                self.register_symbols(s.clone());
                            }
                        }
                    } else if let Statement::SubscriptDecl { .. } = &*field_stmt.borrow() {
                        let sub_sym = Rc::new(RefCell::new(Symbol::ClassSubscript {
                            name: "subscript".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_final,
                        }));
                        _subscripts.push(sub_sym.clone());
                        {
                            let stmt = field_stmt.borrow();
                            if let Statement::SubscriptDecl {
                                parameters,
                                accessors,
                                ..
                            } = &*stmt
                            {
                                for accessor in accessors {
                                    let acc_scope = Rc::new(RefCell::new(Scope::new(
                                        self.current_scope.clone(),
                                    )));
                                    let saved = self.current_scope.clone();
                                    self.current_scope = Some(acc_scope.clone());
                                    for param in parameters {
                                        let param_name = param.borrow().name.value.clone();
                                        if param_name != "_" {
                                            let param_sym =
                                                Rc::new(RefCell::new(Symbol::Variable {
                                                    name: param_name,
                                                    decl: None,
                                                    parameter: Some(param.clone()),
                                                    is_var: true,
                                                    ownership: OwnershipModifier::Strong,
                                                }));
                                            self.enter(param_sym, &param.borrow().name);
                                        }
                                    }
                                    if let Some(p) = &accessor.parameter {
                                        let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                            name: p.value.clone(),
                                            decl: None,
                                            parameter: None,
                                            is_var: true,
                                            ownership: OwnershipModifier::Strong,
                                        }));
                                        self.enter(param_sym, p);
                                    } else {
                                        let param_name = match accessor.kind {
                                            AccessorKind::DidSet => "oldValue",
                                            _ => "newValue",
                                        };
                                        let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                            name: param_name.to_string(),
                                            decl: None,
                                            parameter: None,
                                            is_var: true,
                                            ownership: OwnershipModifier::Strong,
                                        }));
                                        if let Some(scope) = self.current_scope.clone() {
                                            scope.borrow_mut().set_symbol(param_sym);
                                        }
                                    }
                                    for s in &accessor.body {
                                        self.register_symbols(s.clone());
                                    }
                                    self.current_scope = saved;
                                }
                            }
                        }
                    } else {
                        self.register_symbols(field_stmt.clone());
                    }
                }
                self.leave_scope();
            }
            Statement::EnumDecl {
                attributes,
                name,
                cases: ast_cases,
                body,
                scope,
                generic_parameters,
                ..
            } => {
                let has_dml = attributes.iter().any(|a| a.name == "dynamicMemberLookup");
                let has_dcl = attributes.iter().any(|a| a.name == "dynamicCallable");
                let enum_symbol = Rc::new(RefCell::new(Symbol::Enum {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    has_dynamic_member_lookup: has_dml,
                    has_dynamic_callable: has_dcl,
                    cases: vec![],
                    methods: vec![],
                }));
                self.enter(enum_symbol.clone(), name);
                if let Some(scope) = self.current_scope.as_ref() {
                    let enum_ty = Rc::new(RefCell::new(Type::Enum(
                        name.value.clone(),
                        WeakSymbol(Rc::downgrade(&enum_symbol)),
                        vec![],
                    )));
                    scope.borrow_mut().set_type(name.value.clone(), enum_ty);
                }
                let Symbol::Enum { cases, methods, .. } = &mut *enum_symbol.borrow_mut() else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { .. } => {
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                Rc::new(RefCell::new(Type::Never)),
                            )))
                        }
                    };
                    scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.resolve_expression(default_value.clone());
                    }
                }
                for case in ast_cases {
                    let case_symbol = Rc::new(RefCell::new(Symbol::EnumCase {
                        name: case.name.value.clone(),
                        parent: WeakSymbol(Rc::downgrade(&enum_symbol)),
                        decl: Some(stmt.clone()),
                        parameter_types: vec![],
                    }));
                    cases.push(case_symbol.clone());
                    self.enter(case_symbol, &case.name);
                }
                for field_stmt in body {
                    if let Statement::FunctionDecl {
                        name: method_name, ..
                    } = &*field_stmt.borrow()
                    {
                        let method_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                            name: method_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&enum_symbol)),
                            decl: Some(field_stmt.clone()),
                        }));
                        methods.push(method_symbol.clone());
                        self.enter(method_symbol, method_name);
                        if let Statement::FunctionDecl {
                            body: method_body, ..
                        } = &*field_stmt.borrow()
                        {
                            match &*method_body.borrow() {
                                FunctionBody::Statements(stmts) => {
                                    for s in stmts {
                                        self.register_symbols(s.clone());
                                    }
                                }
                                FunctionBody::Expression(expr) => {
                                    self.register_function_symbols_in_expr(expr.clone());
                                }
                                FunctionBody::None => {}
                            }
                        }
                    } else {
                        self.register_symbols(field_stmt.clone());
                    }
                }
                self.leave_scope();
            }
            Statement::ExternBlock { items, .. } => {
                for item in items {
                    self.register_symbols(item.clone());
                }
            }
            Statement::ExternDecl { statement, .. } => {
                self.register_symbols(statement.clone());
            }
            Statement::Defer { body, .. } => {
                for stmt in body {
                    self.register_symbols(stmt.clone());
                }
            }
            Statement::ProtocolDecl {
                name,
                members,
                scope,
                generic_parameters,
                conformances,
                ..
            } => {
                let is_any_object = conformances.iter().any(|c| {
                    if let Expression::Type { name: cn, .. } = &*c.borrow() {
                        cn.value == "AnyObject"
                    } else {
                        false
                    }
                });
                let protocol_symbol = Rc::new(RefCell::new(Symbol::Protocol {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    methods: vec![],
                    properties: vec![],
                    subscripts: vec![],
                    is_any_object_protocol: is_any_object,
                }));
                self.enter(protocol_symbol.clone(), name);
                if let Some(scope) = self.current_scope.as_ref() {
                    let protocol_ty = Rc::new(RefCell::new(Type::Protocol(
                        name.value.clone(),
                        WeakSymbol(Rc::downgrade(&protocol_symbol)),
                        vec![],
                    )));
                    scope.borrow_mut().set_type(name.value.clone(), protocol_ty);
                }
                let Symbol::Protocol {
                    methods,
                    properties,
                    subscripts,
                    ..
                } = &mut *protocol_symbol.borrow_mut()
                else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { .. } => {
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                Rc::new(RefCell::new(Type::Never)),
                            )))
                        }
                    };
                    scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.resolve_expression(default_value.clone());
                    }
                }
                for member in members {
                    match member {
                        ProtocolMember::Method {
                            attributes, decl, ..
                        } => {
                            let is_autowired = attributes.iter().any(|a| a.name == "autowired");
                            let name_token = {
                                let d = decl.borrow();
                                if let Statement::FunctionDecl { name: fn_name, .. } = &*d {
                                    fn_name.clone()
                                } else {
                                    continue;
                                }
                            };
                            let method_name = name_token.value.clone();
                            let method_symbol = Rc::new(RefCell::new(Symbol::ProtocolMethod {
                                name: method_name,
                                parent: WeakSymbol(Rc::downgrade(&protocol_symbol)),
                                decl: Some(decl.clone()),
                                is_autowired,
                            }));
                            methods.push(method_symbol.clone());
                            self.enter(method_symbol, &name_token);
                            if let Statement::FunctionDecl {
                                body: method_body, ..
                            } = &*decl.borrow()
                            {
                                match &*method_body.borrow() {
                                    FunctionBody::Statements(stmts) => {
                                        for s in stmts {
                                            self.register_symbols(s.clone());
                                        }
                                    }
                                    FunctionBody::Expression(expr) => {
                                        self.register_function_symbols_in_expr(expr.clone());
                                    }
                                    FunctionBody::None => {}
                                }
                            }
                        }
                        ProtocolMember::Property {
                            name: prop_name,
                            accessors,
                            ..
                        } => {
                            let prop_symbol = Rc::new(RefCell::new(Symbol::ProtocolProperty {
                                name: prop_name.value.clone(),
                                parent: WeakSymbol(Rc::downgrade(&protocol_symbol)),
                                decl: None,
                                accessors: *accessors,
                            }));
                            properties.push(prop_symbol.clone());
                            self.enter(prop_symbol, prop_name);
                        }
                        ProtocolMember::AssociatedType { name, .. } => {
                            let at_type =
                                Rc::new(RefCell::new(Type::GenericParam(name.value.clone())));
                            scope
                                .as_ref()
                                .unwrap()
                                .borrow_mut()
                                .set_type(name.value.clone(), at_type);
                        }
                        ProtocolMember::TypeAlias {
                            type_expression, ..
                        } => {
                            self.resolve_expression(type_expression.clone());
                        }
                        ProtocolMember::Subscript { accessors, .. } => {
                            let sub = Rc::new(RefCell::new(Symbol::ProtocolSubscript {
                                name: "subscript".to_string(),
                                parent: WeakSymbol(Rc::downgrade(&protocol_symbol)),
                                decl: None,
                                accessors: *accessors,
                            }));
                            subscripts.push(sub);
                        }
                        ProtocolMember::StaticVar {
                            name: prop_name,
                            accessors,
                            ..
                        } => {
                            let prop_symbol = Rc::new(RefCell::new(Symbol::ProtocolProperty {
                                name: prop_name.value.clone(),
                                parent: WeakSymbol(Rc::downgrade(&protocol_symbol)),
                                decl: None,
                                accessors: *accessors,
                            }));
                            properties.push(prop_symbol.clone());
                            self.enter(prop_symbol, prop_name);
                        }
                        ProtocolMember::Init { token, .. } => {
                            let method_symbol = Rc::new(RefCell::new(Symbol::ProtocolMethod {
                                name: "init".to_string(),
                                parent: WeakSymbol(Rc::downgrade(&protocol_symbol)),
                                decl: None,
                                is_autowired: false,
                            }));
                            methods.push(method_symbol.clone());
                            self.enter(method_symbol, token);
                        }
                    }
                }
                self.leave_scope();
            }
            Statement::ExtensionDecl {
                type_name,
                body,
                type_arguments,
                ..
            } => {
                if let Some(type_arguments) = type_arguments {
                    for ta in type_arguments {
                        self.resolve_expression(ta.clone());
                    }
                }
                let Some(target_sym) = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().get_symbol(&type_name.value))
                else {
                    self.emit_error(
                        TrussDiagnosticCode::SymbolError,
                        format!("Cannot extend undefined type '{}'", type_name.value),
                        type_name.as_ref(),
                    );
                    return;
                };

                let target_scope = {
                    let decl = target_sym.borrow().get_decl().ok().flatten();
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
                };

                let saved = self.current_scope.clone();
                if let Some(ref target_scope) = target_scope {
                    self.current_scope = Some(target_scope.clone());
                }

                let target_symbol_rc = target_sym.clone();
                match &mut *target_symbol_rc.borrow_mut() {
                    Symbol::Struct {
                        methods,
                        constructors,
                        destrcutor,
                        subscripts: _subscripts,
                        ..
                    }
                    | Symbol::Class {
                        methods,
                        constructors,
                        destrcutor,
                        subscripts: _subscripts,
                        ..
                    } => {
                        for field_stmt in body {
                            if let Statement::FunctionDecl {
                                name: method_name, ..
                            } = &*field_stmt.borrow()
                            {
                                let method_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                                    name: method_name.value.clone(),
                                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                                    decl: Some(field_stmt.clone()),
                                }));
                                methods.push(method_symbol.clone());
                                self.enter(method_symbol, method_name);
                                if let Statement::FunctionDecl {
                                    body: method_body, ..
                                } = &*field_stmt.borrow()
                                {
                                    match &*method_body.borrow() {
                                        FunctionBody::Statements(stmts) => {
                                            for s in stmts {
                                                self.register_symbols(s.clone());
                                            }
                                        }
                                        FunctionBody::Expression(expr) => {
                                            self.register_function_symbols_in_expr(expr.clone());
                                        }
                                        FunctionBody::None => {}
                                    }
                                }
                            } else if let Statement::InitDecl { body, .. } = &*field_stmt.borrow() {
                                let init_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                                    name: "init".to_string(),
                                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                                    decl: Some(field_stmt.clone()),
                                }));
                                constructors.push(init_symbol.clone());
                                let init_token = {
                                    let stmt = field_stmt.borrow();
                                    if let Statement::InitDecl { token, .. } = &*stmt {
                                        Box::new(token.clone())
                                    } else {
                                        unreachable!()
                                    }
                                };
                                self.enter(init_symbol, &init_token);
                                if let FunctionBody::Statements(stmts) = &*body.borrow() {
                                    for s in stmts {
                                        self.register_symbols(s.clone());
                                    }
                                }
                            } else if let Statement::DeinitDecl { body, .. } = &*field_stmt.borrow()
                            {
                                let deinit_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                                    name: "deinit".to_string(),
                                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                                    decl: Some(field_stmt.clone()),
                                }));
                                let deinit_token = {
                                    let stmt = field_stmt.borrow();
                                    if let Statement::DeinitDecl { token, .. } = &*stmt {
                                        token.as_ref().clone()
                                    } else {
                                        unreachable!()
                                    }
                                };
                                if destrcutor.is_none() {
                                    *destrcutor = Some(deinit_symbol.clone());
                                } else {
                                    self.emit_error(
                                        TrussDiagnosticCode::DuplicateFunction,
                                        "Duplicate deinit function",
                                        &deinit_token,
                                    );
                                }
                                self.enter(deinit_symbol, &deinit_token);
                                if let FunctionBody::Statements(stmts) = &*body.borrow() {
                                    for s in stmts {
                                        self.register_symbols(s.clone());
                                    }
                                }
                            } else if let Statement::SubscriptDecl { .. } = &*field_stmt.borrow() {
                                let sub_sym = Rc::new(RefCell::new(Symbol::StructSubscript {
                                    name: "subscript".to_string(),
                                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                                    decl: Some(field_stmt.clone()),
                                }));
                                _subscripts.push(sub_sym.clone());
                            } else {
                                self.register_symbols(field_stmt.clone());
                            }
                        }
                    }
                    Symbol::Enum { methods, .. } => {
                        for field_stmt in body {
                            if let Statement::FunctionDecl {
                                name: method_name, ..
                            } = &*field_stmt.borrow()
                            {
                                let method_symbol = Rc::new(RefCell::new(Symbol::StructMethod {
                                    name: method_name.value.clone(),
                                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                                    decl: Some(field_stmt.clone()),
                                }));
                                methods.push(method_symbol.clone());
                                self.enter(method_symbol, method_name);
                                if let Statement::FunctionDecl {
                                    body: method_body, ..
                                } = &*field_stmt.borrow()
                                {
                                    match &*method_body.borrow() {
                                        FunctionBody::Statements(stmts) => {
                                            for s in stmts {
                                                self.register_symbols(s.clone());
                                            }
                                        }
                                        FunctionBody::Expression(expr) => {
                                            self.register_function_symbols_in_expr(expr.clone());
                                        }
                                        FunctionBody::None => {}
                                    }
                                }
                            } else {
                                self.register_symbols(field_stmt.clone());
                            }
                        }
                    }
                    Symbol::Protocol {
                        methods,
                        subscripts: _subscripts,
                        ..
                    } => {
                        for field_stmt in body {
                            if let Statement::FunctionDecl {
                                name: method_name, ..
                            } = &*field_stmt.borrow()
                            {
                                let method_symbol = Rc::new(RefCell::new(Symbol::ProtocolMethod {
                                    name: method_name.value.clone(),
                                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                                    decl: Some(field_stmt.clone()),
                                    is_autowired: false,
                                }));
                                methods.push(method_symbol.clone());
                                self.enter(method_symbol, method_name);
                                if let Statement::FunctionDecl {
                                    body: method_body, ..
                                } = &*field_stmt.borrow()
                                {
                                    match &*method_body.borrow() {
                                        FunctionBody::Statements(stmts) => {
                                            for s in stmts {
                                                self.register_symbols(s.clone());
                                            }
                                        }
                                        FunctionBody::Expression(expr) => {
                                            self.register_function_symbols_in_expr(expr.clone());
                                        }
                                        FunctionBody::None => {}
                                    }
                                }
                            } else if let Statement::SubscriptDecl {
                                accessors: sub_accessors,
                                ..
                            } = &*field_stmt.borrow()
                            {
                                let req_accessors = ProtocolAccessorSet {
                                    get: sub_accessors.iter().any(|a| a.kind == AccessorKind::Get),
                                    set: sub_accessors.iter().any(|a| a.kind == AccessorKind::Set),
                                };
                                let sub_sym = Rc::new(RefCell::new(Symbol::ProtocolSubscript {
                                    name: "subscript".to_string(),
                                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                                    decl: Some(field_stmt.clone()),
                                    accessors: req_accessors,
                                }));
                                _subscripts.push(sub_sym.clone());
                            } else {
                                self.register_symbols(field_stmt.clone());
                            }
                        }
                    }
                    _ => {}
                }

                self.current_scope = saved;
            }
            Statement::ModuleDecl {
                name, body, scope, ..
            } => {
                let full_path = {
                    let parent = self
                        .current_module
                        .as_ref()
                        .map(|m| m.borrow().name.clone());
                    match parent {
                        Some(ref p) if *p == self.root_module_name => name.value.clone(),
                        Some(ref p) => format!("{}.{}", p, name.value),
                        None => name.value.clone(),
                    }
                };
                let module = Rc::new(RefCell::new(Module::new(full_path.clone())));
                self.packages
                    .get(&self.current_package)
                    .unwrap()
                    .borrow_mut()
                    .modules
                    .insert(full_path, module.clone());
                if let Some(current) = &self.current_module {
                    current
                        .borrow_mut()
                        .children
                        .insert(name.value.clone(), module.clone());
                }

                let module_symbol = Rc::new(RefCell::new(Symbol::Module {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    module: Some(module.clone()),
                }));
                self.enter(module_symbol, name);

                *scope = Some(self.enter_scope(None));
                module.borrow_mut().scope = scope.clone();

                let saved_module = self.current_module.replace(module);
                for s in body {
                    self.register_symbols(s.clone());
                }
                self.leave_scope();
                self.current_module = saved_module;
            }
            Statement::ImportDecl {
                path,
                kind,
                token,
                selective_members,
                is_current_package,
            } => {
                let resolve_module =
                    |pkg: &Rc<RefCell<Package>>, mp: &str| -> Option<Rc<RefCell<Module>>> {
                        pkg.borrow().modules.get(mp).cloned()
                    };
                if let Some(members) = selective_members {
                    let module_path = path.join(".");
                    let target_pkg = if *is_current_package {
                        self.packages.get(&self.current_package).cloned()
                    } else if path.len() >= 1 && self.packages.contains_key(&path[0]) {
                        self.packages.get(&path[0]).cloned()
                    } else {
                        self.packages.get(&self.current_package).cloned()
                    };
                    let module = target_pkg
                        .as_ref()
                        .and_then(|p| resolve_module(p, &module_path));
                    if let Some(module) = module {
                        for member in members {
                            let found_symbol = module
                                .borrow()
                                .scope
                                .clone()
                                .and_then(|scope| scope.borrow().get_symbol(&member.name));
                            if let Some(symbol) = found_symbol {
                                let alias_name = match &member.alias {
                                    SelectiveAlias::Direct => &member.name,
                                    SelectiveAlias::Named(alias) => alias,
                                    SelectiveAlias::Skip => continue,
                                };
                                let name_token = Token::new(
                                    alias_name.clone(),
                                    crate::lexer::token::TokenType::Identifier,
                                    token.position.clone(),
                                    token.file.clone(),
                                );
                                if alias_name != &member.name {
                                    let renamed = Rc::new(RefCell::new(
                                        symbol.borrow().with_name(alias_name),
                                    ));
                                    self.enter(renamed, &name_token);
                                } else {
                                    self.enter(symbol, &name_token);
                                }
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::SymbolError,
                                    format!(
                                        "Symbol '{}' not found in module '{}'",
                                        member.name, module_path
                                    ),
                                    token.as_ref(),
                                );
                            }
                        }
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::SymbolError,
                            format!("Module '{}' not found", module_path),
                            token.as_ref(),
                        );
                    }
                } else {
                    match kind {
                        ImportKind::Module => {
                            let module_path = path.join(".");
                            let target_pkg = if *is_current_package {
                                self.packages.get(&self.current_package).cloned()
                            } else if path.len() >= 1 && self.packages.contains_key(&path[0]) {
                                self.packages.get(&path[0]).cloned()
                            } else {
                                self.packages.get(&self.current_package).cloned()
                            };
                            let module = target_pkg
                                .as_ref()
                                .and_then(|p| resolve_module(p, &module_path));
                            if let Some(module) = module {
                                let name = path.last().unwrap().clone();
                                let module_symbol = Rc::new(RefCell::new(Symbol::Module {
                                    name: name.clone(),
                                    decl: stmt.clone(),
                                    module: Some(module),
                                }));
                                let name_token = Token::new(
                                    name,
                                    crate::lexer::token::TokenType::Identifier,
                                    token.position.clone(),
                                    token.file.clone(),
                                );
                                self.enter(module_symbol, &name_token);
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::SymbolError,
                                    format!("Module '{}' not found", module_path),
                                    token.as_ref(),
                                );
                            }
                        }
                        ImportKind::Member => {
                            let member_name = path.last().unwrap().clone();
                            let module_path = path[..path.len() - 1].join(".");
                            let target_pkg = if *is_current_package {
                                self.packages.get(&self.current_package).cloned()
                            } else if path.len() >= 2 && self.packages.contains_key(&path[0]) {
                                self.packages.get(&path[0]).cloned()
                            } else {
                                self.packages.get(&self.current_package).cloned()
                            };
                            let found_symbol = target_pkg.as_ref().and_then(|p| {
                                p.borrow().modules.get(&module_path).and_then(|m| {
                                    m.borrow()
                                        .scope
                                        .clone()
                                        .and_then(|scope| scope.borrow().get_symbol(&member_name))
                                })
                            });
                            if let Some(symbol) = found_symbol {
                                self.enter(symbol, token.as_ref());
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::SymbolError,
                                    format!("Symbol '{}' not found", member_name),
                                    token.as_ref(),
                                );
                            }
                        }
                        ImportKind::Wildcard => {
                            let module_path = path.join(".");
                            let target_pkg = if *is_current_package {
                                self.packages.get(&self.current_package).cloned()
                            } else if path.len() >= 1 && self.packages.contains_key(&path[0]) {
                                self.packages.get(&path[0]).cloned()
                            } else {
                                self.packages.get(&self.current_package).cloned()
                            };
                            let module = target_pkg
                                .as_ref()
                                .and_then(|p| resolve_module(p, &module_path));
                            if let Some(module) = module {
                                let names: Vec<String> = {
                                    let scope = module.borrow().scope.clone();
                                    scope
                                        .map(|s| s.borrow().name_table.keys().cloned().collect())
                                        .unwrap_or_default()
                                };
                                for name in names {
                                    if let Some(symbol) = module
                                        .borrow()
                                        .scope
                                        .clone()
                                        .and_then(|scope| scope.borrow().get_symbol(&name))
                                    {
                                        let name_token = Token::new(
                                            name.clone(),
                                            crate::lexer::token::TokenType::Identifier,
                                            token.position.clone(),
                                            token.file.clone(),
                                        );
                                        self.enter(symbol, &name_token);
                                    }
                                }
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::SymbolError,
                                    format!("Module '{}' not found", module_path),
                                    token.as_ref(),
                                );
                            }
                        }
                    }
                }
            }
            Statement::MacroDecl { name, .. } => {
                let symbol = Rc::new(RefCell::new(Symbol::Macro {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                }));
                self.enter(symbol, name);
            }
            Statement::ConditionalBlock { clauses } => {
                for clause in clauses {
                    for s in &clause.body {
                        self.register_symbols(s.clone());
                    }
                }
            }
            Statement::PragmaError { .. } | Statement::PragmaWarning { .. } => {}
            Statement::AsmBlock { .. } => {}
            _ => {}
        }
    }

    fn register_function_symbols_in_expr(&mut self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Closure { body, .. } = &*expr.borrow() {
            for stmt in body {
                self.register_symbols(stmt.clone());
            }
        } else if let Expression::TupleLiteral { elements, .. } = &*expr.borrow() {
            for (_, element) in elements {
                self.register_function_symbols_in_expr(element.clone());
            }
        }
    }

    fn resolve_statement(&mut self, stmt: Rc<RefCell<Statement>>) {
        match &mut *stmt.borrow_mut() {
            Statement::FunctionDecl {
                parameters,
                return_type,
                body,
                scope,
                generic_parameters,
                where_clause,
                ..
            } => {
                if scope.is_none() {
                    *scope = Some(self.enter_scope(None));
                } else {
                    self.enter_scope(scope.clone());
                }
                for gp in generic_parameters {
                    let gp_type = match &gp.kind {
                        GenericParameterKind::Type { .. } => {
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())))
                        }
                        GenericParameterKind::Const { .. } => {
                            Rc::new(RefCell::new(Type::ConstGeneric(
                                gp.name.value.clone(),
                                Rc::new(RefCell::new(Type::Never)),
                            )))
                        }
                    };
                    scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                    if let Some(default_value) = &gp.default_value {
                        self.resolve_expression(default_value.clone());
                    }
                }
                for parameter in parameters {
                    let name = parameter.borrow().name.value.clone();
                    if name != "_" {
                        let symbol = Rc::new(RefCell::new(Symbol::Variable {
                            name,
                            decl: None,
                            parameter: Some(parameter.clone()),
                            is_var: true,
                            ownership: OwnershipModifier::Strong,
                        }));
                        self.enter(symbol, &parameter.borrow().name);
                    }
                    if let Some(default_value) = &parameter.borrow().default_value {
                        self.resolve_expression(default_value.clone());
                    }
                }
                if let Some(return_type) = return_type {
                    self.resolve_expression(return_type.clone());
                }
                if let Some(where_clause) = where_clause {
                    for req in where_clause {
                        self.resolve_where_requirement(req);
                    }
                }
                self.resolve_function_body(body.clone());
                self.leave_scope();
            }
            Statement::VariableDecl {
                name,
                pattern: decl_pattern,
                token: var_token,
                initializer,
                accessors,
                ..
            } => {
                if let Some(pattern) = decl_pattern {
                    let is_var = var_token.value == "var";
                    Self::resolve_variable_pattern(pattern, stmt.clone(), is_var, self);
                } else if name.value != "_" {
                    let is_var = var_token.value == "var";
                    let ownership =
                        if let Statement::VariableDecl { ownership, .. } = &*stmt.borrow() {
                            *ownership
                        } else {
                            OwnershipModifier::Strong
                        };
                    let symbol = Rc::new(RefCell::new(Symbol::Variable {
                        name: name.value.clone(),
                        decl: Some(stmt.clone()),
                        parameter: None,
                        is_var,
                        ownership,
                    }));
                    self.enter(symbol, name);
                }
                if let Some(initializer) = initializer {
                    self.resolve_expression(initializer.clone());
                }
                if !accessors.is_empty() {
                    for accessor in accessors {
                        let accessor_scope =
                            Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
                        let saved = self.current_scope.clone();
                        self.current_scope = Some(accessor_scope.clone());
                        let backing_sym = Rc::new(RefCell::new(Symbol::Variable {
                            name: format!("_{}", name.value),
                            decl: None,
                            parameter: None,
                            is_var: true,
                            ownership: OwnershipModifier::Strong,
                        }));
                        accessor_scope.borrow_mut().set_symbol(backing_sym);
                        if let Some(param) = &accessor.parameter {
                            let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                name: param.value.clone(),
                                decl: None,
                                parameter: None,
                                is_var: true,
                                ownership: OwnershipModifier::Strong,
                            }));
                            self.enter(param_sym, param);
                        } else {
                            let param_name = match accessor.kind {
                                AccessorKind::DidSet => "oldValue",
                                _ => "newValue",
                            };
                            let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                name: param_name.to_string(),
                                decl: None,
                                parameter: None,
                                is_var: true,
                                ownership: OwnershipModifier::Strong,
                            }));
                            if let Some(scope) = self.current_scope.clone() {
                                scope.borrow_mut().set_symbol(param_sym);
                            }
                        }
                        for stmt in &accessor.body {
                            self.resolve_statement(stmt.clone());
                        }
                        self.current_scope = saved;
                    }
                }
            }
            Statement::StructDecl {
                name,
                body,
                scope,
                conformances,
                where_clause,
                ..
            } => {
                for conformance in conformances.iter() {
                    self.resolve_conformance(conformance.clone());
                }
                if let Some(where_clause) = where_clause {
                    for req in where_clause {
                        self.resolve_where_requirement(req);
                    }
                }
                let has_copyable_suppression = conformances.iter().any(|c| {
                    matches!(&*c.borrow(), Expression::Unary { operator: UnaryOperator::BitNot, expression, .. }
                        if matches!(&*expression.borrow(), Expression::Type { name, .. } if name.value == "Copyable"))
                });
                let needs_copy = !has_copyable_suppression && conformances.iter().any(|expr| {
                    let e = expr.borrow();
                    matches!(&*e, Expression::Type { name, .. } if name.value == "Copyable")
                });
                if needs_copy {
                    let already_has_copy = body.iter().any(|s| {
                        matches!(&*s.borrow(), Statement::FunctionDecl { name: n, .. } if n.value == "copy")
                    });
                    if !already_has_copy {
                        let pos = name.position.clone();
                        let file = name.file.clone();
                        let func_name_tok = Token::new(
                            "copy".to_string(),
                            TokenType::Identifier,
                            pos.clone(),
                            file.clone(),
                        );
                        let self_type_expr = Expression::Type {
                            name: Box::new(Token::new(
                                name.value.clone(),
                                TokenType::Identifier,
                                pos.clone(),
                                file.clone(),
                            )),
                            type_parameters: None,
                            ty: None,
                        };
                        let self_var_expr = Expression::Variable {
                            name: Box::new(Token::new(
                                "self".to_string(),
                                TokenType::Keyword {
                                    keyword: KeywordType::SelfKw,
                                },
                                pos.clone(),
                                file.clone(),
                            )),
                            ty: None,
                            symbol: None,
                        };
                        let return_stmt = Statement::Return {
                            token: Box::new(Token::new(
                                "return".to_string(),
                                TokenType::Keyword {
                                    keyword: KeywordType::Return,
                                },
                                pos.clone(),
                                file.clone(),
                            )),
                            value: Some(Rc::new(RefCell::new(self_var_expr))),
                        };
                        let copy_func = Statement::FunctionDecl {
                            attributes: vec![],
                            modifiers: vec![],
                            token: Box::new(func_name_tok.clone()),
                            name: Box::new(func_name_tok),
                            generic_parameters: vec![],
                            parameters: vec![],
                            return_type: Some(Rc::new(RefCell::new(self_type_expr))),
                            throws_types: None,
                            body: Rc::new(RefCell::new(FunctionBody::Statements(vec![Rc::new(
                                RefCell::new(return_stmt),
                            )]))),
                            where_clause: None,
                            scope: None,
                            ty: None,
                            static_method: false,
                            mutating: false,
                            operator_fixity: None,
                        };
                        let func_stmt = Rc::new(RefCell::new(copy_func));
                        body.push(func_stmt.clone());
                        let struct_sym = self
                            .current_scope
                            .as_ref()
                            .and_then(|scope| scope.borrow().get_symbol(&name.value));
                        if let Some(st) = struct_sym {
                            let mut st_binding = st.borrow_mut();
                            if let Symbol::Struct { methods, .. } = &mut *st_binding {
                                let method_sym = Rc::new(RefCell::new(Symbol::StructMethod {
                                    name: "copy".to_string(),
                                    parent: WeakSymbol(Rc::downgrade(&st)),
                                    decl: Some(func_stmt.clone()),
                                }));
                                methods.push(method_sym.clone());
                                if let Some(s) = scope.as_ref() {
                                    s.borrow_mut().set_symbol(method_sym);
                                }
                            }
                        }
                    }
                }
                let is_builtintype = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().get_symbol(&name.value))
                    .map_or(false, |sym| {
                        matches!(
                            &*sym.borrow(),
                            Symbol::Struct {
                                is_builtin_type: true,
                                ..
                            }
                        )
                    });
                if is_builtintype {
                    for conformance in conformances.iter() {
                        let protocol_name = {
                            let e = conformance.borrow();
                            match &*e {
                                Expression::Type { name: n, .. } => n.value.clone(),
                                _ => continue,
                            }
                        };
                        let protocol_sym =
                            match self.current_scope.as_ref().and_then(|scope| {
                                scope.borrow().get_symbol(&protocol_name)
                            }) {
                                Some(s) => s,
                                None => continue,
                            };
                        let autowired_methods: Vec<(
                            String,
                            Rc<RefCell<Statement>>,
                        )> = {
                            let sym = protocol_sym.borrow();
                            match &*sym {
                                Symbol::Protocol { methods, .. } => methods
                                    .iter()
                                    .filter(|m| {
                                        let mb = m.borrow();
                                        matches!(
                                            &*mb,
                                            Symbol::ProtocolMethod {
                                                is_autowired: true,
                                                ..
                                            }
                                        )
                                    })
                                    .filter_map(|m| {
                                        let mb = m.borrow();
                                        if let Symbol::ProtocolMethod {
                                            name, decl, ..
                                        } = &*mb
                                        {
                                            Some((name.clone(), decl.clone()?))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect(),
                                _ => vec![],
                            }
                        };
                        for (method_name, method_decl) in autowired_methods {
                            let already_has = body.iter().any(|s| {
                                matches!(
                                    &*s.borrow(),
                                    Statement::FunctionDecl { name: n, .. }
                                        if n.value == method_name
                                )
                            });
                            if already_has {
                                continue;
                            }
                            let (params, ret_type, sm, mutating, op_fixity, throws) = {
                                let d = method_decl.borrow();
                                match &*d {
                                    Statement::FunctionDecl {
                                        parameters,
                                        return_type,
                                        static_method,
                                        mutating,
                                        operator_fixity,
                                        throws_types,
                                        ..
                                    } => (
                                        parameters.clone(),
                                        return_type.clone(),
                                        *static_method,
                                        *mutating,
                                        *operator_fixity,
                                        throws_types.clone(),
                                    ),
                                    _ => continue,
                                }
                            };
                            let pos = name.position.clone();
                            let file = name.file.clone();
                            let func_name_tok = Token::new(
                                method_name.clone(),
                                TokenType::Identifier,
                                pos.clone(),
                                file.clone(),
                            );
                            let generated_func = Statement::FunctionDecl {
                                attributes: vec![Attribute {
                                    name: "autowired".to_string(),
                                    value: None,
                                }],
                                modifiers: vec![],
                                token: Box::new(func_name_tok.clone()),
                                name: Box::new(func_name_tok),
                                generic_parameters: vec![],
                                parameters: params,
                                return_type: ret_type,
                                throws_types: throws,
                                body: Rc::new(RefCell::new(FunctionBody::None)),
                                where_clause: None,
                                scope: None,
                                ty: None,
                                static_method: sm,
                                mutating,
                                operator_fixity: op_fixity,
                            };
                            let func_stmt = Rc::new(RefCell::new(generated_func));
                            body.push(func_stmt.clone());
                            if let Some(st) = self
                                .current_scope
                                .as_ref()
                                .and_then(|scope| scope.borrow().get_symbol(&name.value))
                            {
                                let mut st_binding = st.borrow_mut();
                                if let Symbol::Struct { methods, .. } = &mut *st_binding {
                                    let method_sym = Rc::new(RefCell::new(
                                        Symbol::StructMethod {
                                            name: method_name.clone(),
                                            parent: WeakSymbol(Rc::downgrade(&st)),
                                            decl: Some(func_stmt.clone()),
                                        },
                                    ));
                                    methods.push(method_sym.clone());
                                    if let Some(s) = scope.as_ref() {
                                        s.borrow_mut().set_symbol(method_sym);
                                    }
                                }
                            }
                        }
                    }
                }
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Statement::ClassDecl {
                name,
                body,
                scope,
                superclass,
                conformances,
                where_clause,
                ..
            } => {
                if let Some(superclass_expr) = superclass {
                    if let Expression::Type {
                        name: super_name,
                        type_parameters,
                        ..
                    } = &*superclass_expr.borrow()
                    {
                        let _ = self.resolve_symbol(super_name);
                        if let Some(type_parameters) = type_parameters {
                            for tp in type_parameters {
                                self.resolve_expression(tp.clone());
                            }
                        }
                        if let Some(super_sym) = self
                            .current_scope
                            .as_ref()
                            .and_then(|scope| scope.borrow().get_symbol(&super_name.value))
                        {
                            if let Some(class_sym) = self
                                .current_scope
                                .as_ref()
                                .and_then(|scope| scope.borrow().get_symbol(&name.value))
                            {
                                let mut class_binding = class_sym.borrow_mut();
                                if let Symbol::Class { superclass: sc, .. } = &mut *class_binding {
                                    *sc = Some(WeakSymbol(Rc::downgrade(&super_sym)));
                                }
                            }
                        }
                    }
                }
                for conformance in conformances {
                    self.resolve_conformance(conformance.clone());
                }
                if let Some(where_clause) = where_clause {
                    for req in where_clause {
                        self.resolve_where_requirement(req);
                    }
                }
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Statement::EnumDecl {
                body,
                scope,
                where_clause,
                conformances,
                ..
            } => {
                for conformance in conformances {
                    self.resolve_conformance(conformance.clone());
                }
                if let Some(where_clause) = where_clause {
                    for req in where_clause {
                        self.resolve_where_requirement(req);
                    }
                }
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Statement::InitDecl {
                parameters,
                body,
                scope,
                ..
            } => {
                *scope = Some(self.enter_scope(None));
                for parameter in parameters {
                    let name = parameter.borrow().name.value.clone();
                    if name != "_" {
                        let symbol = Rc::new(RefCell::new(Symbol::Variable {
                            name,
                            decl: None,
                            parameter: Some(parameter.clone()),
                            is_var: true,
                            ownership: OwnershipModifier::Strong,
                        }));
                        self.enter(symbol, &parameter.borrow().name);
                    }
                    if let Some(default_value) = &parameter.borrow().default_value {
                        self.resolve_expression(default_value.clone());
                    }
                }
                self.resolve_function_body(body.clone());
                self.leave_scope();
            }
            Statement::DeinitDecl { body, scope, .. } => {
                *scope = Some(self.enter_scope(None));
                self.resolve_function_body(body.clone());
                self.leave_scope();
            }
            Statement::ExternBlock { items, .. } => {
                for item in items {
                    self.resolve_statement(item.clone());
                }
            }
            Statement::ExternDecl { statement, .. } => {
                self.resolve_statement(statement.clone());
            }
            Statement::ProtocolDecl {
                conformances,
                scope,
                members,
                where_clause,
                ..
            } => {
                for conformance in conformances {
                    self.resolve_conformance(conformance.clone());
                }
                if let Some(where_clause) = where_clause {
                    for req in where_clause {
                        self.resolve_where_requirement(req);
                    }
                }
                self.enter_scope(scope.clone());
                for member in members {
                    match member {
                        ProtocolMember::Method { decl, .. } => {
                            if let Statement::FunctionDecl {
                                parameters,
                                return_type,
                                body,
                                scope: fn_scope,
                                ..
                            } = &mut *decl.borrow_mut()
                            {
                                *fn_scope = Some(self.enter_scope(None));
                                for parameter in parameters {
                                    let name = parameter.borrow().name.value.clone();
                                    if name != "_" {
                                        let symbol = Rc::new(RefCell::new(Symbol::Variable {
                                            name,
                                            decl: None,
                                            parameter: Some(parameter.clone()),
                                            is_var: true,
                                            ownership: OwnershipModifier::Strong,
                                        }));
                                        self.enter(symbol, &parameter.borrow().name);
                                    }
                                }
                                {
                                    let self_sym = Rc::new(RefCell::new(Symbol::Variable {
                                        name: "self".to_string(),
                                        decl: None,
                                        parameter: None,
                                        is_var: true,
                                        ownership: OwnershipModifier::Strong,
                                    }));
                                    self.enter(
                                        self_sym,
                                        &Token::new(
                                            "self".to_string(),
                                            TokenType::Keyword {
                                                keyword: KeywordType::SelfKw,
                                            },
                                            Position {
                                                pos: 0,
                                                line: 0,
                                                col: 0,
                                                len: 0,
                                            },
                                            Rc::new("".to_string()),
                                        ),
                                    );
                                }
                                if let Some(return_type) = return_type {
                                    self.resolve_expression(return_type.clone());
                                }
                                self.resolve_function_body(body.clone());
                                self.leave_scope();
                            }
                        }
                        ProtocolMember::Property {
                            type_expression, ..
                        } => {
                            self.resolve_expression(type_expression.clone());
                        }
                        ProtocolMember::AssociatedType { constraints, .. } => {
                            for constraint in constraints {
                                self.resolve_expression(constraint.clone());
                            }
                        }
                        ProtocolMember::TypeAlias {
                            type_expression, ..
                        } => {
                            self.resolve_expression(type_expression.clone());
                        }
                        ProtocolMember::Subscript { .. } => {}
                        ProtocolMember::StaticVar {
                            type_expression, ..
                        } => {
                            self.resolve_expression(type_expression.clone());
                        }
                        ProtocolMember::Init { parameters, .. } => {
                            for parameter in parameters {
                                self.resolve_expression(parameter.borrow().type_expression.clone());
                            }
                        }
                    }
                }
                self.leave_scope();
            }
            Statement::ExpressionStatement { expression } => {
                self.resolve_expression(expression.clone())
            }
            Statement::Return {
                value: Some(value), ..
            } => self.resolve_expression(value.clone()),
            Statement::Yield {
                value: Some(value), ..
            } => self.resolve_expression(value.clone()),
            Statement::ExtensionDecl {
                type_name,
                body,
                where_clause,
                type_arguments,
                ..
            } => {
                if let Some(type_arguments) = type_arguments {
                    for ta in type_arguments {
                        self.resolve_expression(ta.clone());
                    }
                }
                if let Some(where_clause) = where_clause {
                    for req in where_clause {
                        self.resolve_where_requirement(req);
                    }
                }
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
                    let saved = self.current_scope.clone();
                    self.current_scope = Some(scope.clone());
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                    self.current_scope = saved;
                } else {
                    for stmt in body {
                        self.resolve_statement(stmt.clone());
                    }
                }
            }
            Statement::TypeAlias {
                type_expression,
                name: _,
                ..
            } => {
                self.resolve_expression(type_expression.clone());
            }
            Statement::Guard {
                condition,
                else_body,
                ..
            } => {
                self.resolve_expression(condition.clone());

                let case_bindings = {
                    let cond = condition.borrow();
                    if let Expression::Case { bindings, .. } = &*cond {
                        Some(bindings.clone())
                    } else {
                        None
                    }
                };

                for stmt in else_body {
                    self.resolve_statement(stmt.clone());
                }

                if let Some(bindings) = case_bindings {
                    Self::resolve_pattern_bindings(&bindings, self);
                }
            }
            Statement::Fallthrough { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. } => {}
            Statement::For {
                pattern,
                iterator,
                body,
                ..
            } => {
                self.enter_scope(None);
                Self::resolve_pattern_bindings_ref(pattern.as_ref(), self);
                self.resolve_expression(iterator.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Statement::Defer { body, .. } => {
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
            }
            Statement::Throw { exception, .. } => {
                self.resolve_expression(exception.clone());
            }
            Statement::ModuleDecl { body, scope, .. } => {
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
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
                    self.resolve_expression(operand.expression.clone());
                }
            }
            _ => {}
        }
    }

    fn resolve_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) {
        match &mut *body.borrow_mut() {
            FunctionBody::Statements(statements) => {
                for stmt in statements {
                    self.resolve_statement(stmt.clone());
                }
            }
            FunctionBody::Expression(expression) => self.resolve_expression(expression.clone()),
            FunctionBody::None => {}
        }
    }

    fn resolve_expression(&mut self, expr: Rc<RefCell<Expression>>) {
        match &mut *expr.borrow_mut() {
            Expression::Variable { name, symbol, .. } => {
                let is_type_name = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().get_symbol(&name.value))
                    .map(|sym| {
                        matches!(
                            &*sym.borrow(),
                            Symbol::Struct { .. }
                                | Symbol::Class { .. }
                                | Symbol::Enum { .. }
                                | Symbol::Protocol { .. }
                        )
                    })
                    .unwrap_or(false);
                if is_type_name {
                    return;
                }
                let is_generic_param = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().get_type(&name.value))
                    .is_some();
                if is_generic_param {
                    return;
                }

                // Detect implicit closure captures
                if !self.closure_capture_stack.is_empty() {
                    let (closure_scope, _) = &self.closure_capture_stack.last().unwrap().clone();
                    let var_name = name.value.clone();
                    if !closure_scope.borrow().name_table.contains_key(&var_name)
                        && self.resolve_symbol(name).is_ok()
                    {
                        let already_captured = self
                            .closure_capture_stack
                            .last()
                            .map_or(false, |(_, caps)| {
                                caps.iter().any(|c| c.name.value == var_name)
                            });
                        if !already_captured {
                            let is_var = match self.resolve_symbol(name) {
                                Ok(sym) => {
                                    matches!(&*sym.borrow(), Symbol::Variable { is_var: true, .. })
                                }
                                Err(_) => false,
                            };
                            if let Some((_, captures)) = self.closure_capture_stack.last_mut() {
                                captures.push(ClosureCapture {
                                    name: Box::new(Token::new(
                                        var_name.clone(),
                                        TokenType::Identifier,
                                        name.position,
                                        name.file.clone(),
                                    )),
                                    expression: None,
                                    is_var,
                                    ownership: OwnershipModifier::Strong,
                                });
                            }
                            let mut current = closure_scope.borrow().parent.clone();
                            while let Some(scope) = current {
                                if scope.borrow().name_table.contains_key(&var_name) {
                                    scope.borrow_mut().captured_by_closures.insert(var_name);
                                    break;
                                }
                                current = scope.borrow().parent.clone();
                            }
                        }
                        if let Ok(sym) = self.resolve_symbol(name) {
                            *symbol = Some(WeakSymbol(Rc::downgrade(&sym)));
                        }
                        return;
                    }
                }

                match self.resolve_symbol(name) {
                    Ok(sym) => *symbol = Some(WeakSymbol(Rc::downgrade(&sym))),
                    Err(_) => {
                        let has_self = self
                            .current_scope
                            .as_ref()
                            .and_then(|scope| scope.borrow().get_type("self"))
                            .is_some();
                        if has_self {
                            return;
                        }
                        let is_known_type = self
                            .current_scope
                            .as_ref()
                            .and_then(|scope| scope.borrow().get_type(&name.value))
                            .is_some();
                        if is_known_type {
                            return;
                        }
                        self.emit_error(
                            TrussDiagnosticCode::UndefinedVariable,
                            format!("Undefined variable '{}'", name.value),
                            name.as_ref(),
                        );
                    }
                }
            }
            Expression::Call {
                callee,
                type_parameters,
                parameters,
                overloads,
                ..
            } => {
                self.resolve_expression(callee.clone());
                if let Some(type_parameters) = type_parameters {
                    for type_parameter in type_parameters {
                        self.resolve_expression(type_parameter.clone())
                    }
                }
                for parameter in parameters {
                    self.resolve_expression(parameter.expression.clone())
                }
                if let Expression::Variable { name, .. } = &*callee.borrow() {
                    if let Some(scope) = self.current_scope.clone() {
                        let candidates = scope.borrow().get_all_symbols(&name.value);
                        if candidates.len() > 1 {
                            *overloads = candidates;
                        }
                    }
                }
            }
            Expression::MemberAccess { object, .. } => {
                let is_static_call = {
                    let obj = object.borrow();
                    if let Expression::Variable { name, .. } = &*obj {
                        self.current_scope
                            .as_ref()
                            .and_then(|scope| scope.borrow().get_symbol(&name.value))
                            .map(|sym| {
                                matches!(
                                    &*sym.borrow(),
                                    Symbol::Struct { .. }
                                        | Symbol::Class { .. }
                                        | Symbol::Enum { .. }
                                        | Symbol::Protocol { .. }
                                )
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    }
                };
                if !is_static_call {
                    self.resolve_expression(object.clone());
                }
            }
            Expression::AssociatedTypeAccess { object, .. } => {
                self.resolve_expression(object.clone());
            }
            Expression::TupleLiteral { elements, .. } => {
                for (_, element) in elements {
                    self.resolve_expression(element.clone());
                }
            }
            Expression::ArrayLiteral { elements, .. } => {
                for element in elements {
                    self.resolve_expression(element.clone());
                }
            }
            Expression::DictionaryLiteral { elements, .. } => {
                for (key, value) in elements {
                    self.resolve_expression(key.clone());
                    self.resolve_expression(value.clone());
                }
            }
            Expression::TupleType { elements, .. } => {
                for (_, element) in elements {
                    self.resolve_expression(element.clone());
                }
            }
            Expression::TupleIndexAccess { object, .. } => {
                self.resolve_expression(object.clone());
            }
            Expression::OptionalChain { object, .. } => {
                self.resolve_expression(object.clone());
            }
            Expression::SelfKeyword { token, symbol, .. } => {
                match self.resolve_symbol(token) {
                    Ok(sym) => {
                        *symbol = Some(WeakSymbol(Rc::downgrade(&sym)));
                    }
                    Err(_) => {
                        // self is not in scope; skip silently for implicit self support
                    }
                }
            }
            Expression::SuperKeyword { token, .. } => {
                let in_method = self
                    .current_scope
                    .as_ref()
                    .and_then(|scope| scope.borrow().get_symbol("self"))
                    .is_some();
                if !in_method {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("'super' is only available inside class methods"),
                        token.as_ref(),
                    );
                }
            }
            Expression::MethodReference {
                type_name,
                method_name,
                method_token,
                ..
            } => {
                if let Some(type_n) = type_name {
                    let found = self.current_scope.as_ref().map_or(false, |scope| {
                        let scope_ref = scope.borrow();
                        if let Some(sym) = scope_ref.get_symbol(type_n) {
                            let methods = match &*sym.borrow() {
                                Symbol::Struct { methods, .. }
                                | Symbol::Class { methods, .. }
                                | Symbol::Enum { methods, .. }
                                | Symbol::Protocol { methods, .. } => methods.clone(),
                                _ => return false,
                            };
                            methods
                                .iter()
                                .any(|m| m.borrow().name().ok().as_deref() == Some(method_name))
                        } else {
                            false
                        }
                    });
                    if !found {
                        self.emit_error(
                            TrussDiagnosticCode::UndefinedFunction,
                            format!("Method '{}' not found on type '{}'", method_name, type_n),
                            method_token.as_ref(),
                        );
                    }
                } else {
                    let found = self
                        .current_scope
                        .as_ref()
                        .and_then(|scope| scope.borrow().get_symbol(method_name));
                    if found.is_none() {
                        let in_method = self
                            .current_scope
                            .as_ref()
                            .and_then(|scope| scope.borrow().get_type("self"))
                            .is_some();
                        if !in_method {
                            self.emit_error(
                                TrussDiagnosticCode::UndefinedFunction,
                                format!("Function '{}' not found", method_name),
                                method_token.as_ref(),
                            );
                        }
                    }
                }
            }
            Expression::Binary { left, right, .. } => {
                self.resolve_expression(left.clone());
                self.resolve_expression(right.clone())
            }
            Expression::Unary { expression, .. } => self.resolve_expression(expression.clone()),
            Expression::Assignment { left, right, .. } => {
                self.resolve_expression(left.clone());
                self.resolve_expression(right.clone())
            }
            Expression::If {
                condition,
                then,
                else_,
                ..
            } => {
                self.resolve_expression(condition.clone());

                fn collect_case_bindings(expr: &Expression) -> Vec<Pattern> {
                    match expr {
                        Expression::Case { bindings, .. } => bindings.clone(),
                        Expression::Binary { left, right, .. } => {
                            let mut result = collect_case_bindings(&*left.borrow());
                            result.extend(collect_case_bindings(&*right.borrow()));
                            result
                        }
                        _ => Vec::new(),
                    }
                }

                let case_bindings = {
                    let cond = condition.borrow();
                    collect_case_bindings(&*cond)
                };

                if !case_bindings.is_empty() {
                    self.enter_scope(None);
                    Self::resolve_pattern_bindings(&case_bindings, self);
                    for stmt in then {
                        self.resolve_statement(stmt.clone());
                    }
                    self.leave_scope();
                } else {
                    for stmt in then {
                        self.resolve_statement(stmt.clone());
                    }
                }

                if let Some(else_) = else_ {
                    match else_ {
                        ElseBranch::Block(body) => {
                            for stmt in body {
                                self.resolve_statement(stmt.clone());
                            }
                        }
                        ElseBranch::If(if_expr) => {
                            self.resolve_expression(if_expr.clone());
                        }
                    }
                }
            }
            Expression::Case { expression, .. } => {
                self.resolve_expression(expression.clone());
            }
            Expression::Do {
                body,
                catch_clauses,
                finally_body,
                scope,
                ..
            } => {
                *scope = Some(self.enter_scope(None));
                for stmt in body.iter() {
                    self.resolve_statement(stmt.clone());
                }
                for clause in catch_clauses {
                    if let Some(pattern) = &clause.pattern {
                        Self::resolve_pattern_bindings_ref(pattern, self);
                    }
                    for stmt in &clause.body {
                        self.resolve_statement(stmt.clone());
                    }
                    if let Some(guard) = &clause.guard {
                        self.resolve_expression(guard.clone());
                    }
                }
                for stmt in finally_body.iter() {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Expression::Try { expression, .. } => {
                self.resolve_expression(expression.clone());
            }
            Expression::Match { value, cases, .. } => {
                self.resolve_expression(value.clone());
                for case in cases {
                    self.enter_scope(None);
                    for p in &case.patterns {
                        Self::resolve_pattern_bindings_ref(p.as_ref(), self);
                    }
                    for stmt in &case.body {
                        self.resolve_statement(stmt.clone());
                    }
                    if let Some(guard) = &case.guard {
                        self.resolve_expression(guard.clone());
                    }
                    self.leave_scope();
                }
            }
            Expression::AnyType { inner, .. } => {
                self.resolve_expression(inner.clone());
            }
            Expression::SomeType { inner, .. } => {
                self.resolve_expression(inner.clone());
            }
            Expression::InlineType { size, base, .. } => {
                if let Some(size) = size {
                    self.resolve_expression(size.clone());
                }
                self.resolve_expression(base.clone());
            }
            Expression::CompoundType { types, .. } => {
                for t in types {
                    self.resolve_expression(t.clone());
                }
            }
            Expression::Closure {
                captures,
                parameters,
                return_type,
                body,
                scope,
                ..
            } => {
                *scope = Some(self.enter_scope(None));
                let closure_scope = scope.as_ref().unwrap().clone();

                // Register explicit captures
                let mut explicit_capture_names = Vec::new();
                for cap in captures.iter() {
                    let cap_name = cap.name.value.clone();
                    explicit_capture_names.push(cap_name.clone());
                    if cap_name != "_" {
                        let symbol = Rc::new(RefCell::new(Symbol::Variable {
                            name: cap_name.clone(),
                            decl: None,
                            parameter: None,
                            is_var: cap.is_var,
                            ownership: OwnershipModifier::Strong,
                        }));
                        self.enter(symbol, &cap.name);
                    }
                    if let Some(expr) = &cap.expression {
                        self.resolve_expression(expr.clone());
                    }
                }

                // Push closure capture context
                let collected_captures: Vec<ClosureCapture> = captures.clone();
                self.closure_capture_stack
                    .push((closure_scope.clone(), collected_captures));

                for param in parameters {
                    let name = param.borrow().name.value.clone();
                    if name != "_" {
                        let symbol = Rc::new(RefCell::new(Symbol::Variable {
                            name: name.clone(),
                            decl: None,
                            parameter: None,
                            is_var: true,
                            ownership: OwnershipModifier::Strong,
                        }));
                        self.enter(symbol, &param.borrow().name);
                    }
                    if let Some(type_annotation) = &param.borrow().type_annotation {
                        self.resolve_expression(type_annotation.clone());
                    }
                }
                if let Some(ret) = return_type {
                    self.resolve_expression(ret.clone());
                }
                for stmt in body.iter() {
                    self.resolve_statement(stmt.clone());
                }
                let max_shorthand = {
                    let mut max = None;
                    Self::find_max_shorthand(body, &mut max);
                    max
                };
                if let Some(max_idx) = max_shorthand {
                    for i in 0..=max_idx {
                        let name = format!("${}", i);
                        let sym = Rc::new(RefCell::new(Symbol::Variable {
                            name: name.clone(),
                            decl: None,
                            parameter: None,
                            is_var: true,
                            ownership: OwnershipModifier::Strong,
                        }));
                        self.enter(
                            sym,
                            &Token::new(
                                name,
                                TokenType::Identifier,
                                Position {
                                    pos: 0,
                                    line: 0,
                                    col: 0,
                                    len: 0,
                                },
                                Rc::new("".to_string()),
                            ),
                        );
                    }
                }

                // Pop closure capture context and backfill captures
                if let Some((_, final_captures)) = self.closure_capture_stack.pop() {
                    *captures = final_captures;
                }

                self.leave_scope();
            }
            Expression::ClosureType { param_types, .. } => {
                for pt in param_types {
                    self.resolve_expression(pt.clone());
                }
            }
            Expression::FunctionType { param_types, .. } => {
                for pt in param_types {
                    self.resolve_expression(pt.clone());
                }
            }
            Expression::OptionalType { inner, .. } => {
                self.resolve_expression(inner.clone());
            }
            Expression::ArrayType { inner, .. } => {
                self.resolve_expression(inner.clone());
            }
            Expression::DictionaryType { key, value, .. } => {
                self.resolve_expression(key.clone());
                self.resolve_expression(value.clone());
            }
            _ => {}
        }
    }

    fn find_max_shorthand(stmts: &[Rc<RefCell<Statement>>], max: &mut Option<u32>) {
        for stmt in stmts {
            match &*stmt.borrow() {
                Statement::ExpressionStatement { expression } => {
                    Self::find_shorthand_in_expr(expression, max);
                }
                Statement::Return {
                    value: Some(val), ..
                } => {
                    Self::find_shorthand_in_expr(val, max);
                }
                Statement::Yield {
                    value: Some(val), ..
                } => {
                    Self::find_shorthand_in_expr(val, max);
                }
                _ => {}
            }
        }
    }

    fn find_shorthand_in_expr(expr: &Rc<RefCell<Expression>>, max: &mut Option<u32>) {
        match &*expr.borrow() {
            Expression::ShorthandArgument { index, .. } => match max {
                Some(m) => {
                    if *index > *m {
                        *max = Some(*index);
                    }
                }
                None => *max = Some(*index),
            },
            Expression::Binary { left, right, .. } => {
                Self::find_shorthand_in_expr(left, max);
                Self::find_shorthand_in_expr(right, max);
            }
            Expression::Unary { expression, .. } => {
                Self::find_shorthand_in_expr(expression, max);
            }
            Expression::Call { parameters, .. } => {
                for param in parameters {
                    Self::find_shorthand_in_expr(&param.expression, max);
                }
            }
            _ => {}
        }
    }

    fn resolve_pattern_bindings_ref(pattern: &Pattern, resolver: &mut SymbolResolver) {
        match pattern {
            Pattern::Identifier(name) => {
                if name.value != "_" {
                    let sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: name.value.clone(),
                        decl: None,
                        parameter: None,
                        is_var: true,
                        ownership: OwnershipModifier::Strong,
                    }));
                    resolver.enter(sym, name);
                }
            }
            Pattern::Tuple(items) => {
                Self::resolve_pattern_bindings(items, resolver);
            }
            Pattern::Ignore => {}
            Pattern::ValueBinding(inner) => {
                Self::resolve_pattern_bindings_ref(inner.as_ref(), resolver);
            }
            Pattern::EnumCase { bindings, .. } => {
                Self::resolve_pattern_bindings(bindings, resolver);
            }
            Pattern::Expr(_) => {}
        }
    }

    fn resolve_pattern_bindings(bindings: &[Pattern], resolver: &mut SymbolResolver) {
        for binding in bindings {
            match binding {
                Pattern::Identifier(name) => {
                    if name.value != "_" {
                        let sym = Rc::new(RefCell::new(Symbol::Variable {
                            name: name.value.clone(),
                            decl: None,
                            parameter: None,
                            is_var: true,
                            ownership: OwnershipModifier::Strong,
                        }));
                        resolver.enter(sym, name);
                    }
                }
                Pattern::Tuple(items) => {
                    Self::resolve_pattern_bindings(items, resolver);
                }
                Pattern::Ignore => {}
                Pattern::ValueBinding(inner) => {
                    Self::resolve_pattern_bindings(&[*(inner.clone())], resolver);
                }
                Pattern::EnumCase { bindings, .. } => {
                    Self::resolve_pattern_bindings(bindings, resolver);
                }
                Pattern::Expr(_) => {}
            }
        }
    }

    fn resolve_conformance(&mut self, conformance: Rc<RefCell<Expression>>) {
        let conformance_kind = match &*conformance.borrow() {
            Expression::Type { .. } => 0u8,
            Expression::CompoundType { .. } => 1u8,
            _ => 2u8,
        };
        if conformance_kind == 2u8 {
            self.resolve_expression(conformance.clone());
            return;
        }
        match &*conformance.borrow() {
            Expression::Type {
                name,
                type_parameters,
                ..
            } => {
                if let Some(current_scope) = self.current_scope.clone()
                    && let Ok(Some(_)) =
                        self.resolve_symbol_in_scope(name.value.clone(), current_scope.clone())
                {
                }
                if let Some(type_parameters) = type_parameters {
                    for tp in type_parameters {
                        self.resolve_expression(tp.clone());
                    }
                }
            }
            Expression::CompoundType { types, .. } => {
                for t in types {
                    self.resolve_conformance(t.clone());
                }
            }
            _ => {}
        }
    }

    fn resolve_variable_pattern(
        pattern: &Pattern,
        decl: Rc<RefCell<Statement>>,
        is_var: bool,
        resolver: &mut SymbolResolver,
    ) {
        match pattern {
            Pattern::Identifier(name) => {
                if name.value != "_" {
                    let sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: name.value.clone(),
                        decl: Some(decl),
                        parameter: None,
                        is_var,
                        ownership: OwnershipModifier::Strong,
                    }));
                    resolver.enter(sym, name);
                }
            }
            Pattern::Tuple(items) => {
                for item in items {
                    Self::resolve_variable_pattern(item, decl.clone(), is_var, resolver);
                }
            }
            Pattern::Ignore => {}
            Pattern::ValueBinding(inner) => {
                Self::resolve_variable_pattern(inner.as_ref(), decl, is_var, resolver);
            }
            Pattern::EnumCase { bindings, .. } => {
                for binding in bindings {
                    Self::resolve_variable_pattern(binding, decl.clone(), is_var, resolver);
                }
            }
            Pattern::Expr(_) => {}
        }
    }

    fn resolve_where_requirement(&mut self, req: &WhereRequirement) {
        match &req.kind {
            WhereRequirementKind::Conformance {
                type_expr,
                constraint,
            } => {
                self.resolve_expression(type_expr.clone());
                self.resolve_expression(constraint.clone());
            }
            WhereRequirementKind::Equality { left, right } => {
                self.resolve_expression(left.clone());
                self.resolve_expression(right.clone());
            }
        }
    }

    fn enter(&mut self, symbol: Rc<RefCell<Symbol>>, token: &Token) {
        if let Some(scope) = self.current_scope.clone() {
            scope.borrow_mut().set_symbol(symbol.clone());
        } else {
            let name = match symbol.borrow().name() {
                Ok(n) => n,
                Err(_) => return,
            };
            self.emit_error(
                TrussDiagnosticCode::SymbolError,
                format!("No avaliable scope for symbol '{}'", name),
                token,
            );
        }
    }

    fn resolve_symbol(&mut self, token: &Token) -> Result<Rc<RefCell<Symbol>>, ()> {
        let name = token.value.clone();
        if let Some(current_scope) = self.current_scope.clone()
            && let Some(symbol) = self.resolve_symbol_in_scope(name.clone(), current_scope)?
        {
            Ok(symbol)
        } else {
            self.emit_error(
                TrussDiagnosticCode::SymbolError,
                format!("No avaliable scope for symbol '{}'", name),
                token,
            );
            Err(())
        }
    }

    fn resolve_symbol_in_scope(
        &mut self,
        name: String,
        scope: Rc<RefCell<Scope>>,
    ) -> Result<Option<Rc<RefCell<Symbol>>>, ()> {
        if let Some(symbol) = scope.borrow().get_symbol(&name) {
            Ok(Some(symbol))
        } else if let Some(parent) = scope.borrow().parent.clone() {
            self.resolve_symbol_in_scope(name, parent)
        } else {
            Ok(None)
        }
    }

    pub fn enter_scope(&mut self, scope: Option<Rc<RefCell<Scope>>>) -> Rc<RefCell<Scope>> {
        let sc = if let Some(scope) = scope {
            scope
        } else {
            Rc::new(RefCell::new(Scope::new(self.current_scope.clone())))
        };
        self.current_scope = Some(sc.clone());
        sc
    }

    fn leave_scope(&mut self) {
        if let Some(scope) = self.current_scope.clone() {
            self.current_scope = scope.borrow().parent.clone();
        }
    }

    fn has_modifier(modifiers: &[Modifier], target: ModifierType) -> bool {
        modifiers.iter().any(|m| m.ty == target)
    }

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>, token: &Token) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg));
        self.engine.borrow_mut().emit(diag);
    }

    fn validate_main_attribute(&self, stmts: &[Rc<RefCell<Statement>>]) {
        let mut main_count = 0u32;
        for stmt in stmts {
            self.count_main_in_stmts(stmt, false, &mut main_count);
        }
    }

    fn count_main_in_stmts(
        &self,
        stmt: &Rc<RefCell<Statement>>,
        inside_module: bool,
        count: &mut u32,
    ) {
        let s = stmt.borrow();
        match &*s {
            Statement::FunctionDecl {
                attributes, name, ..
            } => {
                if attributes.iter().any(|a| a.name == "main") {
                    if inside_module {
                        self.emit_error(
                            TrussDiagnosticCode::MainInsideModule,
                            "'#[main]' attribute is not allowed inside a module",
                            name,
                        );
                    } else {
                        *count += 1;
                        if *count > 1 {
                            self.emit_error(
                                TrussDiagnosticCode::DuplicateMainAttribute,
                                "Multiple functions with '#[main]' attribute",
                                name,
                            );
                        }
                    }
                }
            }
            Statement::ModuleDecl { body, .. } => {
                for child in body {
                    self.count_main_in_stmts(child, true, count);
                }
            }
            _ => {}
        }
    }
}
