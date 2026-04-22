use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{
    ast::{
        expression::Expression,
        node::Program,
        statement::{FunctionBody, Statement},
    },
    id::ModuleId,
    krate::{Crate, Module},
    types::{Type, TypeConstraint, TypeKind},
};

#[derive(Debug)]
pub struct TypeResolver {
    pub krate: Rc<RefCell<Crate>>,
    current_module: Option<Rc<RefCell<Module>>>,
    current_function: Option<Rc<RefCell<Statement>>>,
}

impl TypeResolver {
    pub fn new(krate: Rc<RefCell<Crate>>) -> Self {
        Self {
            krate,
            current_module: None,
            current_function: None,
        }
    }
    pub fn resolve(&mut self, program: &Program, id: ModuleId) -> Result<()> {
        self.current_module = self.krate.borrow().modules.get(&id).cloned();
        for stmt in &program.statements {
            self.parse_statement(stmt.clone())?;
        }
        Ok(())
    }
    pub fn parse_statement(&mut self, statement: Rc<RefCell<Statement>>) -> Result<()> {
        match &mut *statement.borrow_mut() {
            Statement::VariableDecl {
                name,
                type_expression,
                initializer,
                ty,
                ..
            } => {
                if let Some(type_expression) = type_expression {
                } else if let Some(initializer) = initializer {
                    self.parse_expression(initializer.clone())?;
                    *ty = initializer.borrow().get_ty()?;
                } else {
                    return Err(anyhow!(""));
                }
            }
            Statement::FunctionDecl {
                name,
                generic_parameters,
                parameters,
                return_type,
                body,
                ..
            } => {
                if let Some(return_type) = return_type {
                    self.parse_expression(return_type.clone())?;
                }
                let previous_function = self.current_function.clone();
                self.current_function = Some(statement.clone());
                self.parse_function_body(body.clone())?;
                self.current_function = previous_function;
            }
            Statement::ExpressionStatement { expression } => {
                self.parse_expression(expression.clone())?
            }
            Statement::Return { value: Some(value) } => {
                self.parse_expression(value.clone())?;
                if let Statement::FunctionDecl { return_type, .. } =
                    unsafe { &*self.current_function.clone().unwrap().as_ptr() }
                    && let Some(return_type) = return_type
                {
                    let mut value_mut = value.borrow_mut();
                    let value_ty = value_mut.get_ty_mut_ref()?;
                    if let Some(value_ty) = value_ty {
                        value_ty
                            .borrow_mut()
                            .constraints
                            .push(TypeConstraint::ShouldBeType(
                                return_type.borrow().get_ty()?.unwrap(),
                            ));
                    } else {
                        *value_ty = Some(return_type.borrow().get_ty()?.unwrap());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn parse_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) -> Result<()> {
        match &mut *body.borrow_mut() {
            FunctionBody::Statements(statements) => {
                for stmt in statements {
                    self.parse_statement(stmt.clone())?;
                }
            }
            FunctionBody::Expression(expression) => self.parse_expression(expression.clone())?,
        }
        Ok(())
    }
    pub fn parse_expression(&mut self, expression: Rc<RefCell<Expression>>) -> Result<()> {
        match &mut *expression.borrow_mut() {
            Expression::Block { statements } => {
                for stmt in statements {
                    self.parse_statement(stmt.clone())?;
                }
            }
            Expression::IntegerLiteral { token, ty } => {
                *ty = Some(Rc::new(RefCell::new(Type {
                    kind: None,
                    constraints: vec![
                        TypeConstraint::IntegerType,
                        TypeConstraint::DefaultType(Rc::new(RefCell::new(Type::new(Some(
                            TypeKind::Int32,
                        ))))),
                    ],
                })));
            }
            Expression::Variable { ty, symbol, .. } => {
                if let Some(decl) = symbol.clone().unwrap().get_decl()? {
                    *ty = decl.borrow().get_ty()?;
                }
            }
            Expression::Type { name, ty, .. } => {
                if name.value == "Int32" {
                    *ty = Some(Rc::new(RefCell::new(Type::new(Some(TypeKind::Int32)))));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
