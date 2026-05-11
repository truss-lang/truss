use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result, anyhow};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Unit,
    Int32,
    Function(Rc<RefCell<Type>>, Vec<Rc<RefCell<Type>>>),
}
