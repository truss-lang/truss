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
        statement::{FunctionBody, Statement, VariadicKind},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    lexer::token::{Token, TokenType},
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
                initializer,
                ty,
                ..
            } = &*stmt.borrow()
            {
                let llvm_type = if let Some(ty) = ty {
                    self.resolve_type(ty.clone())?
                } else if let Some(type_expr) = type_expression {
                    self.infer_type_from_expression(type_expr.clone())?
                } else if let Some(init) = initializer {
                    self.infer_type_from_expression(init.clone())?
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::TypeInferenceFailed,
                        format!("Variable '{}' has no type annotation", name.value),
                        Some(name),
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
            self.create_function_declarations(stmt.clone());
        }

        for stmt in &program.statements {
            let _ = self.resolve_statement(stmt.clone());
        }
        self.module.clone()
    }

    fn create_function_declarations(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::FunctionDecl { name, ty, body, .. } = &*statement.borrow() {
            if let Some(ty) = ty {
                let _ = self.create_function_declaration(name, ty);
            }
            match &*body.borrow() {
                FunctionBody::Statements(stmts) => {
                    for s in stmts {
                        self.create_function_declarations(s.clone());
                    }
                }
                FunctionBody::Expression(expr) => {
                    self.create_function_declarations_in_expr(expr.clone());
                }
                FunctionBody::None => {}
            }
        }
        if let Statement::ExternBlock { items, .. } = &*statement.borrow() {
            for item in items {
                let _ = self.create_extern_declaration(item.clone());
            }
        }
        if let Statement::ExternDecl { statement, .. } = &*statement.borrow() {
            let _ = self.create_extern_declaration(statement.clone());
        }
    }

    fn create_extern_declaration(&self, statement: Rc<RefCell<Statement>>) -> Result<()> {
        if let Statement::FunctionDecl { name, ty, .. } = &*statement.borrow() {
            if let Some(ty) = ty {
                if let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow() {
                    let function_type = self.get_function_type(
                        return_type.clone(),
                        param_types.clone(),
                        *is_vararg,
                    )?;
                    self.module.add_function(&name.value, function_type, None);
                }
            }
        }
        if let Statement::VariableDecl { name, ty, .. } = &*statement.borrow() {
            if let Some(ty) = ty {
                let llvm_type = self.resolve_type(ty.clone())?;
                self.module.add_global(llvm_type, None, &name.value);
            }
        }
        Ok(())
    }

    fn create_function_declarations_in_expr(&self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Block { statements } = &*expr.borrow() {
            for stmt in statements {
                self.create_function_declarations(stmt.clone());
            }
        }
    }

    fn create_function_declaration(&self, name: &Token, ty: &Rc<RefCell<Type>>) -> Result<()> {
        if let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow() {
            let function_type =
                self.get_function_type(return_type.clone(), param_types.clone(), *is_vararg)?;
            self.module.add_function(&name.value, function_type, None);
        }
        Ok(())
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
                    let init_val = self.resolve_expression(init.clone())?.unwrap();
                    if let Some(ptr) = self.lookup_variable(&name.value) {
                        self.builder.build_store(ptr, init_val)?;
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::IRVariableNotFound,
                            format!("Variable '{}' alloca not found", name.value),
                            Some(name),
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

                let cond_val = self.resolve_expression(condition.clone())?.unwrap();
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
                let cond_val = self.resolve_expression(condition.clone())?.unwrap();
                let cond_int = cond_val.into_int_value();
                self.builder
                    .build_conditional_branch(cond_int, body_bb, exit_bb)?;

                self.builder.position_at_end(exit_bb);
                Ok(false)
            }
            Statement::For { iterator, body, .. } => {
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
                if let Type::Function(_parameter_types, return_type, _) = &*ty.borrow() {
                    let fn_name = &name.value;
                    let function = self.module.get_function(fn_name).unwrap();

                    let current_block = self.builder.get_insert_block();

                    let entry_block = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry_block);

                    self.enter_scope();
                    for (i, param) in parameters.iter().enumerate() {
                        if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                            continue;
                        }
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
                            let value = self.resolve_expression(expr.clone())?.unwrap();
                            self.builder.build_return(Some(&value))?;
                        }
                        FunctionBody::None => {}
                    }

                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
                    }
                }
                Ok(false)
            }
            Statement::Return { value, .. } => {
                match value {
                    Some(value) if !matches!(&*value.borrow(), Expression::VoidLiteral { .. }) => {
                        let value = self.resolve_expression(value.clone())?.unwrap();
                        self.builder.build_return(Some(&value))?;
                    }
                    _ => {
                        self.builder.build_return(None)?;
                    }
                }
                Ok(true)
            }
            Statement::ExpressionStatement { expression } => {
                self.resolve_expression(expression.clone())?;
                Ok(false)
            }
            Statement::ExternBlock { items, .. } => {
                for item in items {
                    let _ = self.resolve_statement(item.clone());
                }
                Ok(false)
            }
            Statement::ExternDecl { .. } => Ok(false),
            _ => Ok(false),
        }
    }

    fn resolve_expression(
        &self,
        expr: Rc<RefCell<Expression>>,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        match &*expr.borrow() {
            Expression::IntegerLiteral { value, ty, .. } => {
                let llvm_type = match ty {
                    Some(t) => self.resolve_type(t.clone())?,
                    None => self.context.i32_type().into(),
                };
                Ok(Some(
                    llvm_type
                        .into_int_type()
                        .const_int(*value as u64, false)
                        .into(),
                ))
            }
            Expression::BooleanLiteral { token } => {
                let value = match &token.ty {
                    TokenType::BooleanLiteral { value } => *value,
                    _ => false,
                };
                Ok(Some(
                    self.context
                        .bool_type()
                        .const_int(value as u64, false)
                        .into(),
                ))
            }
            Expression::DecimalLiteral { value, ty, .. } => {
                let llvm_type = match ty {
                    Some(t) => self.resolve_type(t.clone())?,
                    None => self.context.f64_type().into(),
                };
                Ok(Some(llvm_type.into_float_type().const_float(*value).into()))
            }
            Expression::CharLiteral { .. } => {
                Ok(Some(self.context.i8_type().const_int(0, false).into()))
            }
            Expression::Block { statements } => {
                self.enter_scope_with_stmts(statements)?;
                self.resolve_block_stmts(statements)?;

                Ok(Some(self.context.i32_type().const_int(0, false).into()))
            }
            Expression::Variable { name, ty, .. } => {
                if let Some(ptr) = self.lookup_variable(&name.value) {
                    let llvm_type = if let Some(ty) = ty {
                        self.resolve_type(ty.clone())?
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            "Cannot infer type from variable without type annotation",
                            Some(name),
                        );
                        anyhow::bail!("Cannot infer type from variable")
                    };
                    let val = self.builder.build_load(llvm_type, ptr, "")?;
                    Ok(Some(val))
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("Undefined variable: '{}'", name.value),
                        Some(name),
                    );
                    anyhow::bail!("Undefined variable: {}", name.value);
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_val = self.resolve_expression(left.clone())?.unwrap();
                let right_val = self.resolve_expression(right.clone())?.unwrap();
                let left_ty = left.borrow().get_ty().ok().flatten();
                let is_unsigned = matches!(left_ty, Some(ty) if matches!(&*ty.borrow(), Type::UInt8 | Type::UInt16 | Type::UInt32 | Type::UInt64 | Type::UInt128));

                match operator {
                    BinaryOperator::Plus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_int_add(l, r, "")?.into()))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_add(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for addition");
                        }
                    }
                    BinaryOperator::Minus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_int_sub(l, r, "")?.into()))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_sub(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for subtraction");
                        }
                    }
                    BinaryOperator::Multiply => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_int_mul(l, r, "")?.into()))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_mul(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for multiplication");
                        }
                    }
                    BinaryOperator::Divide => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            if is_unsigned {
                                Ok(Some(self.builder.build_int_unsigned_div(l, r, "")?.into()))
                            } else {
                                Ok(Some(self.builder.build_int_signed_div(l, r, "")?.into()))
                            }
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_div(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for division");
                        }
                    }
                    BinaryOperator::Equal => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_int_compare(inkwell::IntPredicate::EQ, l, r, "")?
                                    .into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OEQ, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for equality comparison");
                        }
                    }
                    BinaryOperator::NotEqual => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_int_compare(inkwell::IntPredicate::NE, l, r, "")?
                                    .into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::ONE, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for inequality comparison");
                        }
                    }
                    BinaryOperator::Less => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::ULT
                            } else {
                                inkwell::IntPredicate::SLT
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OLT, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for less than comparison");
                        }
                    }
                    BinaryOperator::LessEqual => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::ULE
                            } else {
                                inkwell::IntPredicate::SLE
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OLE, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for less than or equal comparison");
                        }
                    }
                    BinaryOperator::Greater => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::UGT
                            } else {
                                inkwell::IntPredicate::SGT
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OGT, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for greater than comparison");
                        }
                    }
                    BinaryOperator::GreaterEqual => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::UGE
                            } else {
                                inkwell::IntPredicate::SGE
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OGE, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for greater than or equal comparison");
                        }
                    }
                    BinaryOperator::And => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_and(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for logical and");
                        }
                    }
                    BinaryOperator::Or => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_or(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for logical or");
                        }
                    }
                    BinaryOperator::BitAnd => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_and(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for bitwise and");
                        }
                    }
                    BinaryOperator::BitOr => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_or(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for bitwise or");
                        }
                    }
                    BinaryOperator::BitXor => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_xor(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for bitwise xor");
                        }
                    }
                    BinaryOperator::LeftShift => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_left_shift(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for left shift");
                        }
                    }
                    BinaryOperator::RightShift => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder.build_right_shift(l, r, false, "")?.into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for right shift");
                        }
                    }
                    BinaryOperator::Modulus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            if is_unsigned {
                                Ok(Some(self.builder.build_int_unsigned_rem(l, r, "")?.into()))
                            } else {
                                Ok(Some(self.builder.build_int_signed_rem(l, r, "")?.into()))
                            }
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_rem(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for modulus");
                        }
                    }
                    BinaryOperator::RangeTo | BinaryOperator::RangeUntil => {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Range expressions are not yet supported in IR generation",
                            None,
                        );
                        anyhow::bail!("Range expressions not implemented");
                    }
                }
            }
            Expression::Unary {
                expression: expr,
                operator,
                ..
            } => {
                let expr_val = self.resolve_expression(expr.clone())?.unwrap();
                match operator {
                    UnaryOperator::Plus => Ok(Some(expr_val)),
                    UnaryOperator::Minus => {
                        if let BasicValueEnum::IntValue(v) = expr_val {
                            Ok(Some(self.builder.build_int_neg(v, "")?.into()))
                        } else if let BasicValueEnum::FloatValue(v) = expr_val {
                            Ok(Some(self.builder.build_float_neg(v, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid type for unary minus");
                        }
                    }
                    UnaryOperator::BitNot => {
                        if let BasicValueEnum::IntValue(v) = expr_val {
                            Ok(Some(self.builder.build_not(v, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid type for bitwise not");
                        }
                    }
                    UnaryOperator::Inc => {
                        let one = if let BasicValueEnum::IntValue(val) = expr_val {
                            val.get_type().const_int(1, false).into()
                        } else if let BasicValueEnum::FloatValue(val) = expr_val {
                            val.get_type().const_float(1.0).into()
                        } else {
                            anyhow::bail!("Invalid type for increment");
                        };
                        let result = if let (
                            BasicValueEnum::IntValue(v),
                            BasicValueEnum::IntValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_int_add(v, one_val, "")?.into()
                        } else if let (
                            BasicValueEnum::FloatValue(v),
                            BasicValueEnum::FloatValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_float_add(v, one_val, "")?.into()
                        } else {
                            anyhow::bail!("Invalid type for increment");
                        };
                        if let Expression::Variable { name, .. } = &*expr.borrow()
                            && let Some(ptr) = self.lookup_variable(&name.value)
                        {
                            self.builder.build_store(ptr, result)?;
                        }
                        Ok(Some(result))
                    }
                    UnaryOperator::Dec => {
                        let one = if let BasicValueEnum::IntValue(val) = expr_val {
                            val.get_type().const_int(1, false).into()
                        } else if let BasicValueEnum::FloatValue(val) = expr_val {
                            val.get_type().const_float(1.0).into()
                        } else {
                            anyhow::bail!("Invalid type for decrement");
                        };
                        let result = if let (
                            BasicValueEnum::IntValue(v),
                            BasicValueEnum::IntValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_int_sub(v, one_val, "")?.into()
                        } else if let (
                            BasicValueEnum::FloatValue(v),
                            BasicValueEnum::FloatValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_float_sub(v, one_val, "")?.into()
                        } else {
                            anyhow::bail!("Invalid type for decrement");
                        };
                        if let Expression::Variable { name, .. } = &*expr.borrow()
                            && let Some(ptr) = self.lookup_variable(&name.value)
                        {
                            self.builder.build_store(ptr, result)?;
                        }
                        Ok(Some(result))
                    }
                    UnaryOperator::NotNullAssertation => {
                        // TODO: for now, just pass through the value
                        // In a full implementation, this would check for null/none
                        Ok(Some(expr_val))
                    }
                    UnaryOperator::OpenRange => {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Open range operator is not yet supported in IR generation",
                            None,
                        );
                        anyhow::bail!("Open range not implemented");
                    }
                }
            }
            Expression::Assignment {
                left,
                operator,
                right,
            } => {
                let right_val = self.resolve_expression(right.clone())?.unwrap();

                let (var_ptr, current_val) =
                    if let Expression::Variable { name, .. } = &*left.borrow() {
                        if let Some(ptr) = self.lookup_variable(&name.value) {
                            let ty_opt = left.borrow().get_ty()?;
                            let ty = if let Some(ty_rc) = ty_opt {
                                self.resolve_type(ty_rc)?
                            } else {
                                self.context.i32_type().into()
                            };
                            let val = self.builder.build_load(ty, ptr, "")?;
                            (ptr, Some(val))
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UndefinedVariable,
                                format!("Undefined variable: '{}'", name.value),
                                Some(name),
                            );
                            anyhow::bail!("Undefined variable");
                        }
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Invalid assignment target",
                            None,
                        );
                        anyhow::bail!("Invalid assignment target");
                    };

                let result = match operator {
                    AssignmentOperator::Assign => {
                        self.builder.build_store(var_ptr, right_val)?;
                        right_val
                    }
                    AssignmentOperator::PlusAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_add(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_add(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for += assignment");
                        }
                    }
                    AssignmentOperator::MinusAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_sub(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_sub(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for -= assignment");
                        }
                    }
                    AssignmentOperator::MultiplyAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_mul(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_mul(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for *= assignment");
                        }
                    }
                    AssignmentOperator::DivideAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_signed_div(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_div(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for /= assignment");
                        }
                    }
                    AssignmentOperator::ModulusAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_signed_rem(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_rem(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for %= assignment");
                        }
                    }
                    AssignmentOperator::BitAndAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_and(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for &= assignment");
                        }
                    }
                    AssignmentOperator::BitOrAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_or(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for |= assignment");
                        }
                    }
                    AssignmentOperator::LeftShiftAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_left_shift(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for <<= assignment");
                        }
                    }
                    AssignmentOperator::RightShiftAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_right_shift(l, r, false, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for >>= assignment");
                        }
                    }
                };

                Ok(Some(result))
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

                let cond_val = self
                    .resolve_expression(condition.clone())?
                    .unwrap()
                    .into_int_value();
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

                // Return None as a placeholder
                Ok(None)
            }
            Expression::Call {
                callee, parameters, ..
            } => {
                let function_name = match &*callee.borrow() {
                    Expression::Variable { name, .. } => name.value.clone(),
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Only simple function calls are supported",
                            None,
                        );
                        anyhow::bail!("Unsupported callee");
                    }
                };

                let function = self.module.get_function(&function_name).ok_or_else(|| {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedFunction,
                        format!("Undefined function: '{}'", function_name),
                        None,
                    );
                    anyhow::anyhow!("Undefined function: {}", function_name)
                })?;

                let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
                for param in parameters {
                    let arg_val = self.resolve_expression(param.expression.clone())?.unwrap();
                    args.push(arg_val.into());
                }

                let result = self.builder.build_call(function, &args, "")?;

                match result.try_as_basic_value() {
                    inkwell::values::ValueKind::Basic(val) => Ok(Some(val)),
                    _ => Ok(None),
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
                    None,
                );
                anyhow::bail!("Never type cannot be converted to LLVM type");
            }
            Type::Void => {
                self.emit_error(
                    TrussDiagnosticCode::VoidTypeConversion,
                    "Void type is handled specially as void return type",
                    None,
                );
                anyhow::bail!("Void type is handled specially as void return type");
            }
            Type::Function(_, _, _) => {
                self.emit_error(
                    TrussDiagnosticCode::NestedFunctionType,
                    "Nested function types are not supported",
                    None,
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
            Expression::Variable { ty, name, .. } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty.clone())
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::TypeInferenceFailed,
                        "Cannot infer type from variable without type annotation",
                        Some(name),
                    );
                    anyhow::bail!("Cannot infer type from variable")
                }
            }
            Expression::Unary { expression, .. } => {
                self.infer_type_from_expression(expression.clone())
            }
            Expression::Binary { left, right, .. } => self
                .infer_type_from_expression(left.clone())
                .or_else(|_| self.infer_type_from_expression(right.clone())),
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::TypeInferenceFailed,
                    "Cannot infer type from this expression",
                    None,
                );
                anyhow::bail!("Cannot infer type")
            }
        }
    }

    fn emit_error(
        &self,
        code: TrussDiagnosticCode,
        message: impl Into<String>,
        token: Option<&Token>,
    ) {
        let msg = message.into();
        let diag = if let Some(token) = token {
            new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg))
        } else {
            new_diagnostic(code, msg)
        };
        self.engine.borrow_mut().emit(diag);
    }
}
