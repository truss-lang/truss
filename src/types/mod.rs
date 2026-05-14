use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result, anyhow};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Never,
    Unit,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    Float32,
    Float64,
    Char,
    Bool,
    Function(Vec<Rc<RefCell<Type>>>, Rc<RefCell<Type>>),
}
