use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    id::{CrateId, ModuleId, SymbolId},
    symbol::Symbol,
};

#[derive(Debug)]
pub struct Crate {
    pub name: String,
    pub id: CrateId,
    pub modules: HashMap<ModuleId, Rc<RefCell<Module>>>,
}
impl Crate {
    pub fn new(name: String, id: CrateId) -> Self {
        Self {
            name,
            id,
            modules: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub id: ModuleId,
    pub symbols: HashMap<SymbolId, Rc<Symbol>>,
    pub name_table: HashMap<String, Rc<Symbol>>,
    pub symbol_count: usize,
}
impl Module {
    pub fn new(name: String, id: ModuleId) -> Self {
        Self {
            name,
            id,
            symbols: HashMap::new(),
            name_table: HashMap::new(),
            symbol_count: 0,
        }
    }
}
