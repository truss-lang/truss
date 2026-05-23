use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{symbol::Symbol, types::Type};

#[derive(Debug, PartialEq)]
pub struct Scope {
    pub name_table: HashMap<String, Rc<RefCell<Symbol>>>,
    pub type_env: HashMap<String, Rc<RefCell<Type>>>,
    pub parent: Option<Rc<RefCell<Scope>>>,
}
impl Scope {
    pub fn new(parent: Option<Rc<RefCell<Scope>>>) -> Self {
        Self {
            name_table: HashMap::new(),
            type_env: HashMap::new(),
            parent,
        }
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
    pub fn set_symbol(&mut self, symbol: Rc<RefCell<Symbol>>) {
        self.name_table
            .insert(symbol.borrow().name().unwrap(), symbol.clone());
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
