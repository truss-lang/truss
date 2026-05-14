use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{Ok, Result, anyhow};

use crate::{
    ast::{
        expression::{BinaryOperator, Expression, UnaryOperator},
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
    fn get(&self, name: &str) -> Result<Rc<RefCell<Type>>> {
        self.vars
            .get(name)
            .cloned()
            .ok_or(anyhow!("Not found variable: {}", name))
    }
    fn set(&mut self, name: String, ty: Rc<RefCell<Type>>) {
        self.vars.insert(name, ty);
    }
}

#[derive(Debug)]
pub struct TypeResolver {
    pub krate: Rc<RefCell<Crate>>,
    current_module: Option<Rc<RefCell<Module>>>,
    current_return_type: Option<Rc<RefCell<Type>>>,
    type_env: Option<Rc<RefCell<TypeEnv>>>,
}

impl TypeResolver {
    pub fn new(krate: Rc<RefCell<Crate>>) -> Self {
        Self {
            krate,
            current_module: None,
            current_return_type: None,
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
                if let Some(type_expr) = type_expression {
                    let annotated = self.infer_type(type_expr.clone())?;
                    if let Some(init) = initializer {
                        self.check_type_with_expected(init.clone(), annotated.clone())?;
                    }
                    *ty = Some(annotated.clone());
                    self.type_env
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set(name.value.clone(), annotated);
                } else if let Some(init) = initializer {
                    let init_ty = self.infer_type(init.clone())?;
                    *ty = Some(init_ty.clone());
                    self.type_env
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .set(name.value.clone(), init_ty);
                } else {
                    return Err(anyhow!(
                        "Variable declaration requires type annotation or initializer"
                    ));
                };
            }
            Statement::FunctionDecl {
                parameters,
                return_type,
                body,
                ty,
                ..
            } => {
                let last_type_env = self.type_env.clone();
                let last_return_type = self.current_return_type.clone();

                self.type_env = Some(Rc::new(RefCell::new(TypeEnv::default())));

                let ret_type = if let Some(return_type_expr) = return_type {
                    self.infer_type(return_type_expr.clone())?
                } else {
                    Rc::new(RefCell::new(Type::Unit))
                };
                self.current_return_type = Some(ret_type.clone());

                let mut parameter_types = Vec::new();
                for param in parameters {
                    self.resolve_param(param.clone())?;
                    parameter_types.push(param.borrow().ty.clone().unwrap());
                }
                *ty = Some(Rc::new(RefCell::new(Type::Function(
                    parameter_types,
                    ret_type,
                ))));

                self.resolve_function_body(body.clone())?;

                self.type_env = last_type_env;
                self.current_return_type = last_return_type;
            }
            Statement::Return { value: Some(value) } => {
                let expected = self
                    .current_return_type
                    .clone()
                    .ok_or(anyhow!("return outside function"))?;
                self.check_type_with_expected(value.clone(), expected)?;
            }
            Statement::ExpressionStatement { expression } => {
                self.infer_type(expression.clone())?;
            }
            _ => {}
        }
        Ok(())
    }

    fn resolve_param(&mut self, param: Rc<RefCell<Parameter>>) -> Result<()> {
        let param_type = self.infer_type(param.borrow().type_expression.clone())?;
        param.borrow_mut().ty = Some(param_type.clone());
        self.type_env
            .as_ref()
            .unwrap()
            .borrow_mut()
            .set(param.borrow().name.value.clone(), param_type);
        Ok(())
    }

    fn resolve_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) -> Result<()> {
        match &mut *body.borrow_mut() {
            FunctionBody::Statements(statements) => {
                for stmt in statements {
                    self.resolve_statement(stmt.clone())?;
                }
            }
            FunctionBody::Expression(expression) => {
                let expected = self
                    .current_return_type
                    .clone()
                    .ok_or(anyhow!("expression body outside function"))?;
                self.check_type_with_expected(expression.clone(), expected)?;
            }
        }
        Ok(())
    }

    fn resolve_type_name(&self, name: &str) -> Result<Rc<RefCell<Type>>> {
        match name {
            "Int8" => Ok(Rc::new(RefCell::new(Type::Int8))),
            "Int16" => Ok(Rc::new(RefCell::new(Type::Int16))),
            "Int32" => Ok(Rc::new(RefCell::new(Type::Int32))),
            "Int64" => Ok(Rc::new(RefCell::new(Type::Int64))),
            "Int128" => Ok(Rc::new(RefCell::new(Type::Int128))),
            "UInt8" => Ok(Rc::new(RefCell::new(Type::UInt8))),
            "UInt16" => Ok(Rc::new(RefCell::new(Type::UInt16))),
            "UInt32" => Ok(Rc::new(RefCell::new(Type::UInt32))),
            "UInt64" => Ok(Rc::new(RefCell::new(Type::UInt64))),
            "UInt128" => Ok(Rc::new(RefCell::new(Type::UInt128))),
            "Float32" => Ok(Rc::new(RefCell::new(Type::Float32))),
            "Float64" => Ok(Rc::new(RefCell::new(Type::Float64))),
            "Bool" => Ok(Rc::new(RefCell::new(Type::Bool))),
            "Unit" => Ok(Rc::new(RefCell::new(Type::Unit))),
            "Char" => Ok(Rc::new(RefCell::new(Type::Char))),
            "Never" => Ok(Rc::new(RefCell::new(Type::Never))),
            _ => Err(anyhow!("Unknown type: {}", name)),
        }
    }

    fn infer_type(&mut self, expression: Rc<RefCell<Expression>>) -> Result<Rc<RefCell<Type>>> {
        let result = match &mut *expression.borrow_mut() {
            Expression::IntegerLiteral { ty, .. } => {
                if ty.is_none() {
                    *ty = Some(Rc::new(RefCell::new(Type::Int32)));
                }
                ty.clone().unwrap()
            }
            Expression::DecimalLiteral { ty, .. } => {
                if ty.is_none() {
                    *ty = Some(Rc::new(RefCell::new(Type::Float64)));
                }
                ty.clone().unwrap()
            }
            Expression::BooleanLiteral { .. } => Rc::new(RefCell::new(Type::Bool)),
            Expression::Variable { name, ty, .. } => {
                let t = self
                    .type_env
                    .as_ref()
                    .ok_or(anyhow!("No type environment"))?
                    .borrow()
                    .get(&name.value)?;
                *ty = Some(t.clone());
                t
            }
            Expression::Type { name, ty, .. } => {
                let t = self.resolve_type_name(&name.value)?;
                *ty = Some(t.clone());
                t
            }
            Expression::Block { statements } => {
                let mut last_ty = Rc::new(RefCell::new(Type::Unit));
                for stmt in statements.iter() {
                    last_ty = self.infer_statement_type(stmt.clone())?;
                }
                last_ty
            }
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => {
                let left_ty = self.infer_type(left.clone())?;
                let right_ty = self.infer_type(right.clone())?;
                self.check_binary(*operator, left_ty, right_ty)?
            }
            Expression::Unary {
                expression,
                operator,
                ..
            } => {
                let operand_ty = self.infer_type(expression.clone())?;
                self.check_unary(*operator, operand_ty)?
            }
            Expression::Call {
                callee,
                type_parameters: _,
                parameters,
                ..
            } => {
                let callee_type = self.infer_type(callee.clone())?;
                match &*callee_type.borrow() {
                    Type::Function(param_tys, ret_ty) => {
                        for (i, param) in parameters.iter().enumerate() {
                            if i < param_tys.len() {
                                self.check_type(param.clone(), param_tys[i].clone())?;
                            }
                        }
                        ret_ty.clone()
                    }
                    _ => return Err(anyhow!("Calling non-function type")),
                }
            }
            Expression::Assignment {
                left,
                operator: _,
                right,
                ..
            } => {
                let left_ty = self.infer_type(left.clone())?;
                let right_ty = self.infer_type(right.clone())?;
                if left_ty.borrow().clone() != right_ty.borrow().clone() {
                    return Err(anyhow!(
                        "Type mismatch in assignment: left is {:?}, right is {:?}",
                        left_ty.borrow().clone(),
                        right_ty.borrow().clone()
                    ));
                }
                left_ty
            }
            Expression::If {
                condition,
                then,
                else_,
                ..
            } => {
                let cond_ty = self.infer_type(condition.clone())?;
                if *cond_ty.borrow() != Type::Bool {
                    return Err(anyhow!(
                        "If condition must be Bool, got {:?}",
                        cond_ty.borrow()
                    ));
                }
                let then_ty = self.infer_type(then.clone())?;
                if let Some(else_expr) = else_ {
                    let else_ty = self.infer_type(else_expr.clone())?;
                    if then_ty.borrow().clone() != else_ty.borrow().clone() {
                        return Err(anyhow!(
                            "If branches have different types: {:?} vs {:?}",
                            then_ty.borrow().clone(),
                            else_ty.borrow().clone()
                        ));
                    }
                }
                then_ty
            }
            Expression::UnitLiteral { .. } => Rc::new(RefCell::new(Type::Unit)),
            Expression::NullLiteral { .. } => Rc::new(RefCell::new(Type::Unit)),
            Expression::NullptrLiteral { .. } => Rc::new(RefCell::new(Type::Unit)),
            Expression::CharLiteral { .. } => Rc::new(RefCell::new(Type::Char)),
        };
        Ok(result)
    }

    fn infer_statement_type(
        &mut self,
        statement: Rc<RefCell<Statement>>,
    ) -> Result<Rc<RefCell<Type>>> {
        match &*statement.borrow() {
            Statement::ExpressionStatement { expression } => self.infer_type(expression.clone()),
            Statement::Return { value: Some(value) } => self.infer_type(value.clone()),
            Statement::VariableDecl { ty, .. } => {
                Ok(ty.clone().unwrap_or(Rc::new(RefCell::new(Type::Unit))))
            }
            _ => Ok(Rc::new(RefCell::new(Type::Unit))),
        }
    }

    fn check_type(
        &mut self,
        expression: Rc<RefCell<Expression>>,
        expected: Rc<RefCell<Type>>,
    ) -> Result<()> {
        let inferred = self.infer_type(expression)?;
        if inferred.borrow().clone() != expected.borrow().clone() {
            return Err(anyhow!(
                "Type mismatch: expected {:?}, got {:?}",
                expected.borrow().clone(),
                inferred.borrow().clone()
            ));
        }
        Ok(())
    }

    fn check_type_with_expected(
        &mut self,
        expression: Rc<RefCell<Expression>>,
        expected: Rc<RefCell<Type>>,
    ) -> Result<()> {
        let is_literal = matches!(
            &*expression.borrow(),
            Expression::IntegerLiteral { .. } | Expression::DecimalLiteral { .. }
        );

        if is_literal {
            let mut expr_mut = expression.borrow_mut();
            match &mut *expr_mut {
                Expression::IntegerLiteral { ty, .. } => {
                    *ty = Some(expected.clone());
                }
                Expression::DecimalLiteral { ty, .. } => {
                    *ty = Some(expected.clone());
                }
                _ => unreachable!(),
            }
            drop(expr_mut);
            Ok(())
        } else {
            let inferred = self.infer_type(expression)?;
            if inferred.borrow().clone() != expected.borrow().clone() {
                return Err(anyhow!(
                    "Type mismatch: expected {:?}, got {:?}",
                    expected.borrow().clone(),
                    inferred.borrow().clone()
                ));
            }
            Ok(())
        }
    }

    fn check_binary(
        &self,
        operator: BinaryOperator,
        left: Rc<RefCell<Type>>,
        right: Rc<RefCell<Type>>,
    ) -> Result<Rc<RefCell<Type>>> {
        match operator {
            BinaryOperator::Plus
            | BinaryOperator::Minus
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulus => {
                let left_ty = left.borrow().clone();
                let right_ty = right.borrow().clone();
                
                if !Self::is_numeric_type(&left_ty) {
                    return Err(anyhow!(
                        "Arithmetic operator requires numeric operand, got {:?}",
                        left_ty
                    ));
                }
                if left_ty != right_ty {
                    return Err(anyhow!(
                        "Arithmetic operands must have same type: {:?} vs {:?}",
                        left_ty,
                        right_ty
                    ));
                }
                Ok(Rc::new(RefCell::new(left_ty)))
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                if left.borrow().clone() != right.borrow().clone() {
                    return Err(anyhow!(
                        "Comparison operands must have same type: {:?} vs {:?}",
                        left.borrow().clone(),
                        right.borrow().clone()
                    ));
                }
                Ok(Rc::new(RefCell::new(Type::Bool)))
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if *left.borrow() != Type::Bool {
                    return Err(anyhow!(
                        "Logical operator requires Bool left operand, got {:?}",
                        left.borrow()
                    ));
                }
                if *right.borrow() != Type::Bool {
                    return Err(anyhow!(
                        "Logical operator requires Bool right operand, got {:?}",
                        right.borrow()
                    ));
                }
                Ok(Rc::new(RefCell::new(Type::Bool)))
            }
            _ => Err(anyhow!("Unsupported binary operator")),
        }
    }

    fn is_numeric_type(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Int128
                | Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::UInt128
                | Type::Float32
                | Type::Float64
        )
    }

    fn check_unary(
        &self,
        operator: UnaryOperator,
        operand: Rc<RefCell<Type>>,
    ) -> Result<Rc<RefCell<Type>>> {
        match operator {
            UnaryOperator::Plus | UnaryOperator::Minus => {
                let op_ty = operand.borrow().clone();
                if !Self::is_numeric_type(&op_ty) {
                    return Err(anyhow!(
                        "Unary arithmetic requires numeric operand, got {:?}",
                        op_ty
                    ));
                }
                Ok(Rc::new(RefCell::new(op_ty)))
            }
            UnaryOperator::Inc | UnaryOperator::Dec => {
                let op_ty = operand.borrow().clone();
                if !Self::is_numeric_type(&op_ty) {
                    return Err(anyhow!(
                        "Inc/Dec requires numeric operand, got {:?}",
                        op_ty
                    ));
                }
                Ok(Rc::new(RefCell::new(op_ty)))
            }
            _ => Err(anyhow!("Unsupported unary operator")),
        }
    }
}
