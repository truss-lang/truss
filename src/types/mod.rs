use std::{cell::RefCell, fmt, rc::Rc};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Never,
    Void,
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
    Function(Vec<Rc<RefCell<Type>>>, Rc<RefCell<Type>>, bool),
    Pointer(Rc<RefCell<Type>>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Never => write!(f, "Never"),
            Type::Void => write!(f, "Void"),
            Type::Int8 => write!(f, "Int8"),
            Type::Int16 => write!(f, "Int16"),
            Type::Int32 => write!(f, "Int32"),
            Type::Int64 => write!(f, "Int64"),
            Type::Int128 => write!(f, "Int128"),
            Type::UInt8 => write!(f, "UInt8"),
            Type::UInt16 => write!(f, "UInt16"),
            Type::UInt32 => write!(f, "UInt32"),
            Type::UInt64 => write!(f, "UInt64"),
            Type::UInt128 => write!(f, "UInt128"),
            Type::Float32 => write!(f, "Float32"),
            Type::Float64 => write!(f, "Float64"),
            Type::Char => write!(f, "Char"),
            Type::Bool => write!(f, "Bool"),
            Type::Function(params, ret, is_vararg) => {
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
                write!(f, ") -> {}", ret.borrow())
            }
            Type::Pointer(inner) => write!(f, "{}*", inner.borrow()),
        }
    }
}
