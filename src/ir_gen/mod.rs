use std::{cell::RefCell, ops::Deref, rc::Rc};

use anyhow::{Ok, Result};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::{AnyType, BasicType, BasicTypeEnum, FunctionType},
};

use crate::{
    ast::{node::Program, statement::Statement},
    types::Type,
};

pub struct IRGenerator {
    context: Context,
}

impl IRGenerator {
    pub fn new() -> Self {
        Self {
            context: Context::create(),
        }
    }
    pub fn generate(&self, program: &Program) -> Result<Module> {
        let ir_module = self.context.create_module("main");
        let builder = self.context.create_builder();
        for stmt in &program.statements {
            self.resolve_statement(&ir_module, &builder, stmt.clone())?;
        }
        Ok(ir_module)
    }
    fn resolve_statement(
        &self,
        ir_module: &Module,
        builder: &Builder,
        statement: Rc<RefCell<Statement>>,
    ) -> Result<()> {
        match &*statement.borrow() {
            Statement::VariableDecl {
                name,
                type_expression,
                initializer,
                ty,
                ..
            } => {}
            Statement::FunctionDecl {
                name,
                parameters,
                return_type,
                body,
                ty: Some(ty),
                ..
            } => {
                if let Type::Function(parameter_types, ret_type) = &*ty.borrow() {
                    let mut param_types = Vec::new();
                    for param_type in parameter_types {
                        param_types.push(self.resolve_type(param_type.clone())?);
                    }
                }
            }
            Statement::Return { value: Some(value) } => {}
            Statement::ExpressionStatement { expression } => {}
            _ => {}
        }
        Ok(())
    }
    fn resolve_type<'a>(&'a self, ty: Rc<RefCell<Type>>) -> Result<Box<dyn AnyType<'a> + 'a>> {
        Ok(Box::new(self.context.i64_type()))
    }
}
