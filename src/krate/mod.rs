use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::scope::Scope;

#[derive(Debug)]
pub struct Crate {
    pub name: String,
    pub modules: HashMap<String, Rc<RefCell<Module>>>,
}
impl Crate {
    pub fn new(name: String) -> Self {
        Self {
            name,
            modules: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub scope: Option<Rc<RefCell<Scope>>>,
}
impl Module {
    pub fn new(name: String) -> Self {
        Self { name, scope: None }
    }
}
