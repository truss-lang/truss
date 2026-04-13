use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{ast::statement::Statement, id::SymbolId};

#[derive(Debug)]
pub enum Symbol {
    Function {
        id: SymbolId,
        decl: Rc<RefCell<Statement>>,
    },
}

impl Symbol {
    pub fn id(&self) -> SymbolId {
        match self {
            Self::Function { id, .. } => *id,
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
        }
    }
}
