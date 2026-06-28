use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use anyhow::{Ok, Result};

use crate::{
    ast::statement::{OwnershipModifier, Parameter, ProtocolAccessorSet, Statement},
    krate::Module as PackageModule,
    types::Type,
};

#[derive(Debug, Clone)]
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
        ownership: OwnershipModifier,
    },
    Struct {
        name: String,
        package: String,
        decl: Rc<RefCell<Statement>>,
        is_builtin_type: bool,
        has_dynamic_member_lookup: bool,
        has_dynamic_callable: bool,
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
        ownership: OwnershipModifier,
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
        package: String,
        decl: Rc<RefCell<Statement>>,
        has_dynamic_member_lookup: bool,
        has_dynamic_callable: bool,
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
        ownership: OwnershipModifier,
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
        package: String,
        decl: Rc<RefCell<Statement>>,
        has_dynamic_member_lookup: bool,
        has_dynamic_callable: bool,
        cases: Vec<Rc<RefCell<Symbol>>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
    },
    EnumCase {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        parameter_types: Vec<Rc<RefCell<Type>>>,
        has_parameters: bool,
    },
    Protocol {
        name: String,
        package: String,
        decl: Rc<RefCell<Statement>>,
        methods: Vec<Rc<RefCell<Symbol>>>,
        properties: Vec<Rc<RefCell<Symbol>>>,
        subscripts: Vec<Rc<RefCell<Symbol>>>,
        is_any_object_protocol: bool,
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
        accessors: ProtocolAccessorSet,
    },
    ProtocolSubscript {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        accessors: ProtocolAccessorSet,
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

impl PartialEq for Symbol {
    /// NOTE: `package` field is intentionally excluded from equality comparison
    /// so that two type symbols with the same name from different packages
    /// are considered equal. Access control uses the `package` field separately.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Function { name: a, decl: ad }, Self::Function { name: b, decl: bd }) => {
                a == b && Rc::ptr_eq(ad, bd)
            }
            (
                Self::Variable {
                    name: a,
                    decl: ad,
                    parameter: ap,
                    is_var: av,
                    ownership: a_own,
                },
                Self::Variable {
                    name: b,
                    decl: bd,
                    parameter: bp,
                    is_var: bv,
                    ownership: b_own,
                },
            ) => a == b && ad == bd && ap == bp && av == bv && a_own == b_own,
            (
                Self::Struct {
                    name: a,
                    decl: ad,
                    is_builtin_type: abt,
                    has_dynamic_member_lookup: hdml_a,
                    has_dynamic_callable: hdc_a,
                    properties: ap,
                    methods: am,
                    constructors: ac,
                    destrcutor: adr,
                    subscripts: as_,
                    package: _,
                },
                Self::Struct {
                    name: b,
                    decl: bd,
                    is_builtin_type: bbt,
                    has_dynamic_member_lookup: hdml_b,
                    has_dynamic_callable: hdc_b,
                    properties: bp,
                    methods: bm,
                    constructors: bc,
                    destrcutor: bdr,
                    subscripts: bs_,
                    package: _,
                },
            ) => {
                a == b
                    && Rc::ptr_eq(ad, bd)
                    && abt == bbt
                    && hdml_a == hdml_b
                    && hdc_a == hdc_b
                    && ap == bp
                    && am == bm
                    && ac == bc
                    && adr == bdr
                    && as_ == bs_
            }
            (
                Self::StructProperty {
                    name: a,
                    parent: ap,
                    decl: ad,
                    is_var: av,
                    ownership: a_own,
                },
                Self::StructProperty {
                    name: b,
                    parent: bp,
                    decl: bd,
                    is_var: bv,
                    ownership: b_own,
                },
            ) => a == b && ap == bp && ad == bd && av == bv && a_own == b_own,
            (Self::StructMethod { name: a, .. }, Self::StructMethod { name: b, .. }) => a == b,
            (Self::StructSubscript { name: a, .. }, Self::StructSubscript { name: b, .. }) => {
                a == b
            }
            (
                Self::Class {
                    name: a,
                    decl: ad,
                    has_dynamic_member_lookup: hdml_a,
                    has_dynamic_callable: hdc_a,
                    properties: ap,
                    methods: am,
                    constructors: ac,
                    destrcutor: adr,
                    superclass: asc,
                    subscripts: as_,
                    is_abstract: iab,
                    is_final: ifa,
                    package: _,
                },
                Self::Class {
                    name: b,
                    decl: bd,
                    has_dynamic_member_lookup: hdml_b,
                    has_dynamic_callable: hdc_b,
                    properties: bp,
                    methods: bm,
                    constructors: bc,
                    destrcutor: bdr,
                    superclass: bsc,
                    subscripts: bs_,
                    is_abstract: ibb,
                    is_final: ifb,
                    package: _,
                },
            ) => {
                a == b
                    && Rc::ptr_eq(ad, bd)
                    && hdml_a == hdml_b
                    && hdc_a == hdc_b
                    && ap == bp
                    && am == bm
                    && ac == bc
                    && adr == bdr
                    && asc == bsc
                    && as_ == bs_
                    && iab == ibb
                    && ifa == ifb
            }
            (
                Self::ClassProperty {
                    name: a,
                    parent: ap,
                    decl: ad,
                    is_var: av,
                    is_final: af,
                    is_override: ao,
                    ownership: a_own,
                },
                Self::ClassProperty {
                    name: b,
                    parent: bp,
                    decl: bd,
                    is_var: bv,
                    is_final: bf,
                    is_override: bo,
                    ownership: b_own,
                },
            ) => a == b && ap == bp && ad == bd && av == bv && af == bf && ao == bo && a_own == b_own,
            (
                Self::ClassMethod { name: a, .. },
                Self::ClassMethod { name: b, .. },
            ) => a == b,
            (Self::ClassSubscript { name: a, .. }, Self::ClassSubscript { name: b, .. }) => a == b,
            (
                Self::Enum {
                    name: a,
                    decl: ad,
                    has_dynamic_member_lookup: hdml_a,
                    has_dynamic_callable: hdc_a,
                    cases: ac,
                    methods: am,
                    package: _,
                },
                Self::Enum {
                    name: b,
                    decl: bd,
                    has_dynamic_member_lookup: hdml_b,
                    has_dynamic_callable: hdc_b,
                    cases: bc,
                    methods: bm,
                    package: _,
                },
            ) => {
                a == b
                    && Rc::ptr_eq(ad, bd)
                    && hdml_a == hdml_b
                    && hdc_a == hdc_b
                    && ac == bc
                    && am == bm
            }
            (
                Self::EnumCase {
                    name: a,
                    parent: ap,
                    decl: ad,
                    parameter_types: apt,
                    has_parameters: ahp,
                },
                Self::EnumCase {
                    name: b,
                    parent: bp,
                    decl: bd,
                    parameter_types: bpt,
                    has_parameters: bhp,
                },
            ) => a == b && ap == bp && ad == bd && apt == bpt && ahp == bhp,
            (
                Self::Protocol {
                    name: a,
                    decl: ad,
                    methods: am,
                    properties: ap,
                    subscripts: as_,
                    is_any_object_protocol: iap,
                    package: _,
                },
                Self::Protocol {
                    name: b,
                    decl: bd,
                    methods: bm,
                    properties: bp,
                    subscripts: bs_,
                    is_any_object_protocol: ibp,
                    package: _,
                },
            ) => {
                a == b
                    && Rc::ptr_eq(ad, bd)
                    && am == bm
                    && ap == bp
                    && as_ == bs_
                    && iap == ibp
            }
            (
                Self::ProtocolMethod {
                    name: a,
                    parent: ap,
                    decl: ad,
                    is_autowired: aa,
                },
                Self::ProtocolMethod {
                    name: b,
                    parent: bp,
                    decl: bd,
                    is_autowired: ba,
                },
            ) => a == b && ap == bp && ad == bd && aa == ba,
            (
                Self::ProtocolProperty {
                    name: a,
                    parent: ap,
                    decl: ad,
                    accessors: aa,
                },
                Self::ProtocolProperty {
                    name: b,
                    parent: bp,
                    decl: bd,
                    accessors: ba,
                },
            ) => a == b && ap == bp && ad == bd && aa == ba,
            (
                Self::ProtocolSubscript {
                    name: a,
                    parent: ap,
                    decl: ad,
                    accessors: aa,
                },
                Self::ProtocolSubscript {
                    name: b,
                    parent: bp,
                    decl: bd,
                    accessors: ba,
                },
            ) => a == b && ap == bp && ad == bd && aa == ba,
            (Self::Module { name: a, .. }, Self::Module { name: b, .. }) => a == b,
            (Self::Macro { name: a, .. }, Self::Macro { name: b, .. }) => a == b,
            _ => false,
        }
    }
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
