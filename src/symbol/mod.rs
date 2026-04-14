use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{ast::statement::Statement, id::SymbolId};

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Function {
        name: String,
        id: SymbolId,
        decl: Rc<RefCell<Statement>>,
    },
    Variable {
        name: String,
        id: SymbolId,
        decl: Rc<RefCell<Statement>>,
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
                if let Statement::FunctionDecl { name, .. } = &*decl.borrow() {
                    Ok(name.value.clone())
                } else {
                    Err(anyhow!(""))
                }
            }
            Self::Variable { decl, .. } => {
                if let Statement::VariableDecl { name, .. } = &*decl.borrow() {
                    Ok(name.value.clone())
                } else {
                    Err(anyhow!(""))
                }
            }
        }
    }
}
