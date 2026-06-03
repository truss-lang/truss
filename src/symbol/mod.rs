use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use anyhow::{Ok, Result};

use crate::{
    ast::statement::{Parameter, Statement},
    krate::Module as KrateModule,
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
        is_var: bool,
    },
    Struct {
        name: String,
        decl: Rc<RefCell<Statement>>,
        properties: Vec<Rc<RefCell<Symbol>>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
        constructors: Vec<Rc<RefCell<Symbol>>>,
        destrcutor: Option<Rc<RefCell<Symbol>>>,
    },
    StructProperty {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        is_var: bool,
    },
    StructMethod {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    Class {
        name: String,
        decl: Rc<RefCell<Statement>>,
        properties: Vec<Rc<RefCell<Symbol>>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
        constructors: Vec<Rc<RefCell<Symbol>>>,
        destrcutor: Option<Rc<RefCell<Symbol>>>,
        superclass: Option<WeakSymbol>,
    },
    ClassProperty {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        is_var: bool,
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
        methods: Vec<Rc<RefCell<Symbol>>>,
        properties: Vec<Rc<RefCell<Symbol>>>,
    },
    ProtocolMethod {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    ProtocolProperty {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    Module {
        name: String,
        decl: Rc<RefCell<Statement>>,
        module: Option<Rc<RefCell<KrateModule>>>,
    },
}

impl Symbol {
    pub fn name(&self) -> Result<String> {
        match self {
            Self::Function { name, .. } => Ok(name.clone()),
            Self::Variable { name, .. } => Ok(name.clone()),
            Self::Struct { name, .. } => Ok(name.clone()),
            Self::StructProperty { name, .. } => Ok(name.clone()),
            Self::StructMethod { name, .. } => Ok(name.clone()),
            Self::Class { name, .. } => Ok(name.clone()),
            Self::ClassProperty { name, .. } => Ok(name.clone()),
            Self::ClassMethod { name, .. } => Ok(name.clone()),
            Self::Enum { name, .. } => Ok(name.clone()),
            Self::EnumCase { name, .. } => Ok(name.clone()),
            Self::Protocol { name, .. } => Ok(name.clone()),
            Self::ProtocolMethod { name, .. } => Ok(name.clone()),
            Self::ProtocolProperty { name, .. } => Ok(name.clone()),
            Self::Module { name, .. } => Ok(name.clone()),
        }
    }
    pub fn get_decl(&self) -> Result<Option<Rc<RefCell<Statement>>>> {
        match self {
            Self::Function { decl, .. } => Ok(Some(decl.clone())),
            Self::Variable { decl, .. } => Ok(decl.clone()),
            Self::Struct { decl, .. } => Ok(Some(decl.clone())),
            Self::StructProperty { decl, .. } => Ok(decl.clone()),
            Self::StructMethod { decl, .. } => Ok(decl.clone()),
            Self::Class { decl, .. } => Ok(Some(decl.clone())),
            Self::ClassProperty { decl, .. } => Ok(decl.clone()),
            Self::ClassMethod { decl, .. } => Ok(decl.clone()),
            Self::Enum { decl, .. } => Ok(Some(decl.clone())),
            Self::EnumCase { decl, .. } => Ok(decl.clone()),
            Self::Protocol { decl, .. } => Ok(Some(decl.clone())),
            Self::ProtocolMethod { decl, .. } => Ok(decl.clone()),
            Self::ProtocolProperty { decl, .. } => Ok(decl.clone()),
            Self::Module { decl, .. } => Ok(Some(decl.clone())),
        }
    }
    pub fn parent(&self) -> Option<Rc<RefCell<Symbol>>> {
        match self {
            Self::StructProperty { parent, .. }
            | Self::StructMethod { parent, .. }
            | Self::ClassProperty { parent, .. }
            | Self::ClassMethod { parent, .. }
            | Self::EnumCase { parent, .. }
            | Self::ProtocolMethod { parent, .. }
            | Self::ProtocolProperty { parent, .. } => Some(parent.0.upgrade().unwrap()),
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
