use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::{
        expression::{BinaryOperator, CallParameter, Expression, UnaryOperator},
        node::Program,
        statement::{FunctionBody, Statement},
    },
    diag::{
        TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token,
        secondary_label_from_token,
    },
    id::ModuleId,
    krate::{Crate, Module},
    lexer::token::{Position, Token, TokenType},
    types::Type,
};

#[derive(Debug, Clone)]
struct TypeEnv {
    vars: HashMap<String, Rc<RefCell<Type>>>,
    parent: Option<Rc<RefCell<TypeEnv>>>,
}
impl TypeEnv {
    fn new(parent: Option<Rc<RefCell<TypeEnv>>>) -> Self {
        Self {
            vars: HashMap::new(),
            parent,
        }
    }
    fn get(&self, name: &str) -> Option<Rc<RefCell<Type>>> {
        if let Some(ty) = self.vars.get(name) {
            return Some(ty.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.borrow().get(name);
        }
        None
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
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
}

impl TypeResolver {
    pub fn new(krate: Rc<RefCell<Crate>>, engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        Self {
            krate,
            current_module: None,
            current_return_type: None,
            type_env: None,
            engine,
        }
    }

    pub fn resolve(&mut self, program: &Program, id: ModuleId) {
        self.current_module = self.krate.borrow().modules.get(&id).cloned();
        self.type_env = Some(Rc::new(RefCell::new(TypeEnv::new(None))));
        
        for stmt in &program.statements {
            self.process_function_decl(stmt.clone());
        }
        
        for stmt in &program.statements {
            self.resolve_statement(stmt.clone());
        }
    }

    fn process_function_decl(&mut self, statement: Rc<RefCell<Statement>>) {
        if let Statement::FunctionDecl {
            name,
            parameters,
            return_type,
            ty,
            body,
            ..
        } = &mut *statement.borrow_mut()
        {
            let ret_type = if let Some(return_type_expr) = return_type {
                self.infer_type(return_type_expr.clone())
                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)))
            } else {
                Rc::new(RefCell::new(Type::Void))
            };

            let mut parameter_types = Vec::new();
            for param in parameters.iter() {
                let param_type = self.infer_type(param.borrow().type_expression.clone());
                if let Some(ref param_type) = param_type {
                    param.borrow_mut().ty = Some(param_type.clone());
                    parameter_types.push(param_type.clone());
                }
            }

            let fn_type = Rc::new(RefCell::new(Type::Function(
                parameter_types,
                ret_type,
            )));
            *ty = Some(fn_type.clone());

            self.type_env
                .as_ref()
                .unwrap()
                .borrow_mut()
                .set(name.value.clone(), fn_type);

            match &*body.borrow() {
                FunctionBody::Statements(stmts) => {
                    for s in stmts {
                        self.process_function_decl(s.clone());
                    }
                }
                FunctionBody::Expression(expr) => {
                    self.process_function_decl_in_expr(expr.clone());
                }
                FunctionBody::None => {}
            }
        }
    }

    fn process_function_decl_in_expr(&mut self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Block { statements } = &*expr.borrow() {
            for stmt in statements {
                self.process_function_decl(stmt.clone());
            }
        }
    }

    fn resolve_statement(&mut self, statement: Rc<RefCell<Statement>>) {
        match &mut *statement.borrow_mut() {
            Statement::VariableDecl {
                name,
                type_expression,
                initializer,
                ty,
                ..
            } => {
                if let Some(type_expr) = type_expression {
                    let annotated = self.infer_type(type_expr.clone());
                    if let Some(annotated) = annotated {
                        if let Some(init) = initializer {
                            self.check_type_with_expected(
                                init.clone(),
                                annotated.clone(),
                                name.as_ref(),
                            );
                        }
                        *ty = Some(annotated.clone());
                        self.type_env
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set(name.value.clone(), annotated);
                    }
                } else if let Some(init) = initializer {
                    let init_ty = self.infer_type(init.clone());
                    if let Some(init_ty) = init_ty {
                        *ty = Some(init_ty.clone());
                        self.type_env
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set(name.value.clone(), init_ty);
                    }
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingTypeAnnotation,
                        "Variable declaration requires type annotation or initializer",
                        name.as_ref(),
                    );
                };
            }
            Statement::FunctionDecl {
                parameters,
                body,
                ty,
                ..
            } => {
                let last_return_type = self.current_return_type.clone();

                let fn_type = if ty.is_some() {
                    ty.clone().unwrap()
                } else {
                    Rc::new(RefCell::new(Type::Function(vec![], Rc::new(RefCell::new(Type::Void)))))
                };
                
                let ret_type = if let Type::Function(_, ret) = &*fn_type.borrow() {
                    ret.clone()
                } else {
                    Rc::new(RefCell::new(Type::Void))
                };
                self.current_return_type = Some(ret_type.clone());

                let last_type_env = self.type_env.clone();
                self.type_env = Some(Rc::new(RefCell::new(TypeEnv::new(last_type_env.clone()))));
                
                for param in parameters.iter() {
                    if let Some(param_ty) = param.borrow().ty.clone() {
                        self.type_env
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .set(param.borrow().name.value.clone(), param_ty);
                    }
                }

                self.resolve_function_body(body.clone());

                self.type_env = last_type_env;
                self.current_return_type = last_return_type;
            }
            Statement::Return {
                value: Some(value), ..
            } => {
                let token = &Self::get_token_from_expr(value);
                if let Some(expected) = self.current_return_type.clone() {
                    self.check_type_with_expected(value.clone(), expected, token);
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::ReturnTypeMismatch,
                        "Return statement outside function",
                        token,
                    );
                }
            }
            Statement::Return { value: None, token } => {
                if let Some(expected) = self.current_return_type.clone() {
                    if !matches!(&*expected.borrow(), Type::Void) {
                        self.emit_error(
                            TrussDiagnosticCode::ReturnTypeMismatch,
                            format!(
                                "Expected return value of type {:?}, found `return` without value",
                                expected.borrow()
                            ),
                            token.as_ref(),
                        );
                    }
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::ReturnTypeMismatch,
                        "Return statement outside function",
                        token.as_ref(),
                    );
                }
            }
            Statement::ExpressionStatement { expression } => {
                self.infer_type(expression.clone());
            }
            Statement::While { condition, body } => {
                let cond_ty = self.infer_type(condition.clone());
                if let Some(cond_ty) = cond_ty
                    && *cond_ty.borrow() != Type::Bool
                {
                    self.emit_error(
                        TrussDiagnosticCode::InvalidConditionType,
                        format!("While condition must be Bool, found {:?}", cond_ty.borrow()),
                        &Self::get_token_from_expr(condition),
                    );
                }
                self.resolve_block_expression(body.clone());
            }
            Statement::Loop { body } => {
                self.resolve_block_expression(body.clone());
            }
            Statement::RepeatWhile { body, condition } => {
                self.resolve_block_expression(body.clone());
                let cond_ty = self.infer_type(condition.clone());
                if let Some(cond_ty) = cond_ty
                    && *cond_ty.borrow() != Type::Bool
                {
                    self.emit_error(
                        TrussDiagnosticCode::InvalidConditionType,
                        format!(
                            "Repeat-while condition must be Bool, found {:?}",
                            cond_ty.borrow()
                        ),
                        &Self::get_token_from_expr(condition),
                    );
                }
            }
            Statement::For {
                iterator,
                body,
                ..
            } => {
                let _ = self.infer_type(iterator.clone());
                self.resolve_block_expression(body.clone());
            }
            _ => {}
        }
    }

    fn resolve_block_expression(&mut self, block_expr: Rc<RefCell<Expression>>) {
        if let Expression::Block { statements } = &*block_expr.borrow() {
            for stmt in statements {
                self.resolve_statement(stmt.clone());
            }
        }
    }

    fn resolve_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) {
        match &mut *body.borrow_mut() {
            FunctionBody::Statements(statements) => {
                for stmt in statements {
                    self.resolve_statement(stmt.clone());
                }
            }
            FunctionBody::Expression(expression) => {
                if let Some(expected) = self.current_return_type.clone() {
                    let token = &Self::get_token_from_expr(expression);
                    self.check_type_with_expected(expression.clone(), expected, token);
                }
            }
            FunctionBody::None => {}
        }
    }

    fn resolve_type_name(&self, name: &str, token: &Token) -> Option<Rc<RefCell<Type>>> {
        match name {
            "Int8" => Some(Rc::new(RefCell::new(Type::Int8))),
            "Int16" => Some(Rc::new(RefCell::new(Type::Int16))),
            "Int32" => Some(Rc::new(RefCell::new(Type::Int32))),
            "Int64" => Some(Rc::new(RefCell::new(Type::Int64))),
            "Int128" => Some(Rc::new(RefCell::new(Type::Int128))),
            "UInt8" => Some(Rc::new(RefCell::new(Type::UInt8))),
            "UInt16" => Some(Rc::new(RefCell::new(Type::UInt16))),
            "UInt32" => Some(Rc::new(RefCell::new(Type::UInt32))),
            "UInt64" => Some(Rc::new(RefCell::new(Type::UInt64))),
            "UInt128" => Some(Rc::new(RefCell::new(Type::UInt128))),
            "Float32" => Some(Rc::new(RefCell::new(Type::Float32))),
            "Float64" => Some(Rc::new(RefCell::new(Type::Float64))),
            "Bool" => Some(Rc::new(RefCell::new(Type::Bool))),
            "Void" => Some(Rc::new(RefCell::new(Type::Void))),
            "Char" => Some(Rc::new(RefCell::new(Type::Char))),
            "Never" => Some(Rc::new(RefCell::new(Type::Never))),
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::UnknownType,
                    format!("Unknown type '{}'", name),
                    token,
                );
                None
            }
        }
    }

    fn infer_type(&mut self, expression: Rc<RefCell<Expression>>) -> Option<Rc<RefCell<Type>>> {
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
                    .ok_or_else(|| {
                        self.emit_error(
                            TrussDiagnosticCode::TypeError,
                            "No type environment available",
                            name.as_ref(),
                        );
                    })
                    .ok()?
                    .borrow()
                    .get(&name.value);
                let t = t?;
                *ty = Some(t.clone());
                t
            }
            Expression::Type { name, ty, .. } => {
                let t = self.resolve_type_name(&name.value, name.as_ref())?;
                *ty = Some(t.clone());
                t
            }
            Expression::Block { statements } => {
                let mut last_ty = Rc::new(RefCell::new(Type::Void));
                for stmt in statements.iter() {
                    if let Some(ty) = self.infer_statement_type(stmt.clone()) {
                        last_ty = ty;
                    }
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

                {
                    let mut right_mut = right.borrow_mut();
                    match &mut *right_mut {
                        Expression::IntegerLiteral { ty, .. } if ty.is_none() => {
                            if Self::is_integer_type(&left_ty.borrow()) {
                                *ty = Some(left_ty.clone());
                            }
                        }
                        Expression::DecimalLiteral { ty, .. } if ty.is_none() => {
                            if Self::is_float_type(&left_ty.borrow()) {
                                *ty = Some(left_ty.clone());
                            }
                        }
                        _ => {}
                    }
                }

                let right_ty = self.infer_type(right.clone())?;
                if let Some(result) =
                    self.check_binary(*operator, left_ty.clone(), right_ty.clone())
                {
                    result
                } else {
                    let token = Self::get_token_from_expr(left);
                    self.emit_error(
                        TrussDiagnosticCode::InvalidOperand,
                        format!(
                            "Invalid operands for binary operator: {:?} and {:?}",
                            left_ty.borrow().clone(),
                            right_ty.borrow().clone()
                        ),
                        &token,
                    );
                    return None;
                }
            }
            Expression::Unary {
                expression,
                operator,
                ..
            } => {
                let operand_ty = self.infer_type(expression.clone())?;
                if let Some(result) = self.check_unary(*operator, operand_ty.clone()) {
                    result
                } else {
                    let token = Self::get_token_from_expr(expression);
                    self.emit_error(
                        TrussDiagnosticCode::InvalidOperand,
                        format!(
                            "Invalid operand for unary operator: {:?}",
                            operand_ty.borrow().clone()
                        ),
                        &token,
                    );
                    return None;
                }
            }
            Expression::Call {
                callee, parameters, ..
            } => {
                let callee_type = self.infer_type(callee.clone())?;
                match &*callee_type.borrow() {
                    Type::Function(param_tys, ret_ty) => {
                        if parameters.len() != param_tys.len() {
                            let token = &Self::get_token_from_expr(callee);
                            self.emit_error(
                                TrussDiagnosticCode::ArgumentCountMismatch,
                                format!(
                                    "Expected {} arguments but found {}",
                                    param_tys.len(),
                                    parameters.len()
                                ),
                                token,
                            );
                        }
                        
                        let func_decl = self.get_function_decl_from_callee(callee.clone());
                        
                        for (i, param) in parameters.iter().enumerate() {
                            if i < param_tys.len() {
                                let expected_ty = param_tys[i].clone();
                                self.infer_expression_type(param.expression.clone(), expected_ty);
                                
                                if let Some(ref decl) = func_decl {
                                    self.check_parameter_label(param, decl, i);
                                }
                            }
                        }
                        ret_ty.clone()
                    }
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::CallingNonFunction,
                            format!("Cannot call non-function type {:?}", callee_type.borrow()),
                            &Self::get_token_from_expr(callee),
                        );
                        return None;
                    }
                }
            }
            Expression::Assignment { left, right, .. } => {
                let left_ty = self.infer_type(left.clone())?;
                let right_ty = self.infer_type(right.clone())?;
                if left_ty.borrow().clone() != right_ty.borrow().clone() {
                    let expected_msg = format!("expected {:?}", left_ty.borrow().clone());
                    let found_msg = format!("found {:?}", right_ty.borrow().clone());
                    self.emit_error_with_labels(
                        TrussDiagnosticCode::TypeMismatch,
                        format!(
                            "Type mismatch in assignment: {:?} vs {:?}",
                            left_ty.borrow().clone(),
                            right_ty.borrow().clone()
                        ),
                        primary_label_from_token(&Self::get_token_from_expr(left), &expected_msg),
                        secondary_label_from_token(&Self::get_token_from_expr(right), &found_msg),
                    );
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
                    self.emit_error(
                        TrussDiagnosticCode::InvalidConditionType,
                        format!("If condition must be Bool, found {:?}", cond_ty.borrow()),
                        &Self::get_token_from_expr(condition),
                    );
                }
                let then_ty = self.infer_type(then.clone())?;
                if let Some(else_expr) = else_ {
                    let else_ty = self.infer_type(else_expr.clone())?;
                    if then_ty.borrow().clone() != else_ty.borrow().clone() {
                        self.emit_error(
                            TrussDiagnosticCode::BranchTypeMismatch,
                            format!(
                                "If branches have different types: {:?} vs {:?}",
                                then_ty.borrow().clone(),
                                else_ty.borrow().clone()
                            ),
                            &Self::get_token_from_expr(then),
                        );
                    }
                }
                then_ty
            }
            Expression::VoidLiteral { .. } => Rc::new(RefCell::new(Type::Void)),
            Expression::NullLiteral { .. } => Rc::new(RefCell::new(Type::Void)),
            Expression::NullptrLiteral { .. } => Rc::new(RefCell::new(Type::Void)),
            Expression::CharLiteral { .. } => Rc::new(RefCell::new(Type::Char)),
        };
        Some(result)
    }

    fn infer_statement_type(
        &mut self,
        statement: Rc<RefCell<Statement>>,
    ) -> Option<Rc<RefCell<Type>>> {
        match &*statement.borrow() {
            Statement::ExpressionStatement { expression } => self.infer_type(expression.clone()),
            Statement::Return {
                value: Some(value), ..
            } => self.infer_type(value.clone()),
            Statement::Return { value: None, .. } => Some(Rc::new(RefCell::new(Type::Void))),
            Statement::VariableDecl { ty, .. } => {
                Some(ty.clone().unwrap_or(Rc::new(RefCell::new(Type::Void))))
            }
            _ => Some(Rc::new(RefCell::new(Type::Void))),
        }
    }

    fn check_type_with_expected(
        &mut self,
        expression: Rc<RefCell<Expression>>,
        expected: Rc<RefCell<Type>>,
        token: &Token,
    ) {
        let is_int_literal = matches!(&*expression.borrow(), Expression::IntegerLiteral { .. });
        let is_float_literal = matches!(&*expression.borrow(), Expression::DecimalLiteral { .. });

        if is_int_literal {
            if Self::is_integer_type(&expected.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::IntegerLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {:?}, found integer literal",
                        expected.borrow().clone()
                    ),
                    token,
                );
            }
        } else if is_float_literal {
            if Self::is_float_type(&expected.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::DecimalLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected.clone());
                }
                drop(expr_mut);
            } else {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {:?}, found float literal",
                        expected.borrow().clone()
                    ),
                    token,
                );
            }
        } else if let Some(inferred) = self.infer_type(expression) {
            if inferred.borrow().clone() != expected.borrow().clone() {
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {:?}, found {:?}",
                        expected.borrow().clone(),
                        inferred.borrow().clone()
                    ),
                    token,
                );
            }
        } else {
            self.emit_error(
                TrussDiagnosticCode::TypeMismatch,
                format!("Type mismatch: expected {:?}", expected.borrow().clone()),
                token,
            );
        }
    }

    fn is_integer_type(ty: &Type) -> bool {
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
        )
    }

    fn is_float_type(ty: &Type) -> bool {
        matches!(ty, Type::Float32 | Type::Float64)
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

    fn infer_expression_type(
        &mut self,
        expression: Rc<RefCell<Expression>>,
        expected_type: Rc<RefCell<Type>>,
    ) -> Option<Rc<RefCell<Type>>> {
        let expr_ref = expression.borrow();
        let is_int_literal = matches!(&*expr_ref, Expression::IntegerLiteral { .. });
        let is_float_literal = matches!(&*expr_ref, Expression::DecimalLiteral { .. });
        drop(expr_ref);

        if is_int_literal {
            if Self::is_integer_type(&expected_type.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::IntegerLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected_type.clone());
                }
                return Some(expected_type);
            } else {
                let token = Self::get_token_from_expr(&expression);
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {:?}, found integer literal",
                        expected_type.borrow().clone()
                    ),
                    &token,
                );
                return None;
            }
        }

        if is_float_literal {
            if Self::is_float_type(&expected_type.borrow()) {
                let mut expr_mut = expression.borrow_mut();
                if let Expression::DecimalLiteral { ty, .. } = &mut *expr_mut {
                    *ty = Some(expected_type.clone());
                }
                return Some(expected_type);
            } else {
                let token = Self::get_token_from_expr(&expression);
                self.emit_error(
                    TrussDiagnosticCode::TypeMismatch,
                    format!(
                        "Type mismatch: expected {:?}, found float literal",
                        expected_type.borrow().clone()
                    ),
                    &token,
                );
                return None;
            }
        }

        self.infer_type(expression)
    }

    fn check_binary(
        &self,
        operator: BinaryOperator,
        left: Rc<RefCell<Type>>,
        right: Rc<RefCell<Type>>,
    ) -> Option<Rc<RefCell<Type>>> {
        match operator {
            BinaryOperator::Plus
            | BinaryOperator::Minus
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulus => {
                let left_ty = left.borrow().clone();
                let right_ty = right.borrow().clone();

                if !Self::is_numeric_type(&left_ty) {
                    return None;
                }
                if left_ty != right_ty {
                    return None;
                }
                Some(Rc::new(RefCell::new(left_ty)))
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                if left.borrow().clone() != right.borrow().clone() {
                    return None;
                }
                Some(Rc::new(RefCell::new(Type::Bool)))
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if *left.borrow() != Type::Bool {
                    return None;
                }
                if *right.borrow() != Type::Bool {
                    return None;
                }
                Some(Rc::new(RefCell::new(Type::Bool)))
            }
            _ => None,
        }
    }

    fn check_unary(
        &self,
        operator: UnaryOperator,
        operand: Rc<RefCell<Type>>,
    ) -> Option<Rc<RefCell<Type>>> {
        match operator {
            UnaryOperator::Plus | UnaryOperator::Minus => {
                let op_ty = operand.borrow().clone();
                if !Self::is_numeric_type(&op_ty) {
                    return None;
                }
                Some(Rc::new(RefCell::new(op_ty)))
            }
            UnaryOperator::Inc | UnaryOperator::Dec => {
                let op_ty = operand.borrow().clone();
                if !Self::is_numeric_type(&op_ty) {
                    return None;
                }
                Some(Rc::new(RefCell::new(op_ty)))
            }
            UnaryOperator::BitNot => {
                let op_ty = operand.borrow().clone();
                if !Self::is_integer_type(&op_ty) {
                    return None;
                }
                Some(Rc::new(RefCell::new(op_ty)))
            }
            _ => None,
        }
    }

    fn get_token_from_expr(expr: &Rc<RefCell<Expression>>) -> Token {
        match &*expr.borrow() {
            Expression::IntegerLiteral { token, .. } => (**token).clone(),
            Expression::DecimalLiteral { token, .. } => (**token).clone(),
            Expression::BooleanLiteral { token } => (**token).clone(),
            Expression::NullLiteral { token } => (**token).clone(),
            Expression::NullptrLiteral { token } => (**token).clone(),
            Expression::CharLiteral { token } => (**token).clone(),
            Expression::VoidLiteral { left, .. } => (**left).clone(),
            Expression::Variable { name, .. } => (**name).clone(),
            Expression::Type { name, .. } => (**name).clone(),
            Expression::Unary { expression, .. } => Self::get_token_from_expr(expression),
            Expression::Binary { left, .. } => Self::get_token_from_expr(left),
            Expression::Call { callee, .. } => Self::get_token_from_expr(callee),
            Expression::Assignment { left, .. } => Self::get_token_from_expr(left),
            Expression::If { condition, .. } => Self::get_token_from_expr(condition),
            Expression::Block { statements } => {
                if let Some(last) = statements.last() {
                    match &*last.borrow() {
                        Statement::ExpressionStatement { expression } => {
                            Self::get_token_from_expr(expression)
                        }
                        Statement::Return {
                            value: Some(value), ..
                        } => Self::get_token_from_expr(value),
                        Statement::Return { token, .. } => (**token).clone(),
                        _ => Token::new(
                            "".to_string(),
                            TokenType::Identifier,
                            Position {
                                pos: 0,
                                line: 0,
                                col: 0,
                                len: 0,
                            },
                            Rc::new("".to_string()),
                        ),
                    }
                } else {
                    Token::new(
                        "".to_string(),
                        TokenType::Identifier,
                        Position {
                            pos: 0,
                            line: 0,
                            col: 0,
                            len: 0,
                        },
                        Rc::new("".to_string()),
                    )
                }
            }
        }
    }

    fn get_function_decl_from_callee(&self, callee: Rc<RefCell<Expression>>) -> Option<Rc<RefCell<Statement>>> {
        if let Expression::Variable { symbol, .. } = &*callee.borrow() {
            if let Some(sym) = symbol {
                if let Ok(Some(decl)) = sym.get_decl() {
                    return Some(decl);
                }
            }
        }
        None
    }

    fn check_parameter_label(
        &self,
        call_param: &CallParameter,
        func_decl: &Rc<RefCell<Statement>>,
        param_index: usize,
    ) {
        if let Statement::FunctionDecl { parameters, .. } = &*func_decl.borrow() {
            if param_index >= parameters.len() {
                return;
            }
            
            let decl_param = &parameters[param_index];
            let decl_param_label = &decl_param.borrow().label;
            let decl_param_name = &decl_param.borrow().name;
            let provided_label = &call_param.label;

            let expected_label: Option<&Token>;

            if let Some(label) = decl_param_label {
                if label.value == "_" {
                    expected_label = None;
                } else {
                    expected_label = Some(label);
                }
            } else {
                expected_label = Some(decl_param_name);
            }

            match (expected_label, provided_label) {
                (Some(expected), Some(provided)) => {
                    if expected.value != provided.value {
                        self.emit_error(
                            TrussDiagnosticCode::ArgumentLabelMismatch,
                            format!(
                                "Expected argument label '{}' but found '{}'",
                                expected.value, provided.value
                            ),
                            provided,
                        );
                    }
                }
                (Some(expected), None) => {
                    let token = Self::get_token_from_expr(&call_param.expression);
                    self.emit_error(
                        TrussDiagnosticCode::MissingArgumentLabel,
                        format!(
                            "Missing argument label '{}' in call",
                            expected.value
                        ),
                        &token,
                    );
                }
                (None, Some(provided)) => {
                    if provided.value != "_" {
                        self.emit_error(
                            TrussDiagnosticCode::ArgumentLabelMismatch,
                            format!(
                                "Argument should not have a label, but found '{}'",
                                provided.value
                            ),
                            provided,
                        );
                    }
                }
                (None, None) => {
                }
            }
        }
    }

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>, token: &Token) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg));
        self.engine.borrow_mut().emit(diag);
    }

    fn emit_error_with_labels(
        &self,
        code: TrussDiagnosticCode,
        message: impl Into<String>,
        primary: duck_diagnostic::Label,
        secondary: duck_diagnostic::Label,
    ) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg)
            .with_label(primary)
            .with_label(secondary);
        self.engine.borrow_mut().emit(diag);
    }
}
