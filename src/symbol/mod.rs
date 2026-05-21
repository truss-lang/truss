use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result};

use crate::{
    ast::statement::{Parameter, Statement},
    id::SymbolId,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Function {
        name: String,
        id: SymbolId,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    Variable {
        name: String,
        id: SymbolId,
        decl: Option<Rc<RefCell<Statement>>>,
        parameter: Option<Rc<RefCell<Parameter>>>,
    },
    Struct {
        name: String,
        id: SymbolId,
        decl: Option<Rc<RefCell<Statement>>>,
    },
}

impl Symbol {
    pub fn id(&self) -> SymbolId {
        match self {
            Self::Function { id, .. } => *id,
            Self::Variable { id, .. } => *id,
            Self::Struct { id, .. } => *id,
        }
    }
    pub fn name(&self) -> Result<String> {
        match self {
            Self::Function { name, .. } => Ok(name.clone()),
            Self::Variable { name, .. } => Ok(name.clone()),
            Self::Struct { name, .. } => Ok(name.clone()),
        }
    }
    pub fn get_decl(&self) -> Result<Option<Rc<RefCell<Statement>>>> {
        match self {
            Self::Function { decl, .. } => Ok(decl.clone()),
            Self::Variable { decl, .. } => Ok(decl.clone()),
            Self::Struct { decl, .. } => Ok(decl.clone()),
        }
    }
}
