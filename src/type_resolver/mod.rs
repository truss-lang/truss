use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{Ok, Result, anyhow};

use crate::{
    ast::{
        expression::Expression,
        node::Program,
        statement::{FunctionBody, Parameter, Statement},
    },
    id::ModuleId,
    krate::{Crate, Module},
    types::Type,
};

#[derive(Debug, Clone, Default)]
struct TypeEnv {
    vars: HashMap<String, Rc<RefCell<Type>>>,
}
impl TypeEnv {
    fn get(&self, name: String) -> Result<Rc<RefCell<Type>>> {
        self.vars
            .get(&name)
            .cloned()
            .ok_or(anyhow!(format!("Not found variable: {}", name)))
    }
    fn set(&mut self, name: String, ty: Rc<RefCell<Type>>) -> Option<Rc<RefCell<Type>>> {
        self.vars.insert(name.clone(), ty)
    }
    fn contains(&self, name: String) -> bool {
        self.vars.contains_key(&name)
    }
    fn remove(&mut self, name: String) -> Result<Rc<RefCell<Type>>> {
        self.vars.remove(&name).ok_or(anyhow!("Error"))
    }
}

#[derive(Debug)]
pub struct TypeResolver {
    pub krate: Rc<RefCell<Crate>>,
    current_module: Option<Rc<RefCell<Module>>>,
    current_function: Option<Rc<RefCell<Statement>>>,
    type_env: Option<Rc<RefCell<TypeEnv>>>,
}

impl TypeResolver {
    pub fn new(krate: Rc<RefCell<Crate>>) -> Self {
        Self {
            krate,
            current_module: None,
            current_function: None,
            type_env: None,
        }
    }
    pub fn resolve(&mut self, program: &Program, id: ModuleId) -> Result<()> {
        self.current_module = self.krate.borrow().modules.get(&id).cloned();
        for stmt in &program.statements {
            self.resolve_statement(stmt.clone())?;
        }
        Ok(())
    }
    fn resolve_statement(&mut self, statement: Rc<RefCell<Statement>>) -> Result<()> {
        match &mut *statement.borrow_mut() {
            Statement::VariableDecl {
                name,
                type_expression,
                initializer,
                ty,
                ..
            } => {
                if let Some(type_expression) = type_expression {
                    self.resolve_expression(type_expression.clone())?;
                    *ty = type_expression.borrow().get_ty()?;
                    self.type_env
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set(name.value.clone(), ty.clone().unwrap());
                } else if let Some(initializer) = initializer {
                    *ty = Some(self.infer(initializer.clone())?);
                    self.type_env
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set(name.value.clone(), ty.clone().unwrap());
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
                let last_type_env = self.type_env.clone();
                self.type_env = Some(Rc::new(RefCell::new(TypeEnv::default())));
                for param in parameters {
                    self.resolve_param(param.clone())?;
                    self.type_env.as_ref().unwrap().borrow_mut().set(
                        param.borrow().name.value.clone(),
                        param.borrow().ty.clone().unwrap(),
                    );
                }
                self.resolve_function_body(body.clone())?;
                self.type_env = last_type_env;
            }
            Statement::ExpressionStatement { expression } => {}
            Statement::Return { value: Some(value) } => {
                self.resolve_expression(value.clone())?;
            }
            _ => {}
        }
        Ok(())
    }
    fn resolve_param(&mut self, param: Rc<RefCell<Parameter>>) -> Result<()> {
        let type_expression = param.borrow().type_expression.clone();
        self.resolve_expression(type_expression.clone())?;
        param.borrow_mut().ty = type_expression.borrow().get_ty()?;
        Ok(())
    }
    fn resolve_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) -> Result<()> {
        match &mut *body.borrow_mut() {
            FunctionBody::Statements(statements) => {
                for stmt in statements {
                    self.resolve_statement(stmt.clone())?;
                }
            }
            FunctionBody::Expression(expression) => self.resolve_expression(expression.clone())?,
        }
        Ok(())
    }
    fn resolve_expression(&mut self, expression: Rc<RefCell<Expression>>) -> Result<()> {
        match &mut *expression.borrow_mut() {
            Expression::Block { statements } => {
                for stmt in statements {
                    self.resolve_statement(stmt.clone())?;
                }
            }
            Expression::IntegerLiteral { token, ty } => {
                *ty = Some(Rc::new(RefCell::new(Type::Int32)));
            }
            Expression::Variable { name, ty, .. } => {
                *ty = Some(
                    self.type_env
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .get(name.value.clone())?,
                )
            }
            Expression::Type { name, ty, .. } => {
                if name.value == "Int32" {
                    *ty = Some(Rc::new(RefCell::new(Type::Int32)));
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn infer(&mut self, expression: Rc<RefCell<Expression>>) -> Result<Rc<RefCell<Type>>> {
        match &mut *expression.borrow_mut() {
            Expression::IntegerLiteral { token, ty } => {
                *ty = Some(Rc::new(RefCell::new(Type::Int32)));
                Ok(ty.clone().unwrap())
            }
            Expression::Variable { name, ty, .. } => {
                *ty = Some(
                    self.type_env
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .get(name.value.clone())?,
                );
                Ok(ty.clone().unwrap())
            }
            _ => Ok(Rc::new(RefCell::new(Type::Unit))),
        }
    }
}
