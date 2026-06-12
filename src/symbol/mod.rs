use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use anyhow::{Ok, Result};

use crate::{
    ast::statement::{Parameter, Statement},
    krate::Module as PackageModule,
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
        is_builtin_type: bool,
        properties: Vec<Rc<RefCell<Symbol>>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
        constructors: Vec<Rc<RefCell<Symbol>>>,
        destrcutor: Option<Rc<RefCell<Symbol>>>,
        subscripts: Vec<Rc<RefCell<Symbol>>>,
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
    StructSubscript {
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
        subscripts: Vec<Rc<RefCell<Symbol>>>,
        is_abstract: bool,
        is_final: bool,
    },
    ClassProperty {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        is_var: bool,
        is_final: bool,
        is_override: bool,
    },
    ClassMethod {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        is_abstract: bool,
        is_final: bool,
        is_override: bool,
    },
    ClassSubscript {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        is_final: bool,
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
        subscripts: Vec<Rc<RefCell<Symbol>>>,
    },
    ProtocolMethod {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        is_autowired: bool,
    },
    ProtocolProperty {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    ProtocolSubscript {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
    },
    Module {
        name: String,
        decl: Rc<RefCell<Statement>>,
        module: Option<Rc<RefCell<PackageModule>>>,
    },
    Macro {
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
            Self::StructProperty { name, .. } => Ok(name.clone()),
            Self::StructMethod { name, .. } => Ok(name.clone()),
            Self::StructSubscript { name, .. } => Ok(name.clone()),
            Self::Class { name, .. } => Ok(name.clone()),
            Self::ClassProperty { name, .. } => Ok(name.clone()),
            Self::ClassMethod { name, .. } => Ok(name.clone()),
            Self::ClassSubscript { name, .. } => Ok(name.clone()),
            Self::Enum { name, .. } => Ok(name.clone()),
            Self::EnumCase { name, .. } => Ok(name.clone()),
            Self::Protocol { name, .. } => Ok(name.clone()),
            Self::ProtocolMethod { name, .. } => Ok(name.clone()),
            Self::ProtocolProperty { name, .. } => Ok(name.clone()),
            Self::ProtocolSubscript { name, .. } => Ok(name.clone()),
            Self::Module { name, .. } => Ok(name.clone()),
            Self::Macro { name, .. } => Ok(name.clone()),
        }
    }
    pub fn with_name(&self, new_name: &str) -> Self {
        let mut cloned = self.clone();
        match &mut cloned {
            Self::Function { name, .. } => *name = new_name.to_string(),
            Self::Variable { name, .. } => *name = new_name.to_string(),
            Self::Struct { name, .. } => *name = new_name.to_string(),
            Self::StructProperty { name, .. } => *name = new_name.to_string(),
            Self::StructMethod { name, .. } => *name = new_name.to_string(),
            Self::StructSubscript { name, .. } => *name = new_name.to_string(),
            Self::Class { name, .. } => *name = new_name.to_string(),
            Self::ClassProperty { name, .. } => *name = new_name.to_string(),
            Self::ClassMethod { name, .. } => *name = new_name.to_string(),
            Self::ClassSubscript { name, .. } => *name = new_name.to_string(),
            Self::Enum { name, .. } => *name = new_name.to_string(),
            Self::EnumCase { name, .. } => *name = new_name.to_string(),
            Self::Protocol { name, .. } => *name = new_name.to_string(),
            Self::ProtocolMethod { name, .. } => *name = new_name.to_string(),
            Self::ProtocolProperty { name, .. } => *name = new_name.to_string(),
            Self::ProtocolSubscript { name, .. } => *name = new_name.to_string(),
            Self::Module { name, .. } => *name = new_name.to_string(),
            Self::Macro { name, .. } => *name = new_name.to_string(),
        }
        cloned
    }
    pub fn get_decl(&self) -> Result<Option<Rc<RefCell<Statement>>>> {
        match self {
            Self::Function { decl, .. } => Ok(Some(decl.clone())),
            Self::Variable { decl, .. } => Ok(decl.clone()),
            Self::Struct { decl, .. } => Ok(Some(decl.clone())),
            Self::StructProperty { decl, .. } => Ok(decl.clone()),
            Self::StructMethod { decl, .. } => Ok(decl.clone()),
            Self::StructSubscript { decl, .. } => Ok(decl.clone()),
            Self::Class { decl, .. } => Ok(Some(decl.clone())),
            Self::ClassProperty { decl, .. } => Ok(decl.clone()),
            Self::ClassMethod { decl, .. } => Ok(decl.clone()),
            Self::ClassSubscript { decl, .. } => Ok(decl.clone()),
            Self::Enum { decl, .. } => Ok(Some(decl.clone())),
            Self::EnumCase { decl, .. } => Ok(decl.clone()),
            Self::Protocol { decl, .. } => Ok(Some(decl.clone())),
            Self::ProtocolMethod { decl, .. } => Ok(decl.clone()),
            Self::ProtocolProperty { decl, .. } => Ok(decl.clone()),
            Self::ProtocolSubscript { decl, .. } => Ok(decl.clone()),
            Self::Module { decl, .. } => Ok(Some(decl.clone())),
            Self::Macro { decl, .. } => Ok(Some(decl.clone())),
        }
    }
    pub fn parent(&self) -> Option<Rc<RefCell<Symbol>>> {
        match self {
            Self::StructProperty { parent, .. }
            | Self::StructMethod { parent, .. }
            | Self::StructSubscript { parent, .. }
            | Self::ClassProperty { parent, .. }
            | Self::ClassMethod { parent, .. }
            | Self::ClassSubscript { parent, .. }
            | Self::EnumCase { parent, .. }
            | Self::ProtocolMethod { parent, .. }
            | Self::ProtocolProperty { parent, .. }
            | Self::ProtocolSubscript { parent, .. } => Some(parent.0.upgrade().unwrap()),
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
