use std::rc::Rc;

use super::statement::Statement;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub file: Rc<String>,
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn new(file: Rc<String>) -> Self {
        Self {
            file,
            statements: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParameter {}
