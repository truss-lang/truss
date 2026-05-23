use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    id::{CrateId, ModuleId},
    scope::Scope,
};

#[derive(Debug)]
pub struct Crate {
    pub name: String,
    pub id: CrateId,
    pub modules: HashMap<ModuleId, Rc<RefCell<Module>>>,
    pub name_to_modules: HashMap<String, Rc<RefCell<Module>>>,
}
impl Crate {
    pub fn new(name: String, id: CrateId) -> Self {
        Self {
            name,
            id,
            modules: HashMap::new(),
            name_to_modules: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub id: ModuleId,
    pub scope: Option<Rc<RefCell<Scope>>>,
}
impl Module {
    pub fn new(name: String, id: ModuleId) -> Self {
        Self {
            name,
            id,
            scope: None,
        }
    }
}
