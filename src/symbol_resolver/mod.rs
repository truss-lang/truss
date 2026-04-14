use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{Ok, Result, anyhow};

use crate::{
    ast::{expression::Expression, node::Program, statement::Statement},
    id::{ModuleId, SymbolId},
    krate::{Crate, Module},
    symbol::Symbol,
    types::Type,
};

#[derive(Debug)]
struct Scope {
    pub symbols: HashMap<SymbolId, Rc<Symbol>>,
    pub name_table: HashMap<String, Rc<Symbol>>,
    pub parent: Option<Rc<RefCell<Scope>>>,
}
impl Scope {
    fn new(parent: Option<Rc<RefCell<Scope>>>) -> Self {
        Self {
            symbols: HashMap::new(),
            name_table: HashMap::new(),
            parent,
        }
    }
}

#[derive(Debug)]
pub struct SymbolResolver {
    pub krate: Crate,
    current_module: Option<Rc<RefCell<Module>>>,
    current_scope: Option<Rc<RefCell<Scope>>>,
}
impl SymbolResolver {
    pub fn new(krate: Crate) -> Self {
        Self {
            krate,
            current_module: None,
            current_scope: None,
        }
    }
    pub fn resolve(&mut self, program: &Program, module_name: String) -> Result<()> {
        let module = Rc::new(RefCell::new(Module::new(
            module_name,
            ModuleId {
                id: self.krate.modules.len(),
            },
        )));
        self.krate
            .modules
            .insert(module.borrow().id, module.clone());
        self.current_module = Some(module.clone());
        for stmt in &program.statements {
            self.resolve_statement(stmt.clone())?;
        }
        Ok(())
    }
    fn resolve_statement(&mut self, stmt: Rc<RefCell<Statement>>) -> Result<()> {
        match &*stmt.borrow() {
            Statement::FunctionDecl {
                name,
                return_type,
                body,
                ..
            } => {
                let module = self.current_module.clone().unwrap();
                let id = SymbolId {
                    id: module.borrow().symbol_count,
                };
                module.borrow_mut().symbol_count += 1;
                let symbol = Rc::new(Symbol::Function {
                    name: name.value.clone(),
                    id,
                    decl: stmt.clone(),
                });
                self.enter(id, symbol)?;
                self.enter_scope();
                // TODO: enter parameters
                if let Some(return_type) = return_type {
                    self.resolve_expression(return_type.clone())?;
                }
                self.resolve_expression(body.clone())?;
                self.leave_scope();
            }
            Statement::VariableDecl {
                name, initializer, ..
            } => {
                let module = self.current_module.clone().unwrap();
                let id = SymbolId {
                    id: module.borrow().symbol_count,
                };
                module.borrow_mut().symbol_count += 1;
                let symbol = Rc::new(Symbol::Variable {
                    name: name.value.clone(),
                    id,
                    decl: stmt.clone(),
                });
                self.enter(id, symbol)?;
                if let Some(initializer) = initializer {
                    self.resolve_expression(initializer.clone())?;
                }
            }
            Statement::ExpressionStatement { expression } => {
                self.resolve_expression(expression.clone())?
            }
        }
        Ok(())
    }
    fn resolve_expression(&mut self, expr: Rc<RefCell<Expression>>) -> Result<()> {
        match &mut *expr.borrow_mut() {
            Expression::Block { statements } => {
                self.enter_scope();
                for stmt in statements {
                    self.resolve_statement(stmt.clone())?;
                }
                self.leave_scope();
            }
            Expression::Variable {
                name,
                expression,
                symbol,
                ..
            } => {
                if let Some(name) = name {
                    *symbol = Some(self.resolve_symbol(name.value.clone())?);
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn enter(&mut self, id: SymbolId, symbol: Rc<Symbol>) -> Result<()> {
        if let Some(scope) = self.current_scope.clone() {
            scope.borrow_mut().symbols.insert(id, symbol.clone());
            scope.borrow_mut().name_table.insert(symbol.name()?, symbol);
        } else {
            let module = self.current_module.clone().unwrap();
            module.borrow_mut().symbols.insert(id, symbol.clone());
            module
                .borrow_mut()
                .name_table
                .insert(symbol.name()?, symbol);
        }
        Ok(())
    }
    fn resolve_symbol(&mut self, name: String) -> Result<Rc<Symbol>> {
        if let Some(current_scope) = self.current_scope.clone()
            && let Some(symbol) = self.resolve_symbol_in_scope(name.clone(), current_scope)?
        {
            Ok(symbol)
        } else {
            let module = self.current_module.clone().unwrap();
            module
                .borrow()
                .name_table
                .get(&name)
                .cloned()
                .ok_or(anyhow!("symbol not found"))
        }
    }
    fn resolve_symbol_in_scope(
        &mut self,
        name: String,
        scope: Rc<RefCell<Scope>>,
    ) -> Result<Option<Rc<Symbol>>> {
        if let Some(symbol) = scope.borrow().name_table.get(&name).cloned() {
            Ok(Some(symbol))
        } else if let Some(parent) = scope.borrow().parent.clone() {
            self.resolve_symbol_in_scope(name, parent)
        } else {
            Ok(None)
        }
    }
    fn enter_scope(&mut self) {
        let scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
        self.current_scope = Some(scope);
    }
    fn leave_scope(&mut self) {
        self.current_scope = self.current_scope.clone().unwrap().borrow().parent.clone();
    }
}
