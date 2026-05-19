use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{Ok, Result};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    values::{BasicValueEnum, PointerValue},
};

use crate::{
    ast::{
        expression::{AssignmentOperator, BinaryOperator, Expression, UnaryOperator},
        node::Program,
        statement::{FunctionBody, Statement},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic},
    lexer::token::TokenType,
    types::Type,
};

struct Scope<'ctx> {
    variables: HashMap<String, PointerValue<'ctx>>,
}

impl<'ctx> Scope<'ctx> {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
}

pub struct IRGenerator<'ctx> {
    context: &'ctx Context,
    module: Rc<Module<'ctx>>,
    builder: Builder<'ctx>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
    scope_stack: Rc<RefCell<Vec<Scope<'ctx>>>>,
    alloca_namer: Rc<RefCell<HashMap<String, u32>>>,
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
            scope_stack: Rc::new(RefCell::new(vec![Scope::new()])),
            alloca_namer: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn unique_alloca_name(&self, base_name: &str) -> String {
        let mut namer = self.alloca_namer.borrow_mut();
        let counter = namer.entry(base_name.to_string()).or_insert(0);
        let name = format!("{}_{}", base_name, counter);
        *counter += 1;
        name
    }

    fn enter_scope_with_stmts(&self, statements: &[Rc<RefCell<Statement>>]) -> Result<()> {
        self.scope_stack.borrow_mut().push(Scope::new());

        for stmt in statements {
            if let Statement::VariableDecl {
                name,
                type_expression,
                initializer: _,
                ty,
                ..
            } = &*stmt.borrow()
            {
                let llvm_type = if let Some(ty) = ty {
                    self.resolve_type(ty.clone())?
                } else if let Some(type_expr) = type_expression {
                    self.infer_type_from_expression(type_expr.clone())?
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::TypeInferenceFailed,
                        format!("Variable '{}' has no type annotation", name.value),
                    );
                    anyhow::bail!("Cannot determine variable type");
                };

                let alloca_name = self.unique_alloca_name(&name.value);
                let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                self.declare_variable(name.value.clone(), ptr);
            }
        }

        Ok(())
    }

    fn resolve_block_stmts(&self, statements: &[Rc<RefCell<Statement>>]) -> Result<bool> {
        for stmt in statements {
            let terminates = self.resolve_statement(stmt.clone())?;
            if terminates {
                return Ok(true);
            }
        }
        self.exit_scope();
        Ok(false)
    }

    fn enter_scope(&self) {
        self.scope_stack.borrow_mut().push(Scope::new());
    }

    fn exit_scope(&self) {
        self.scope_stack.borrow_mut().pop();
    }

    fn declare_variable(&self, name: String, ptr: PointerValue<'ctx>) {
        self.scope_stack
            .borrow_mut()
            .last_mut()
            .unwrap()
            .variables
            .insert(name, ptr);
    }

    fn lookup_variable(&self, name: &str) -> Option<PointerValue<'ctx>> {
        let stack = self.scope_stack.borrow();
        for scope in stack.iter().rev() {
            if let Some(ptr) = scope.variables.get(name) {
                return Some(*ptr);
            }
        }
        None
    }

    pub fn generate(&self, program: &Program) -> Rc<Module<'ctx>> {
        for stmt in &program.statements {
            let _ = self.resolve_statement(stmt.clone());
        }
        self.module.clone()
    }

    fn resolve_block_expression(&self, block_expr: Rc<RefCell<Expression>>) -> Result<bool> {
        if let Expression::Block { statements } = &*block_expr.borrow() {
            self.enter_scope_with_stmts(statements)?;
            self.resolve_block_stmts(statements)
        } else {
            Ok(false)
        }
    }

    fn resolve_statement(&self, statement: Rc<RefCell<Statement>>) -> Result<bool> {
        match &*statement.borrow() {
            Statement::VariableDecl {
                name, initializer, ..
            } => {
                if let Some(init) = initializer {
                    let init_val = self.resolve_expression(init.clone())?;
                    if let Some(ptr) = self.lookup_variable(&name.value) {
                        self.builder.build_store(ptr, init_val)?;
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::IRVariableNotFound,
                            format!("Variable '{}' alloca not found", name.value),
                        );
                        anyhow::bail!("Variable alloca not found");
                    }
                }
                Ok(false)
            }
            Statement::While { condition, body } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let while_bb = self.context.append_basic_block(fn_val, "while_cond");
                let body_bb = self.context.append_basic_block(fn_val, "while_body");
                let exit_bb = self.context.append_basic_block(fn_val, "while_exit");

                self.builder.build_unconditional_branch(while_bb)?;
                self.builder.position_at_end(while_bb);

                let cond_val = self.resolve_expression(condition.clone())?;
                let cond_int = cond_val.into_int_value();
                self.builder
                    .build_conditional_branch(cond_int, body_bb, exit_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body.clone())?;

                if !terminates {
                    self.builder.build_unconditional_branch(while_bb)?;
                }

                self.builder.position_at_end(exit_bb);
                Ok(false)
            }
            Statement::Loop { body } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let body_bb = self.context.append_basic_block(fn_val, "loop_body");
                let _ = self.context.append_basic_block(fn_val, "loop_exit");

                self.builder.build_unconditional_branch(body_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body.clone())?;

                if !terminates {
                    self.builder.build_unconditional_branch(body_bb)?;
                }

                Ok(false)
            }
            Statement::RepeatWhile { body, condition } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let body_bb = self.context.append_basic_block(fn_val, "repeat_body");
                let cond_bb = self.context.append_basic_block(fn_val, "repeat_cond");
                let exit_bb = self.context.append_basic_block(fn_val, "repeat_exit");

                self.builder.build_unconditional_branch(body_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body.clone())?;

                if !terminates {
                    self.builder.build_unconditional_branch(cond_bb)?;
                }

                self.builder.position_at_end(cond_bb);
                let cond_val = self.resolve_expression(condition.clone())?;
                let cond_int = cond_val.into_int_value();
                self.builder
                    .build_conditional_branch(cond_int, body_bb, exit_bb)?;

                self.builder.position_at_end(exit_bb);
                Ok(false)
            }
            Statement::For {
                pattern: _,
                iterator,
                body,
            } => {
                let _ = self.resolve_expression(iterator.clone());
                self.resolve_block_expression(body.clone())?;
                Ok(false)
            }
            Statement::FunctionDecl {
                ty: Some(ty),
                name,
                parameters,
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

                    self.enter_scope();
                    for (i, param) in parameters.iter().enumerate() {
                        let param_name = &param.borrow().name.value;
                        let llvm_type = self.resolve_type(param.borrow().ty.clone().unwrap())?;
                        let alloca_name = self.unique_alloca_name(param_name);
                        let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                        let param_value = function.get_nth_param(i as u32).unwrap();
                        self.builder.build_store(ptr, param_value)?;
                        self.declare_variable(param_name.clone(), ptr);
                    }

                    let is_void = matches!(&*return_type.borrow(), Type::Void);

                    match &*body.borrow() {
                        FunctionBody::Statements(stmts) => {
                            self.enter_scope_with_stmts(stmts)?;
                            let mut has_return = false;
                            for stmt in stmts {
                                let terminates = self.resolve_statement(stmt.clone())?;
                                if terminates {
                                    has_return = true;
                                    break;
                                }
                            }
                            if is_void && !has_return {
                                self.builder.build_return(None)?;
                            }
                            self.exit_scope();
                        }
                        FunctionBody::Expression(expr) => {
                            let value = self.resolve_expression(expr.clone())?;
                            self.builder.build_return(Some(&value))?;
                        }
                    }
                }
                Ok(false)
            }
            Statement::Return { value, .. } => {
                match value {
                    Some(value) if !matches!(&*value.borrow(), Expression::VoidLiteral { .. }) => {
                        let value = self.resolve_expression(value.clone())?;
                        self.builder.build_return(Some(&value))?;
                    }
                    _ => {
                        self.builder.build_return(None)?;
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn resolve_expression(&self, expr: Rc<RefCell<Expression>>) -> Result<BasicValueEnum<'ctx>> {
        match &*expr.borrow() {
            Expression::IntegerLiteral { value, ty, .. } => {
                let llvm_type = match ty {
                    Some(t) => self.resolve_type(t.clone())?,
                    None => self.context.i32_type().into(),
                };
                Ok(llvm_type
                    .into_int_type()
                    .const_int(*value as u64, false)
                    .into())
            }
            Expression::BooleanLiteral { token } => {
                let value = match &token.ty {
                    TokenType::BooleanLiteral { value } => *value,
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
            Expression::Block { statements } => {
                self.enter_scope_with_stmts(statements)?;
                self.resolve_block_stmts(statements)?;

                // Return an empty int value as a placeholder
                Ok(self.context.i32_type().const_int(0, false).into())
            }
            Expression::Variable { name, ty, .. } => {
                if let Some(ptr) = self.lookup_variable(&name.value) {
                    let llvm_type = if let Some(ty) = ty {
                        self.resolve_type(ty.clone())?
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            format!(
                                "Variable '{}' needs type annotation for load operation",
                                name.value
                            ),
                        );
                        anyhow::bail!("Variable needs type annotation");
                    };
                    let val = self.builder.build_load(llvm_type, ptr, "")?;
                    Ok(val)
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("Undefined variable: '{}'", name.value),
                    );
                    anyhow::bail!("Undefined variable: {}", name.value);
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_val = self.resolve_expression(left.clone())?;
                let right_val = self.resolve_expression(right.clone())?;

                match operator {
                    BinaryOperator::Plus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_add(l, r, "")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_add(l, r, "")?.into())
                        } else {
                            anyhow::bail!("Invalid types for addition");
                        }
                    }
                    BinaryOperator::Minus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_sub(l, r, "")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_sub(l, r, "")?.into())
                        } else {
                            anyhow::bail!("Invalid types for subtraction");
                        }
                    }
                    BinaryOperator::Multiply => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_mul(l, r, "")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_mul(l, r, "")?.into())
                        } else {
                            anyhow::bail!("Invalid types for multiplication");
                        }
                    }
                    BinaryOperator::Divide => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(self.builder.build_int_signed_div(l, r, "")?.into())
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(self.builder.build_float_div(l, r, "")?.into())
                        } else {
                            anyhow::bail!("Invalid types for division");
                        }
                    }
                    _ => anyhow::bail!("Binary operator {:?} not implemented", operator),
                }
            }
            Expression::If {
                condition,
                then,
                else_,
            } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let cond_bb = self.context.append_basic_block(fn_val, "if_cond");
                let then_bb = self.context.append_basic_block(fn_val, "if_then");
                let else_bb = if else_.is_some() {
                    Some(self.context.append_basic_block(fn_val, "if_else"))
                } else {
                    None
                };
                let exit_bb = self.context.append_basic_block(fn_val, "if_exit");

                self.builder.build_unconditional_branch(cond_bb)?;
                self.builder.position_at_end(cond_bb);

                let cond_val = self.resolve_expression(condition.clone())?.into_int_value();
                self.builder.build_conditional_branch(
                    cond_val,
                    then_bb,
                    if let Some(else_bb) = else_bb {
                        else_bb
                    } else {
                        exit_bb
                    },
                )?;

                self.builder.position_at_end(then_bb);
                let terminates = self.resolve_block_expression(then.clone())?;
                if !terminates {
                    self.builder.build_unconditional_branch(exit_bb)?;
                }

                if let Some(else_) = else_ {
                    self.builder.position_at_end(else_bb.unwrap());
                    let terminates = self.resolve_block_expression(else_.clone())?;
                    if !terminates {
                        self.builder.build_unconditional_branch(exit_bb)?;
                    }
                }

                self.builder.position_at_end(exit_bb);

                // Return an empty int value as a placeholder
                Ok(self.context.i32_type().const_int(0, false).into())
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

    fn infer_type_from_expression(
        &self,
        expr: Rc<RefCell<Expression>>,
    ) -> Result<BasicTypeEnum<'ctx>> {
        match &*expr.borrow() {
            Expression::IntegerLiteral { ty, .. } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty.clone())
                } else {
                    Ok(self.context.i32_type().into())
                }
            }
            Expression::DecimalLiteral { ty, .. } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty.clone())
                } else {
                    Ok(self.context.f64_type().into())
                }
            }
            Expression::BooleanLiteral { .. } => Ok(self.context.bool_type().into()),
            Expression::CharLiteral { .. } => Ok(self.context.i8_type().into()),
            Expression::Variable { ty, .. } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty.clone())
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::TypeInferenceFailed,
                        "Cannot infer type from variable without type annotation",
                    );
                    anyhow::bail!("Cannot infer type from variable")
                }
            }
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::TypeInferenceFailed,
                    "Cannot infer type from this expression",
                );
                anyhow::bail!("Cannot infer type")
            }
        }
    }

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>) {
        let diag = new_diagnostic(code, message);
        self.engine.borrow_mut().emit(diag);
    }
}
