use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::scope::Scope;

#[derive(Debug, PartialEq)]
pub struct Package {
    pub name: String,
    pub modules: HashMap<String, Rc<RefCell<Module>>>,
}
impl Package {
    pub fn new(name: String) -> Self {
        Self {
            name,
            modules: HashMap::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Module {
    pub name: String,
    pub scope: Option<Rc<RefCell<Scope>>>,
    pub children: HashMap<String, Rc<RefCell<Module>>>,
}
impl Module {
    pub fn new(name: String) -> Self {
        Self {
            name,
            scope: None,
            children: HashMap::new(),
        }
    }
}
