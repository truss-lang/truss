use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        expression::Expression,
        node::Program,
        statement::{FunctionBody, Statement},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    id::{ModuleId, SymbolId},
    krate::{Crate, Module},
    lexer::token::Token,
    scope::Scope,
    symbol::Symbol,
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

    pub fn resolve(&mut self, program: &Program, module_name: String) -> ModuleId {
        let id = ModuleId {
            id: self.krate.borrow().modules.len(),
        };
        let module = Rc::new(RefCell::new(Module::new(module_name.clone(), id)));
        self.krate.borrow_mut().modules.insert(id, module.clone());
        self.krate
            .borrow_mut()
            .name_to_modules
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
        id
    }

    fn register_symbols(&mut self, stmt: Rc<RefCell<Statement>>) {
        match &mut *stmt.borrow_mut() {
            Statement::FunctionDecl { name, body, .. } => {
                let id = self.get_symbol_id();
                let symbol = Rc::new(Symbol::Function {
                    name: name.value.clone(),
                    id,
                    decl: stmt.clone(),
                });
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
                let struct_id = self.get_symbol_id();
                let struct_symbol = Rc::new(Symbol::Struct {
                    name: name.value.clone(),
                    id: struct_id,
                    decl: stmt.clone(),
                });
                self.enter(struct_symbol, name);

                *scope = Some(self.enter_scope(None));
                for field_stmt in body {
                    if let Statement::VariableDecl {
                        name: field_name, ..
                    } = &*field_stmt.borrow()
                    {
                        let field_id = self.get_symbol_id();
                        let field_symbol = Rc::new(Symbol::StructField {
                            name: field_name.value.clone(),
                            id: field_id,
                            parent: struct_id,
                            decl: Some(field_stmt.clone()),
                        });
                        self.enter(field_symbol, field_name);
                    } else if let Statement::FunctionDecl {
                        name: method_name, ..
                    } = &*field_stmt.borrow()
                    {
                        let method_id = self.get_symbol_id();
                        let method_symbol = Rc::new(Symbol::StructMethod {
                            name: method_name.value.clone(),
                            id: method_id,
                            parent: struct_id,
                            decl: Some(field_stmt.clone()),
                        });
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
            _ => {}
        }
    }

    fn register_function_symbols_in_expr(&mut self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Block { statements, .. } = &*expr.borrow() {
            for stmt in statements {
                self.register_symbols(stmt.clone());
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
                        let id = self.get_symbol_id();
                        let symbol = Rc::new(Symbol::Variable {
                            name,
                            id,
                            decl: None,
                            parameter: Some(parameter.clone()),
                        });
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
                name, initializer, ..
            } => {
                if name.value != "_" {
                    let id = self.get_symbol_id();
                    let symbol = Rc::new(Symbol::Variable {
                        name: name.value.clone(),
                        id,
                        decl: Some(stmt.clone()),
                        parameter: None,
                    });
                    self.enter(symbol, name);
                }
                if let Some(initializer) = initializer {
                    self.resolve_expression(initializer.clone());
                }
            }
            Statement::StructDecl { body, scope, .. } => {
                self.enter_scope(scope.clone());
                for stmt in body {
                    self.resolve_statement(stmt.clone());
                }
                self.leave_scope();
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
                Ok(sym) => *symbol = Some(sym),
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
            Expression::Binary { left, right, .. } => {
                self.resolve_expression(left.clone());
                self.resolve_expression(right.clone())
            }
            Expression::Unary { expression, .. } => self.resolve_expression(expression.clone()),
            Expression::Assignment { left, right, .. } => {
                self.resolve_expression(left.clone());
                self.resolve_expression(right.clone())
            }
            _ => {}
        }
    }

    fn enter(&mut self, symbol: Rc<Symbol>, token: &Token) {
        if let Some(scope) = self.current_scope.clone() {
            scope.borrow_mut().set_symbol(symbol.clone());
        } else {
            let name = match symbol.name() {
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

    fn resolve_symbol(&mut self, token: &Token) -> Result<Rc<Symbol>, ()> {
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
    ) -> Result<Option<Rc<Symbol>>, ()> {
        if let Some(symbol) = scope.borrow().get_symbol(&name) {
            Ok(Some(symbol))
        } else if let Some(parent) = scope.borrow().parent.clone() {
            self.resolve_symbol_in_scope(name, parent)
        } else {
            Ok(None)
        }
    }

    fn get_symbol_id(&mut self) -> SymbolId {
        let module = self.current_module.clone().unwrap();
        let id = SymbolId {
            id: module.borrow().symbol_count,
        };
        module.borrow_mut().symbol_count += 1;
        id
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
