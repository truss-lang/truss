use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        expression::{ElseBranch, Expression},
        node::Program,
        statement::{
            AccessorKind, FunctionBody, ImportKind, Pattern, ProtocolMember, Statement,
            WhereRequirement, WhereRequirementKind,
        },
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    krate::{Crate, Module},
    lexer::token::{Position, Token, TokenType},
    scope::Scope,
    symbol::{Symbol, WeakSymbol},
    types::Type,
};

#[derive(Debug)]
pub struct SymbolResolver {
    pub krate: Rc<RefCell<Crate>>,
    current_module: Option<Rc<RefCell<Module>>>,
    current_scope: Option<Rc<RefCell<Scope>>>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
    root_module_name: String,
}
impl SymbolResolver {
    pub fn new(krate: Rc<RefCell<Crate>>, engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        Self {
            krate,
            current_module: None,
            current_scope: None,
            engine,
            root_module_name: String::new(),
        }
    }

    pub fn resolve(&mut self, program: &Program, module_name: String) -> Rc<RefCell<Module>> {
        self.root_module_name = module_name.clone();
        let module = Rc::new(RefCell::new(Module::new(module_name.clone())));
        self.krate
            .borrow_mut()
            .modules
            .insert(module_name, module.clone());
        self.current_module = Some(module.clone());
        let scope = self.enter_scope(None);
        self.current_module.as_ref().unwrap().borrow_mut().scope = Some(scope.clone());

        for stmt in &program.statements {
            self.register_symbols(stmt.clone());
        }

        for stmt in &program.statements {
            self.resolve_statement(stmt.clone());
        }
        self.leave_scope();
        module
    }

