use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{symbol::Symbol, types::Type};

#[derive(Debug, PartialEq)]
pub struct Scope {
    pub name_table: HashMap<String, Rc<RefCell<Symbol>>>,
    pub overloads: HashMap<String, Vec<Rc<RefCell<Symbol>>>>,
    pub type_env: HashMap<String, Rc<RefCell<Type>>>,
    pub parent: Option<Rc<RefCell<Scope>>>,
}
impl Scope {
    pub fn new(parent: Option<Rc<RefCell<Scope>>>) -> Self {
        Self {
            name_table: HashMap::new(),
            overloads: HashMap::new(),
            type_env: HashMap::new(),
            parent,
        }
    }

    fn is_overloadable_symbol(symbol: &Symbol) -> bool {
        matches!(
            symbol,
            Symbol::Function { .. }
                | Symbol::StructMethod { .. }
                | Symbol::ClassMethod { .. }
                | Symbol::ProtocolMethod { .. }
        )
    }

    pub fn get_symbol(&self, name: &str) -> Option<Rc<RefCell<Symbol>>> {
        if let Some(symbol) = self.name_table.get(name) {
            Some(symbol.clone())
        } else if let Some(parent) = &self.parent {
            parent.borrow().get_symbol(name)
        } else {
            None
        }
    }

    pub fn get_all_symbols(&self, name: &str) -> Vec<Rc<RefCell<Symbol>>> {
        let mut result: Vec<Rc<RefCell<Symbol>>> = Vec::new();
        if let Some(prev_overloads) = self.overloads.get(name) {
            result.extend(prev_overloads.iter().cloned());
        }
        if let Some(symbol) = self.name_table.get(name) {
            result.push(symbol.clone());
        } else if let Some(parent) = &self.parent {
            return parent.borrow().get_all_symbols(name);
        }
        result
    }

    pub fn set_symbol(&mut self, symbol: Rc<RefCell<Symbol>>) {
        let name = symbol.borrow().name().unwrap();
        if let Some(existing) = self.name_table.get(&name) {
            if Self::is_overloadable_symbol(&existing.borrow()) {
                self.overloads
                    .entry(name.clone())
                    .or_default()
                    .push(existing.clone());
            }
        }
        self.name_table.insert(name, symbol);
    }

    pub fn get_type(&self, name: &str) -> Option<Rc<RefCell<Type>>> {
        if let Some(ty) = self.type_env.get(name) {
            Some(ty.clone())
        } else if let Some(parent) = &self.parent {
            parent.borrow().get_type(name)
        } else {
            None
        }
    }

    pub fn set_type(&mut self, name: String, ty: Rc<RefCell<Type>>) {
        self.type_env.insert(name, ty);
    }
}
