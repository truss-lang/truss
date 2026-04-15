use std::{cell::RefCell, rc::Rc};

use super::statement::Statement;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub file: Rc<String>,
    pub statements: Vec<Rc<RefCell<Statement>>>,
}

impl Program {
    pub fn new(file: Rc<String>) -> Self {
        Self {
            file,
            statements: vec![],
        }
    }
}
