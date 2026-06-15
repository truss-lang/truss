use std::{cell::RefCell, fmt, rc::Rc};

use crate::symbol::WeakSymbol;

#[derive(Debug, Clone)]
pub enum Type {
    Never,
    Void,
    Function(
        Vec<Rc<RefCell<Type>>>,
        Rc<RefCell<Type>>,
        bool,
        Option<Vec<Rc<RefCell<Type>>>>,
    ),
    Pointer(Rc<RefCell<Type>>),
    NonNullPointer(Rc<RefCell<Type>>),
    Tuple(Vec<(Option<String>, Rc<RefCell<Type>>)>),
    Struct(String, WeakSymbol, Vec<Rc<RefCell<Type>>>),
    Class(String, WeakSymbol, Vec<Rc<RefCell<Type>>>),
    Enum(String, WeakSymbol, Vec<Rc<RefCell<Type>>>),
    Protocol(String, WeakSymbol, Vec<Rc<RefCell<Type>>>),
    Compound(Vec<Rc<RefCell<Type>>>),
    Inline(Rc<RefCell<Type>>, Option<u64>),
    GenericParam(String),
    ConstGeneric(String, Rc<RefCell<Type>>),
    AssociatedType(Rc<RefCell<Type>>, String),
    ClosureContext(Vec<Rc<RefCell<Type>>>, Rc<RefCell<Type>>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Never => write!(f, "Never"),
            Type::Void => write!(f, "Void"),
            Type::Function(params, ret, is_vararg, throws) => {
                write!(f, "Function(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param.borrow())?;
                }
                if *is_vararg {
                    write!(f, ", ...")?;
                }
                write!(f, ")")?;
                if let Some(throws_types) = throws {
                    if throws_types.is_empty() {
                        write!(f, " throws")?;
                    } else {
                        write!(f, " throws(")?;
                        for (i, th) in throws_types.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", th.borrow())?;
                        }
                        write!(f, ")")?;
                    }
                }
                write!(f, " -> {}", ret.borrow())
            }
            Type::Pointer(inner) => write!(f, "{}*", inner.borrow()),
            Type::NonNullPointer(inner) => write!(f, "{}*!", inner.borrow()),
            Type::Tuple(elements) => {
                write!(f, "(")?;
                for (i, (name, element)) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Some(name) = name {
                        write!(f, "{}: ", name)?;
                    }
                    write!(f, "{}", element.borrow())?;
                }
                write!(f, ")")
            }
            Type::Struct(name, _, type_params) => {
                if type_params.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}<", name)?;
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", tp.borrow())?;
                    }
                    write!(f, ">")
                }
            }
            Type::Class(name, _, type_params) => {
                if type_params.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}<", name)?;
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", tp.borrow())?;
                    }
                    write!(f, ">")
                }
            }
            Type::Enum(name, _, type_params) => {
                if type_params.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}<", name)?;
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", tp.borrow())?;
                    }
                    write!(f, ">")
                }
            }
            Type::Protocol(name, _, type_params) => {
                if type_params.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}<", name)?;
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", tp.borrow())?;
                    }
                    write!(f, ">")
                }
            }
            Type::Compound(types) => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " & ")?;
                    }
                    write!(f, "{}", t.borrow())?;
                }
                Ok(())
            }
            Type::Inline(inner, size) => {
                if let Some(size) = size {
                    write!(f, "Inline({}, {})", inner.borrow(), size)
                } else {
                    write!(f, "Inline({}, auto)", inner.borrow())
                }
            }
            Type::GenericParam(name) => write!(f, "GenericParam({})", name),
            Type::ConstGeneric(name, ty) => write!(f, "ConstGeneric({}, {})", name, ty.borrow()),
            Type::AssociatedType(base, name) => write!(f, "{}.{}", base.borrow(), name),
            Type::ClosureContext(params, ret) => {
                write!(f, "closure(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param.borrow())?;
                }
                write!(f, ") -> {}", ret.borrow())
            }
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Type::Never, Type::Never) => true,
            (Type::Void, Type::Void) => true,
            (Type::Struct(n1, _, p1), Type::Struct(n2, _, p2))
            | (Type::Class(n1, _, p1), Type::Class(n2, _, p2))
            | (Type::Enum(n1, _, p1), Type::Enum(n2, _, p2))
            | (Type::Protocol(n1, _, p1), Type::Protocol(n2, _, p2)) => n1 == n2 && p1 == p2,
            (Type::Function(p1, r1, v1, t1), Type::Function(p2, r2, v2, t2)) => {
                p1 == p2 && r1 == r2 && v1 == v2 && t1 == t2
            }
            (Type::Pointer(p1), Type::Pointer(p2)) => p1 == p2,
            (Type::NonNullPointer(p1), Type::NonNullPointer(p2)) => p1 == p2,
            (Type::Tuple(t1), Type::Tuple(t2)) => t1 == t2,
            (Type::Inline(i1, s1), Type::Inline(i2, s2)) => i1 == i2 && s1 == s2,
            (Type::Compound(c1), Type::Compound(c2)) => c1 == c2,
            (Type::ConstGeneric(n1, t1), Type::ConstGeneric(n2, t2)) => n1 == n2 && t1 == t2,
            (Type::GenericParam(n1), Type::GenericParam(n2)) => n1 == n2,
            (Type::AssociatedType(b1, n1), Type::AssociatedType(b2, n2)) => b1 == b2 && n1 == n2,
            (Type::ClosureContext(p1, r1), Type::ClosureContext(p2, r2)) => p1 == p2 && r1 == r2,
            _ => false,
        }
    }
}

pub fn builtin_type(name: &str) -> Type {
    Type::Struct(name.to_string(), WeakSymbol(std::rc::Weak::new()), vec![])
}
