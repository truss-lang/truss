use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicType, BasicTypeEnum, BasicMetadataTypeEnum},
};

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
            Statement::FunctionDecl { ty: Some(ty), name, .. } => {
                if let Type::Function(parameter_types, return_type) = &*ty.borrow() {
                    let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
                    for param_type in parameter_types {
                        let basic_type = self.resolve_type(param_type.clone())?;
                        param_types.push(basic_type.into());
                    }
                    
                    let fn_name = &name.value;
                    
                    let is_void = matches!(&*return_type.borrow(), Type::Unit); 
                    if is_void {
                        let void_type = self.context.void_type();
                        let function_type = void_type.fn_type(&param_types, false);
                        self.module.add_function(fn_name, function_type, None);
                    } else {
                        let return_type = self.resolve_type(return_type.clone())?;
                        let function_type = return_type.fn_type(&param_types, false);
                        self.module.add_function(fn_name, function_type, None);
                    }
                }
            }
            Statement::Return { value: Some(_) } => {}
            Statement::ExpressionStatement { expression: _ } => {}
            _ => {}
        }
        Ok(())
    }

    fn resolve_type(&'ctx self, ty: Rc<RefCell<Type>>) -> Result<BasicTypeEnum<'ctx>> {
        let resolved = match &*ty.borrow() {
            Type::Int8 => self.context.i8_type().into(),
            Type::Int16 => self.context.i16_type().into(),
            Type::Int32 => self.context.i32_type().into(),
            Type::Int64 => self.context.i64_type().into(),
            Type::Int128 => self.context.i128_type().into(),
            Type::UInt8 => self.context.i8_type().into(),
            Type::UInt16 => self.context.i16_type().into(),
            Type::UInt32 => self.context.i32_type().into(),
            Type::UInt64 => self.context.i64_type().into(),
            Type::UInt128 => self.context.i128_type().into(),
            Type::Float32 => self.context.f32_type().into(),
            Type::Float64 => self.context.f64_type().into(),
            Type::Bool => self.context.bool_type().into(),
            Type::Char => self.context.i8_type().into(),
            Type::Never => return Err(anyhow::anyhow!("Never type cannot be converted to LLVM type")),
            Type::Unit => return Err(anyhow::anyhow!("Unit type is handled specially as void return type")),
            Type::Function(_, _) => {
                return Err(anyhow::anyhow!("Nested function types are not supported"));
            }
        };
        Ok(resolved)
    }
}