    fn register_symbols(&mut self, stmt: Rc<RefCell<Statement>>) {
        match &mut *stmt.borrow_mut() {
            Statement::FunctionDecl { name, body, .. } => {
                let symbol = Rc::new(RefCell::new(Symbol::Function {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                }));
                self.enter(symbol, name);
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
            }
            Statement::StructDecl {
                name,
                body,
                scope,
                generic_parameters,
                ..
            } => {
                let struct_symbol = Rc::new(RefCell::new(Symbol::Struct {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    properties: vec![],
                    methods: vec![],
                    constructors: vec![],
                    destrcutor: None,
                    subscripts: vec![],
                }));
                self.enter(struct_symbol.clone(), name);
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
                        let gp_type =
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())));
                        scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(gp.name.value.clone(), gp_type);
                    }
                }
                {
                    let self_sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: "self".to_string(),
                        decl: None,
                        parameter: None,
                        is_var: true,
                    }));
                    self.enter(self_sym, name);
                }
                for field_stmt in body {
                    if let Statement::VariableDecl {
                        name: field_name,
                        token: field_token,
                        ..
                    } = &*field_stmt.borrow()
                    {
                        let is_var = field_token.value == "var";
                        let field_symbol = Rc::new(RefCell::new(Symbol::StructProperty {
                            name: field_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&struct_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_var,
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
                            if let Statement::SubscriptDecl { parameters, accessors, .. } = &*stmt {
                                for accessor in accessors {
                                    let acc_scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
                                    let saved = self.current_scope.clone();
                                    self.current_scope = Some(acc_scope.clone());
                                    for param in parameters {
                                        let param_name = param.borrow().name.value.clone();
                                        if param_name != "_" {
                                            let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                                name: param_name,
                                                decl: None,
                                                parameter: Some(param.clone()),
                                                is_var: true,
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
                                        }));
                                        if let Some(scope) = self.current_scope.clone() {
                                            scope.borrow_mut().set_symbol(param_sym);
                                        }
                                    }
                                    for s in &accessor.body {
                                        self.resolve_statement(s.clone());
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
                name,
                body,
                scope,
                generic_parameters,
                ..
            } => {
                let class_symbol = Rc::new(RefCell::new(Symbol::Class {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    properties: vec![],
                    methods: vec![],
                    constructors: vec![],
                    destrcutor: None,
                    superclass: None,
                    subscripts: vec![],
                }));
                self.enter(class_symbol.clone(), name);
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
                        let gp_type =
                            Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())));
                        scope
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set_type(gp.name.value.clone(), gp_type);
                    }
                }
                {
                    let self_sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: "self".to_string(),
                        decl: None,
                        parameter: None,
                        is_var: true,
                    }));
                    self.enter(self_sym, name);
                }
                for field_stmt in body {
                    if let Statement::VariableDecl {
                        name: field_name,
                        token: field_token,
                        ..
                    } = &*field_stmt.borrow()
                    {
                        let is_var = field_token.value == "var";
                        let field_symbol = Rc::new(RefCell::new(Symbol::ClassProperty {
                            name: field_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
                            is_var,
                        }));
                        fields.push(field_symbol.clone());
                        self.enter(field_symbol, field_name);
                    } else if let Statement::FunctionDecl {
                        name: method_name, ..
                    } = &*field_stmt.borrow()
                    {
                        let method_symbol = Rc::new(RefCell::new(Symbol::ClassMethod {
                            name: method_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
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
                        let init_symbol = Rc::new(RefCell::new(Symbol::ClassMethod {
                            name: "init".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
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
                        let deinit_symbol = Rc::new(RefCell::new(Symbol::ClassMethod {
                            name: "deinit".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
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
                        let sub_sym = Rc::new(RefCell::new(Symbol::ClassSubscript {
                            name: "subscript".to_string(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
                        }));
                        _subscripts.push(sub_sym.clone());
                        {
                            let stmt = field_stmt.borrow();
                            if let Statement::SubscriptDecl { parameters, accessors, .. } = &*stmt {
                                for accessor in accessors {
                                    let acc_scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
                                    let saved = self.current_scope.clone();
                                    self.current_scope = Some(acc_scope.clone());
                                    for param in parameters {
                                        let param_name = param.borrow().name.value.clone();
                                        if param_name != "_" {
                                            let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                                name: param_name,
                                                decl: None,
                                                parameter: Some(param.clone()),
                                                is_var: true,
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
                                        }));
                                        if let Some(scope) = self.current_scope.clone() {
                                            scope.borrow_mut().set_symbol(param_sym);
                                        }
                                    }
                                    for s in &accessor.body {
                                        self.resolve_statement(s.clone());
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
                name,
                cases: ast_cases,
                body,
                scope,
                generic_parameters,
                ..
            } => {
                let enum_symbol = Rc::new(RefCell::new(Symbol::Enum {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    cases: vec![],
                    methods: vec![],
                }));
                self.enter(enum_symbol.clone(), name);
                let Symbol::Enum { cases, methods, .. } = &mut *enum_symbol.borrow_mut() else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                for gp in generic_parameters {
                    let gp_type = Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())));
                    scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
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
                ..
            } => {
                let protocol_symbol = Rc::new(RefCell::new(Symbol::Protocol {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    methods: vec![],
                    properties: vec![],
                    subscripts: vec![],
                }));
                self.enter(protocol_symbol.clone(), name);
                let Symbol::Protocol {
                    methods,
                    properties,
                    subscripts: _subscripts,
                    ..
                } = &mut *protocol_symbol.borrow_mut()
                else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                for gp in generic_parameters {
                    let gp_type = Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())));
                    scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                }
                for member in members {
                    match member {
                        ProtocolMember::Method { decl, .. } => {
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
                            name: prop_name, ..
                        } => {
                            let prop_symbol = Rc::new(RefCell::new(Symbol::ProtocolProperty {
                                name: prop_name.value.clone(),
                                parent: WeakSymbol(Rc::downgrade(&protocol_symbol)),
                                decl: None,
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
                        ProtocolMember::Subscript { .. } => {}
                    }
                }
                self.leave_scope();
            }
            Statement::ExtensionDecl {
                type_name, body, ..
            } => {
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
                    Symbol::Protocol { methods, subscripts: _subscripts, .. } => {
                        for field_stmt in body {
                            if let Statement::FunctionDecl {
                                name: method_name, ..
                            } = &*field_stmt.borrow()
                            {
                                let method_symbol = Rc::new(RefCell::new(Symbol::ProtocolMethod {
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
                            } else if let Statement::SubscriptDecl { .. } = &*field_stmt.borrow() {
                                let sub_sym = Rc::new(RefCell::new(Symbol::ProtocolSubscript {
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
                self.krate
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
            Statement::ImportDecl { path, kind, token } => match kind {
                ImportKind::Module => {
                    let module_path = path.join(".");
                    let result = { self.krate.borrow().modules.get(&module_path).cloned() };
                    if let Some(module) = result {
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
                    let found_symbol = {
                        self.krate.borrow().modules.get(&module_path).and_then(|m| {
                            m.borrow()
                                .scope
                                .clone()
                                .and_then(|scope| scope.borrow().get_symbol(&member_name))
                        })
                    };
                    if let Some(symbol) = found_symbol {
                        self.enter(symbol, token.as_ref());
                    } else {
                        let exists = self.krate.borrow().modules.contains_key(&module_path);
                        if exists {
                            self.emit_error(
                                TrussDiagnosticCode::SymbolError,
                                format!(
                                    "Symbol '{}' not found in module '{}'",
                                    member_name, module_path
                                ),
                                token.as_ref(),
                            );
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::SymbolError,
                                format!("Module '{}' not found", module_path),
                                token.as_ref(),
                            );
                        }
                    }
                }
                ImportKind::Wildcard => {
                    let module_path = path.join(".");
                    let module = self.krate.borrow().modules.get(&module_path).cloned();
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
            },
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
                *scope = Some(self.enter_scope(None));
                for gp in generic_parameters {
                    let gp_type = Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())));
                    scope
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set_type(gp.name.value.clone(), gp_type);
                }
                for parameter in parameters {
                    let name = parameter.borrow().name.value.clone();
                    if name != "_" {
                        let symbol = Rc::new(RefCell::new(Symbol::Variable {
                            name,
                            decl: None,
                            parameter: Some(parameter.clone()),
                            is_var: true,
                        }));
                        self.enter(symbol, &parameter.borrow().name);
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
                token: var_token,
                initializer,
                accessors,
                ..
            } => {
                if name.value != "_" {
                    let is_var = var_token.value == "var";
                    let symbol = Rc::new(RefCell::new(Symbol::Variable {
                        name: name.value.clone(),
                        decl: Some(stmt.clone()),
                        parameter: None,
                        is_var,
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
                        }));
                        accessor_scope.borrow_mut().set_symbol(backing_sym);
                        if let Some(param) = &accessor.parameter {
                            let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                name: param.value.clone(),
                                decl: None,
                                parameter: None,
                                is_var: true,
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
                body,
                scope,
                conformances,
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
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Statement::ClassDecl {
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
                ..
            } => {
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
                        }));
                        self.enter(symbol, &parameter.borrow().name);
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
                                        }));
                                        self.enter(symbol, &parameter.borrow().name);
                                    }
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
            Statement::ExtensionDecl {
                type_name,
                body,
                where_clause,
                ..
            } => {
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
            Statement::Fallthrough { .. } | Statement::Break { .. } => {}
            Statement::Defer { body, .. } => {
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
            }
            Statement::ModuleDecl { body, scope, .. } => {
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
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
            Expression::Variable { name, symbol, .. } => match self.resolve_symbol(name) {
                Ok(sym) => *symbol = Some(WeakSymbol(Rc::downgrade(&sym))),
                Err(_) => {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("Undefined variable '{}'", name.value),
                        name.as_ref(),
                    );
                }
            },
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
                self.resolve_expression(object.clone());
            }
            Expression::AssociatedTypeAccess { object, .. } => {
                self.resolve_expression(object.clone());
            }
            Expression::TupleLiteral { elements, .. } => {
                for (_, element) in elements {
                    self.resolve_expression(element.clone());
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
            Expression::SelfKeyword { token, symbol, .. } => match self.resolve_symbol(token) {
                Ok(sym) => *symbol = Some(WeakSymbol(Rc::downgrade(&sym))),
                Err(_) => {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("'self' is only available inside methods"),
                        token.as_ref(),
                    );
                }
            },
            Expression::SuperKeyword { token, .. } => {
                let in_method = self.current_scope.as_ref().and_then(|scope| {
                    scope.borrow().get_symbol("self")
                }).is_some();
                if !in_method {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("'super' is only available inside class methods"),
                        token.as_ref(),
                    );
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

                let case_bindings = {
                    let cond = condition.borrow();
                    if let Expression::Case { bindings, .. } = &*cond {
                        bindings.clone()
                    } else {
                        Vec::new()
                    }
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
            Expression::CompoundType { types, .. } => {
                for t in types {
                    self.resolve_expression(t.clone());
                }
            }
            Expression::Closure {
                parameters,
                return_type,
                body,
                scope,
                ..
            } => {
                *scope = Some(self.enter_scope(None));
                for param in parameters {
                    let name = param.borrow().name.value.clone();
                    if name != "_" {
                        let symbol = Rc::new(RefCell::new(Symbol::Variable {
                            name: name.clone(),
                            decl: None,
                            parameter: None,
                            is_var: true,
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
                self.leave_scope();
            }
            Expression::FunctionType { param_types, .. } => {
                for pt in param_types {
                    self.resolve_expression(pt.clone());
                }
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
                _ => {}
            }
        }
    }

    fn find_shorthand_in_expr(expr: &Rc<RefCell<Expression>>, max: &mut Option<u32>) {
        match &*expr.borrow() {
            Expression::ShorthandArgument { index, .. } => {
                match max {
                    Some(m) => {
                        if *index > *m {
                            *max = Some(*index);
                        }
                    }
                    None => *max = Some(*index),
                }
            }
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
        match &*conformance.borrow() {
            Expression::Type {
                name,
                type_parameters,
                ..
            } => {
                let _ = self.resolve_symbol(name);
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
            _ => {
                self.resolve_expression(conformance.clone());
            }
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

    fn enter_scope(&mut self, scope: Option<Rc<RefCell<Scope>>>) -> Rc<RefCell<Scope>> {
        let sc = if let Some(scope) = scope {
            scope
        } else {
            Rc::new(RefCell::new(Scope::new(self.current_scope.clone())))
        };
        self.current_scope = Some(sc.clone());
        sc
    }

    fn leave_scope(&mut self) {
        self.current_scope = self.current_scope.clone().unwrap().borrow().parent.clone();
    }

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>, token: &Token) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg));
        self.engine.borrow_mut().emit(diag);
    }
}
