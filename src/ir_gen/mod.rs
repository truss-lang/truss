use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result};
use inkwell::{builder::Builder, context::Context, module::Module, types::AnyType};

use crate::{
    ast::{node::Program, statement::Statement},
    types::Type,
};

pub struct IRGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    #[allow(dead_code)]
    builder: Builder<'ctx>,
}

impl<'ctx> IRGenerator<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("main");
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
        }
    }

    pub fn generate(&'ctx self, program: &Program) -> Result<Module<'ctx>> {
        for stmt in &program.statements {
            self.resolve_statement(stmt.clone())?;
        }
        Ok(self.module.clone())
    }

    fn resolve_statement(&'ctx self, statement: Rc<RefCell<Statement>>) -> Result<()> {
        match &*statement.borrow() {
            Statement::VariableDecl { .. } => {}
            Statement::FunctionDecl { ty: Some(ty), .. } => {
                if let Type::Function(parameter_types, _ret_type) = &*ty.borrow() {
                    let mut param_types = Vec::new();
                    for param_type in parameter_types {
                        param_types.push(self.resolve_type(param_type.clone())?);
                    }
                }
            }
            Statement::Return { value: Some(_) } => {}
            Statement::ExpressionStatement { expression: _ } => {}
            _ => {}
        }
        Ok(())
    }

    fn resolve_type(&'ctx self, _ty: Rc<RefCell<Type>>) -> Result<Box<dyn AnyType<'ctx> + 'ctx>> {
        Ok(Box::new(self.context.i64_type()) as Box<dyn AnyType<'ctx> + 'ctx>)
    }
}
