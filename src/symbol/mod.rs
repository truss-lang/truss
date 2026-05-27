use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use anyhow::{Ok, Result};

use crate::{
    ast::statement::{Parameter, Statement},
    types::Type,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Function {
        name: String,
        decl: Rc<RefCell<Statement>>,
    },
    Variable {
        name: String,
        decl: Option<Rc<RefCell<Statement>>>,
        parameter: Option<Rc<RefCell<Parameter>>>,
    },
    Struct {
        name: String,
        decl: Rc<RefCell<Statement>>,
        fields: Vec<Rc<RefCell<Symbol>>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
        constructors: Vec<Rc<RefCell<Symbol>>>,
        destrcutor: Option<Rc<RefCell<Symbol>>>,
    },
    StructField {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    StructMethod {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    Class {
        name: String,
        decl: Rc<RefCell<Statement>>,
        fields: Vec<Rc<RefCell<Symbol>>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
        constructors: Vec<Rc<RefCell<Symbol>>>,
        destrcutor: Option<Rc<RefCell<Symbol>>>,
        superclass: Option<WeakSymbol>,
    },
    ClassField {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    ClassMethod {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    Enum {
        name: String,
        decl: Rc<RefCell<Statement>>,
        cases: Vec<Rc<RefCell<Symbol>>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
    },
    EnumCase {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        parameter_types: Vec<Rc<RefCell<Type>>>,
    },
    Protocol {
        name: String,
        decl: Rc<RefCell<Statement>>,
    },
}

impl Symbol {
    pub fn name(&self) -> Result<String> {
        match self {
            Self::Function { name, .. } => Ok(name.clone()),
            Self::Variable { name, .. } => Ok(name.clone()),
            Self::Struct { name, .. } => Ok(name.clone()),
            Self::StructField { name, .. } => Ok(name.clone()),
            Self::StructMethod { name, .. } => Ok(name.clone()),
            Self::Class { name, .. } => Ok(name.clone()),
            Self::ClassField { name, .. } => Ok(name.clone()),
            Self::ClassMethod { name, .. } => Ok(name.clone()),
            Self::Enum { name, .. } => Ok(name.clone()),
            Self::EnumCase { name, .. } => Ok(name.clone()),
            Self::Protocol { name, .. } => Ok(name.clone()),
        }
    }
    pub fn get_decl(&self) -> Result<Option<Rc<RefCell<Statement>>>> {
        match self {
            Self::Function { decl, .. } => Ok(Some(decl.clone())),
            Self::Variable { decl, .. } => Ok(decl.clone()),
            Self::Struct { decl, .. } => Ok(Some(decl.clone())),
            Self::StructField { decl, .. } => Ok(decl.clone()),
            Self::StructMethod { decl, .. } => Ok(decl.clone()),
            Self::Class { decl, .. } => Ok(Some(decl.clone())),
            Self::ClassField { decl, .. } => Ok(decl.clone()),
            Self::ClassMethod { decl, .. } => Ok(decl.clone()),
            Self::Enum { decl, .. } => Ok(Some(decl.clone())),
            Self::EnumCase { decl, .. } => Ok(decl.clone()),
            Self::Protocol { decl, .. } => Ok(Some(decl.clone())),
        }
    }
    pub fn parent(&self) -> Option<Rc<RefCell<Symbol>>> {
        match self {
            Self::StructField { parent, .. }
            | Self::StructMethod { parent, .. }
            | Self::ClassField { parent, .. }
            | Self::ClassMethod { parent, .. }
            | Self::EnumCase { parent, .. } => Some(parent.0.upgrade().unwrap()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeakSymbol(pub Weak<RefCell<Symbol>>);

impl PartialEq for WeakSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
