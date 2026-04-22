use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result, anyhow};

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: Option<TypeKind>,
    pub constraints: Vec<TypeConstraint>,
}
impl Type {
    pub fn new(kind: Option<TypeKind>) -> Self {
        Self {
            kind,
            constraints: vec![],
        }
    }
    pub fn combine(left: Rc<RefCell<Type>>, right: Rc<RefCell<Type>>) -> Result<Type> {
        if left.borrow().kind.is_none() {
            if right.borrow().kind.is_none() {
                let mut ty = left.borrow().clone();
                ty.constraints.extend(right.borrow().constraints.clone());
                Ok(ty)
            } else {
                Err(anyhow!(""))
            }
        } else if right.borrow().kind.is_some() {
            let mut ty = right.borrow().clone();
            ty.constraints.extend(left.borrow().constraints.clone());
            Ok(ty)
        } else {
            let mut constraints = left.borrow().constraints.clone();
            constraints.extend(right.borrow().constraints.clone());
            Ok(Type {
                kind: None,
                constraints,
            })
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Int32,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TypeConstraint {
    IntegerType,
    DefaultType(Rc<RefCell<Type>>),
    ShouldBeType(Rc<RefCell<Type>>),
}
