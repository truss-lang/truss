use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    values::BasicValueEnum,
};

use crate::{
    ast::{expression::Expression, node::Program, statement::Statement},
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic},
    types::Type,
};

pub struct IRGenerator<'ctx> {
    context: &'ctx Context,
    module: Rc<Module<'ctx>>,
    builder: Builder<'ctx>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
}

impl<'ctx> IRGenerator<'ctx> {
    pub fn new(context: &'ctx Context, engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        let module = Rc::new(context.create_module("main"));
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            engine,
        }
    }

    pub fn generate(&self, program: &Program) -> Rc<Module<'ctx>> {
        for stmt in &program.statements {
            let _ = self.resolve_statement(stmt.clone());
        }
        self.module.clone()
    }

    fn resolve_statement(&self, statement: Rc<RefCell<Statement>>) -> Result<()> {
        match &*statement.borrow() {
            Statement::VariableDecl { .. } => {}
            Statement::FunctionDecl {
                ty: Some(ty),
                name,
                body,
                ..
            } => {
                if let Type::Function(parameter_types, return_type) = &*ty.borrow() {
                    let fn_name = &name.value;
                    let function = self.module.add_function(
                        fn_name,
                        self.get_function_type(
                            return_type.clone(),
                            parameter_types.clone(),
                            false,
                        )?,
                        None,
                    );

                    let entry_block = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry_block);

                    match &*body.borrow() {
                        crate::ast::statement::FunctionBody::Statements(stmts) => {
                            for stmt in stmts {
                                self.resolve_statement(stmt.clone())?;
                            }
                        }
                        crate::ast::statement::FunctionBody::Expression(expr) => {
                            let value = self.resolve_expression(expr.clone())?;
                            self.builder.build_return(Some(&value))?;
                        }
                    }
                }
            }
            Statement::Return { value, .. } => {
                if let Some(value) = value {
                    if matches!(&*value.borrow(), Expression::VoidLiteral { .. }) {
                        self.builder.build_return(None)?;
                    } else {
                        let value = self.resolve_expression(value.clone())?;
                        self.builder.build_return(Some(&value))?;
                    }
                } else {
                    self.builder.build_return(None)?;
                }
            }
            Statement::ExpressionStatement { .. } => {}
            _ => {}
        }
        Ok(())
    }

    fn resolve_expression(&self, expr: Rc<RefCell<Expression>>) -> Result<BasicValueEnum<'ctx>> {
        match &*expr.borrow() {
            Expression::IntegerLiteral { value, ty, .. } => {
                let llvm_type = match ty {
                    Some(t) => self.resolve_type(t.clone())?,
                    None => self.context.i32_type().into(),
                };
                Ok(llvm_type.into_int_type().const_int(*value as u64, false).into())
            }
            Expression::BooleanLiteral { token } => {
                let value = match &token.ty {
                    crate::lexer::token::TokenType::BooleanLiteral { value } => *value,
                    _ => false,
                };
                Ok(self
                    .context
                    .bool_type()
                    .const_int(value as u64, false)
                    .into())
            }
            Expression::DecimalLiteral { value, ty, .. } => {
                let llvm_type = match ty {
                    Some(t) => self.resolve_type(t.clone())?,
                    None => self.context.f64_type().into(),
                };
                Ok(llvm_type.into_float_type().const_float(*value).into())
            }
            Expression::CharLiteral { .. } => Ok(self.context.i8_type().const_int(0, false).into()),
            Expression::Variable { .. } => {
                anyhow::bail!("Variable lookup not implemented");
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_val = self.resolve_expression(left.clone())?;
                let right_val = self.resolve_expression(right.clone())?;

                match operator {
                    crate::ast::expression::BinaryOperator::Plus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_add(l, r, "add")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_add(l, r, "fadd")?.into())
                        } else {
                            anyhow::bail!("Invalid types for addition");
                        }
                    }
                    crate::ast::expression::BinaryOperator::Minus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_sub(l, r, "sub")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_sub(l, r, "fsub")?.into())
                        } else {
                            anyhow::bail!("Invalid types for subtraction");
                        }
                    }
                    crate::ast::expression::BinaryOperator::Multiply => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_mul(l, r, "mul")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_mul(l, r, "fmul")?.into())
                        } else {
                            anyhow::bail!("Invalid types for multiplication");
                        }
                    }
                    crate::ast::expression::BinaryOperator::Divide => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_signed_div(l, r, "sdiv")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_div(l, r, "fdiv")?.into())
                        } else {
                            anyhow::bail!("Invalid types for division");
                        }
                    }
                    _ => anyhow::bail!("Binary operator {:?} not implemented", operator),
                }
            }
            _ => anyhow::bail!("Expression type not implemented"),
        }
    }
    fn get_function_type(
        &self,
        return_type: Rc<RefCell<Type>>,
        param_types: Vec<Rc<RefCell<Type>>>,
        is_vararg: bool,
    ) -> Result<inkwell::types::FunctionType<'ctx>> {
        let mut param_basic_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
        for param_type in param_types {
            param_basic_types.push(self.resolve_type(param_type.clone())?.into());
        }

        let is_void = matches!(&*return_type.borrow(), Type::Void);
        if is_void {
            let void_type = self.context.void_type();
            Ok(void_type.fn_type(&param_basic_types, is_vararg))
        } else {
            let return_basic_type = self.resolve_type(return_type.clone())?;
            Ok(return_basic_type.fn_type(&param_basic_types, is_vararg))
        }
    }
    fn resolve_type(&self, ty: Rc<RefCell<Type>>) -> Result<BasicTypeEnum<'ctx>> {
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
            Type::Never => {
                self.emit_error(
                    TrussDiagnosticCode::NeverTypeConversion,
                    "Never type cannot be converted to LLVM type",
                );
                anyhow::bail!("Never type cannot be converted to LLVM type");
            }
            Type::Void => {
                self.emit_error(
                    TrussDiagnosticCode::VoidTypeConversion,
                    "Void type is handled specially as void return type",
                );
                anyhow::bail!("Void type is handled specially as void return type");
            }
            Type::Function(_, _) => {
                self.emit_error(
                    TrussDiagnosticCode::NestedFunctionType,
                    "Nested function types are not supported",
                );
                anyhow::bail!("Nested function types are not supported");
            }
        };
        Ok(resolved)
    }

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>) {
        let diag = new_diagnostic(code, message);
        self.engine.borrow_mut().emit(diag);
    }
}
