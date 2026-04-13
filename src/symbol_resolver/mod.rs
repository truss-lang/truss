use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::Result;

use crate::{
    ast::{expression::Expression, node::Program, statement::Statement},
    id::{ModuleId, SymbolId},
    krate::{Crate, Module},
    symbol::Symbol,
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
    file: Rc<String>,
    current_module: Option<Rc<RefCell<Module>>>,
    current_scope: Option<Rc<RefCell<Scope>>>,
}
impl SymbolResolver {
    pub fn new(krate: Crate, file: Rc<String>) -> Self {
        Self {
            krate,
            file,
            current_module: None,
            current_scope: None,
        }
    }
    pub fn resolve(&mut self, program: Program, module_name: String) -> Result<()> {
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
        for stmt in program.statements {
            self.resolve_statement(stmt)?;
        }
        Ok(())
    }
    fn resolve_statement(&mut self, stmt: Rc<Statement>) -> Result<()> {
        match &*stmt {
            Statement::FunctionDecl { body, .. } => {
                let module = self.current_module.clone().unwrap();
                let id = SymbolId {
                    id: module.borrow().symbol_count,
                };
                module.borrow_mut().symbol_count += 1;
                let symbol = Rc::new(Symbol::Function {
                    id,
                    decl: stmt.clone(),
                });
                self.enter(id, symbol)?;
                self.enter_scope();
                // TODO: enter parameters
                self.resolve_expression(body.clone())?;
                self.leave_scope();
            }
            Statement::ExpressionStatement { expression } => {
                self.resolve_expression(expression.clone())?
            }
        }
        Ok(())
    }
    fn resolve_expression(&mut self, expr: Rc<Expression>) -> Result<()> {
        match &*expr {
            Expression::Block { statements } => {
                self.enter_scope();
                for stmt in statements {
                    self.resolve_statement(stmt.clone())?;
                }
                self.leave_scope();
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
    fn enter_scope(&mut self) {
        let scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
        self.current_scope = Some(scope);
    }
    fn leave_scope(&mut self) {
        self.current_scope = self.current_scope.clone().unwrap().borrow().parent.clone();
    }
}
