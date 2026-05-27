use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        expression::Expression,
        node::Program,
        statement::{AccessorKind, FunctionBody, Pattern, Statement},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    krate::{Crate, Module},
    lexer::token::Token,
    scope::Scope,
    symbol::{Symbol, WeakSymbol},
};

#[derive(Debug)]
pub struct SymbolResolver {
    pub krate: Rc<RefCell<Crate>>,
    current_module: Option<Rc<RefCell<Module>>>,
    current_scope: Option<Rc<RefCell<Scope>>>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
}
impl SymbolResolver {
    pub fn new(krate: Rc<RefCell<Crate>>, engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        Self {
            krate,
            current_module: None,
            current_scope: None,
            engine,
        }
    }

    pub fn resolve(&mut self, program: &Program, module_name: String) -> Rc<RefCell<Module>> {
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
                name, body, scope, ..
            } => {
                let struct_symbol = Rc::new(RefCell::new(Symbol::Struct {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    fields: vec![],
                    methods: vec![],
                    constructors: vec![],
                    destrcutor: None,
                }));
                self.enter(struct_symbol.clone(), name);
                let Symbol::Struct {
                    fields,
                    methods,
                    constructors,
                    destrcutor,
                    ..
                } = &mut *struct_symbol.borrow_mut()
                else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                {
                    let self_sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: "self".to_string(),
                        decl: None,
                        parameter: None,
                    }));
                    self.enter(self_sym, name);
                }
                for field_stmt in body {
                    if let Statement::VariableDecl {
                        name: field_name, ..
                    } = &*field_stmt.borrow()
                    {
                        let field_symbol = Rc::new(RefCell::new(Symbol::StructField {
                            name: field_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&struct_symbol)),
                            decl: Some(field_stmt.clone()),
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
                    } else {
                        self.register_symbols(field_stmt.clone());
                    }
                }
                self.leave_scope();
            }
            Statement::ClassDecl {
                name, body, scope, ..
            } => {
                let class_symbol = Rc::new(RefCell::new(Symbol::Class {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    fields: vec![],
                    methods: vec![],
                    constructors: vec![],
                    destrcutor: None,
                    superclass: None,
                }));
                self.enter(class_symbol.clone(), name);
                let Symbol::Class {
                    fields,
                    methods,
                    constructors,
                    destrcutor,
                    ..
                } = &mut *class_symbol.borrow_mut()
                else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
                {
                    let self_sym = Rc::new(RefCell::new(Symbol::Variable {
                        name: "self".to_string(),
                        decl: None,
                        parameter: None,
                    }));
                    self.enter(self_sym, name);
                }
                for field_stmt in body {
                    if let Statement::VariableDecl {
                        name: field_name, ..
                    } = &*field_stmt.borrow()
                    {
                        let field_symbol = Rc::new(RefCell::new(Symbol::ClassField {
                            name: field_name.value.clone(),
                            parent: WeakSymbol(Rc::downgrade(&class_symbol)),
                            decl: Some(field_stmt.clone()),
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
                    } else {
                        self.register_symbols(field_stmt.clone());
                    }
                }
                self.leave_scope();
            }
            Statement::EnumDecl {
                name, cases: ast_cases, body, scope, ..
            } => {
                let enum_symbol = Rc::new(RefCell::new(Symbol::Enum {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                    cases: vec![],
                    methods: vec![],
                }));
                self.enter(enum_symbol.clone(), name);
                let Symbol::Enum { cases, methods, .. } = &mut *enum_symbol.borrow_mut()
                else {
                    return;
                };

                *scope = Some(self.enter_scope(None));
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
            Statement::ProtocolDecl {
                name, ..
            } => {
                let symbol = Rc::new(RefCell::new(Symbol::Protocol {
                    name: name.value.clone(),
                    decl: stmt.clone(),
                }));
                self.enter(symbol, name);
            }
            _ => {}
        }
    }

    fn register_function_symbols_in_expr(&mut self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Block { statements, .. } = &*expr.borrow() {
            for stmt in statements {
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
            Statement::VariableDecl {
                name, initializer, accessors, ..
            } => {
                if name.value != "_" {
                    let symbol = Rc::new(RefCell::new(Symbol::Variable {
                        name: name.value.clone(),
                        decl: Some(stmt.clone()),
                        parameter: None,
                    }));
                    self.enter(symbol, name);
                }
                if let Some(initializer) = initializer {
                    self.resolve_expression(initializer.clone());
                }
                if !accessors.is_empty() {
                    for accessor in accessors {
                        let accessor_scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
                        let saved = self.current_scope.clone();
                        self.current_scope = Some(accessor_scope.clone());
                        let backing_sym = Rc::new(RefCell::new(Symbol::Variable {
                            name: format!("_{}", name.value),
                            decl: None,
                            parameter: None,
                        }));
                        accessor_scope.borrow_mut().set_symbol(backing_sym);
                        if let Some(param) = &accessor.parameter {
                            let param_sym = Rc::new(RefCell::new(Symbol::Variable {
                                name: param.value.clone(),
                                decl: None,
                                parameter: None,
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
            Statement::StructDecl { body, scope, .. } => {
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Statement::ClassDecl { body, scope, superclass, conformances, .. } => {
                if let Some(superclass_expr) = superclass {
                    if let Expression::Type { name: super_name, .. } = &*superclass_expr.borrow() {
                        let _ = self.resolve_symbol(super_name);
                    }
                }
                for conformance in conformances {
                    if let Expression::Type { name, .. } = &*conformance.borrow() {
                        let _ = self.resolve_symbol(name);
                    }
                }
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
            Statement::EnumDecl { body, scope, .. } => {
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
            Statement::ProtocolDecl { conformances, .. } => {
                for conformance in conformances {
                    if let Expression::Type { name, .. } = &*conformance.borrow() {
                        let _ = self.resolve_symbol(name);
                    }
                }
            }
            Statement::ExpressionStatement { expression } => {
                self.resolve_expression(expression.clone())
            }
            Statement::Return {
                value: Some(value), ..
            } => self.resolve_expression(value.clone()),
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
            Expression::Block { statements, scope } => {
                self.enter_scope(scope.clone());
                for stmt in statements {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
            }
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
            }
            Expression::MemberAccess { object, .. } => {
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
            Expression::SelfKeyword { token, symbol, .. } => {
                match self.resolve_symbol(token) {
                    Ok(sym) => *symbol = Some(WeakSymbol(Rc::downgrade(&sym))),
                    Err(_) => {
                        self.emit_error(
                            TrussDiagnosticCode::UndefinedVariable,
                            format!("'self' is only available inside methods"),
                            token.as_ref(),
                        );
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
                    if let Expression::Block {
                        statements,
                        scope,
                    } = &mut *then.borrow_mut()
                    {
                        if let Some(existing_scope) = scope.as_ref() {
                            self.enter_scope(Some(existing_scope.clone()));
                        } else {
                            let new_scope = self.enter_scope(None);
                            *scope = Some(new_scope);
                        }
                        Self::resolve_pattern_bindings(&case_bindings, self);
                        for stmt in statements {
                            self.resolve_statement(stmt.clone());
                        }
                        self.leave_scope();
                    } else {
                        self.resolve_expression(then.clone());
                    }
                } else {
                    self.resolve_expression(then.clone());
                }

                if let Some(else_) = else_ {
                    self.resolve_expression(else_.clone());
                }
            }
            Expression::Case { expression, .. } => {
                self.resolve_expression(expression.clone());
            }
            _ => {}
        }
    }

    fn resolve_pattern_bindings(
        bindings: &[Pattern],
        resolver: &mut SymbolResolver,
    ) {
        for binding in bindings {
            match binding {
                Pattern::Identifier(name) => {
                    if name.value != "_" {
                        let sym = Rc::new(RefCell::new(Symbol::Variable {
                            name: name.value.clone(),
                            decl: None,
                            parameter: None,
                        }));
                        resolver.enter(sym, name);
                    }
                }
                Pattern::Tuple(items) => {
                    Self::resolve_pattern_bindings(items, resolver);
                }
                Pattern::Ignore => {}
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
