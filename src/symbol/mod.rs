use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result, anyhow};

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
}

impl Symbol {
    pub fn id(&self) -> SymbolId {
        match self {
            Self::Function { id, .. } => *id,
            Self::Variable { id, .. } => *id,
        }
    }
    pub fn name(&self) -> Result<String> {
        match self {
            Self::Function { decl, .. } => {
                if let Some(decl) = decl.clone()
                    && let Statement::FunctionDecl { name, .. } = &*decl.borrow()
                {
                    Ok(name.value.clone())
                } else {
                    Err(anyhow!(""))
                }
            }
            Self::Variable {
                decl, parameter, ..
            } => {
                if let Some(decl) = decl.clone()
                    && let Statement::VariableDecl { name, .. } = &*decl.borrow()
                {
                    Ok(name.value.clone())
                } else if let Some(parameter) = parameter {
                    Ok(parameter.borrow().name.value.clone())
                } else {
                    Err(anyhow!(""))
                }
            }
        }
    }
    pub fn get_decl(&self) -> Result<Option<Rc<RefCell<Statement>>>> {
        match self {
            Self::Function { decl, .. } => Ok(decl.clone()),
            Self::Variable { decl, .. } => Ok(decl.clone()),
        }
    }
}
