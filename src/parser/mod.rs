pub mod precedence;

use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        expression::{
            AssignmentOperator, BinaryOperator, CallParameter, CastKind, Expression, UnaryOperator,
        },
        node::Program,
        statement::{
            AccessModifier, Accessor, AccessorKind, EnumCase, EnumCaseParameter, FunctionBody,
            Modifier, ModifierType, Parameter, Pattern, ProtocolAccessorSet, ProtocolMember,
            Statement, VariadicKind,
        },
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    lexer::token::{KeywordType, OperatorType, Position, SeparatorType, Token, TokenType},
    parser::precedence::Precedence,
};

#[derive(Debug)]
pub struct Parser {
    file: Rc<String>,
    tokens: Vec<Token>,
    index: usize,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
}

impl Parser {
    pub fn new(
        file: Rc<String>,
        tokens: Vec<Token>,
        engine: Rc<RefCell<TrussDiagnosticEngine>>,
    ) -> Self {
        Self {
            file,
            tokens,
            index: 0,
            engine,
        }
    }

    pub fn get_file(&mut self) -> Rc<String> {
        self.file.clone()
    }

    fn is_empty(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn peek(&self) -> Option<Token> {
        if self.index < self.tokens.len() {
            Some(self.tokens[self.index].clone())
        } else {
            None
        }
    }

    fn peek2(&self) -> Option<Token> {
        if self.index + 1 < self.tokens.len() {
            Some(self.tokens[self.index + 1].clone())
        } else {
            None
        }
    }

    fn next(&mut self) -> Option<Token> {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            Some(token)
        } else {
            None
        }
    }

    fn skip(&mut self) {
        if !self.is_empty() {
            self.index += 1;
        }
    }

    pub fn parse(&mut self) -> Program {
        let mut program = Program::new(self.file.clone());
        while !self.is_empty() {
            if let Ok(statement) = self.parse_statement() {
                program.statements.push(Rc::new(RefCell::new(statement)));
            } else {
                self.skip();
            }
        }
        program
    }

    fn parse_statement(&mut self) -> Result<Statement, ()> {
        let modifiers = self.parse_modifiers()?;
        let Some(token) = self.peek() else {
            return Err(());
        };
        match token.ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(false, modifiers),
                KeywordType::Let | KeywordType::Var => self.parse_variable_decl(false, modifiers),
                KeywordType::Struct => self.parse_struct_decl(modifiers),
                KeywordType::Class => self.parse_class_decl(modifiers),
                KeywordType::Protocol => self.parse_protocol_decl(modifiers),
                KeywordType::Enum => self.parse_enum_decl(modifiers),
                KeywordType::Extern => self.parse_extern(modifiers),
                KeywordType::Init => self.parse_function_decl(false, modifiers),
                KeywordType::Deinit => self.parse_deinit_decl(modifiers),
                KeywordType::Return => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_return()
                }
                KeywordType::Loop => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_loop()
                }
                KeywordType::While => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_while()
                }
                KeywordType::Repeat => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_repeat_while()
                }
                KeywordType::For => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_for()
                }
                KeywordType::Throw => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_throw()
                }
                _ => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    Ok(Statement::ExpressionStatement {
                        expression: Rc::new(RefCell::new(self.parse_expression()?)),
                    })
                }
            },
            TokenType::Separator { separator } => match separator {
                SeparatorType::SemiColon => {
                    self.index += 1;
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            "Modifiers are not allowed on empty statement.",
                            &modifiers[0].token,
                        );
                    }
                    Ok(Statement::EmptyStatement {
                        token: Box::new(token),
                    })
                }
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!("Unexpected token '{}'", token.value),
                        &token,
                    );
                    Err(())
                }
            },
            _ => {
                if !modifiers.is_empty() {
                    self.emit_error(
                        TrussDiagnosticCode::ModifierNotAllowedHere,
                        format!("Modifiers are not allowed on '{}'.", token.value),
                        &modifiers[0].token,
                    );
                }
                Ok(Statement::ExpressionStatement {
                    expression: Rc::new(RefCell::new(self.parse_expression()?)),
                })
            }
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ()> {
        let left = self.parse_binary(Precedence::Assignment)?;
        if let Some(token) = self.peek()
            && let TokenType::Operator { operator } = token.ty
            && let Some(operator) = AssignmentOperator::from_operator(operator)
        {
            self.index += 1;
            let right = self.parse_expression()?;
            Ok(Expression::Assignment {
                left: Rc::new(RefCell::new(left)),
                operator,
                right: Rc::new(RefCell::new(right)),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_binary(&mut self, precedence: Precedence) -> Result<Expression, ()> {
        let mut left = self.parse_unary()?;
        if !self.is_empty() {
            let Some(token) = self.peek() else {
                return Err(());
            };
            let mut token = token;
            while !self.is_empty()
                && let Some(prec) = Precedence::get_precedence(&token)
                && prec > precedence
            {
                self.index += 1;
                if let TokenType::Operator { operator } = token.ty {
                    let right = self.parse_binary(prec)?;
                    let Some(op) = BinaryOperator::from_operator(operator) else {
                        self.emit_error(
                            TrussDiagnosticCode::InvalidOperator,
                            format!("Invalid binary operator '{}'", token.value),
                            &token,
                        );
                        return Err(());
                    };
                    left = Expression::Binary {
                        left: Rc::new(RefCell::new(left)),
                        operator: op,
                        right: Rc::new(RefCell::new(right)),
                    }
                } else if let TokenType::Keyword { keyword } = token.ty
                    && keyword == KeywordType::As
                {
                    let kind = if let Some(next) = self.peek() {
                        if OperatorType::is_operator(&next, OperatorType::QuestionMark) {
                            self.index += 1;
                            CastKind::Conditional
                        } else if OperatorType::is_operator(&next, OperatorType::Not) {
                            self.index += 1;
                            if let Some(next2) = self.peek() {
                                if OperatorType::is_operator(&next2, OperatorType::Not) {
                                    self.index += 1;
                                    CastKind::ForceBitcast
                                } else {
                                    CastKind::Force
                                }
                            } else {
                                CastKind::Force
                            }
                        } else {
                            CastKind::Regular
                        }
                    } else {
                        CastKind::Regular
                    };
                    let kind_tokens = match kind {
                        CastKind::Conditional => Some((
                            Box::new(self.tokens[self.index - 1].clone()),
                            Box::new(self.tokens[self.index - 1].clone()),
                        )),
                        CastKind::Force => Some((
                            Box::new(self.tokens[self.index - 1].clone()),
                            Box::new(self.tokens[self.index - 1].clone()),
                        )),
                        CastKind::ForceBitcast => {
                            let first_not = self.tokens[self.index - 2].clone();
                            let second_not = self.tokens[self.index - 1].clone();
                            Some((Box::new(first_not), Box::new(second_not)))
                        }
                        CastKind::Regular => None,
                    };
                    let target_type = self.parse_type_expression()?;
                    left = Expression::Cast {
                        expression: Rc::new(RefCell::new(left)),
                        target_type: Rc::new(RefCell::new(target_type)),
                        token: Box::new(token),
                        kind_tokens,
                        kind,
                        ty: None,
                    }
                } else if let TokenType::Separator { .. } = token.ty {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!("Expected operator, found separator '{}'", token.value),
                        &token,
                    );
                    return Err(());
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!("Not an operator token '{}'", token.value),
                        &token,
                    );
                    return Err(());
                }
                let Some(t) = self.peek() else { break };
                token = t;
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, ()> {
        if let Some(token) = self.peek()
            && let TokenType::Operator { operator } = token.ty
            && let OperatorType::Plus
            | OperatorType::Minus
            | OperatorType::Inc
            | OperatorType::Dec
            | OperatorType::BitNot
            | OperatorType::Multiply = operator
        {
            self.index += 1;
            let expression = self.parse_unary()?;
            let Some(op) = UnaryOperator::from_operator(operator) else {
                self.emit_error(
                    TrussDiagnosticCode::InvalidOperator,
                    format!("Invalid unary operator '{}'", token.value),
                    &token,
                );
                return Err(());
            };
            Ok(Expression::Unary {
                expression: Rc::new(RefCell::new(expression)),
                operator: op,
                is_prefix: true,
            })
        } else {
            let expression = self.parse_primary()?;
            if let Some(token) = self.peek()
                && let TokenType::Operator { operator } = token.ty
            {
                match operator {
                    OperatorType::Inc | OperatorType::Dec => {
                        self.index += 1;
                        let Some(op) = UnaryOperator::from_operator(operator) else {
                            self.emit_error(
                                TrussDiagnosticCode::InvalidOperator,
                                format!("Invalid unary operator '{}'", token.value),
                                &token,
                            );
                            return Err(());
                        };
                        Ok(Expression::Unary {
                            expression: Rc::new(RefCell::new(expression)),
                            operator: op,
                            is_prefix: false,
                        })
                    }
                    OperatorType::Not => {
                        if let Some(token2) = self.peek2()
                            && let TokenType::Operator { operator } = token2.ty
                            && let OperatorType::Not = operator
                        {
                            self.index += 2;
                            Ok(Expression::Unary {
                                expression: Rc::new(RefCell::new(expression)),
                                operator: UnaryOperator::NotNullAssertation,
                                is_prefix: false,
                            })
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UnexpectedToken,
                                "Expected '!!' for not-null assertion",
                                &token,
                            );
                            Err(())
                        }
                    }
                    _ => Ok(expression),
                }
            } else {
                Ok(expression)
            }
        }
    }

    fn parse_primary(&mut self) -> Result<Expression, ()> {
        let Some(token) = self.peek() else {
            let last_token = &self.tokens[self.index.saturating_sub(1)];
            self.emit_error(
                TrussDiagnosticCode::ExpectedExpression,
                "Expected expression but reached end of input",
                last_token,
            );
            return Err(());
        };
        let mut expression = match token.ty {
            TokenType::IntegerLiteral { value } => {
                self.index += 1;
                Ok(Expression::IntegerLiteral {
                    token: Box::new(token),
                    value,
                    ty: None,
                })
            }
            TokenType::DecimalLiteral { value } => {
                self.index += 1;
                Ok(Expression::DecimalLiteral {
                    token: Box::new(token),
                    value,
                    ty: None,
                })
            }
            TokenType::BooleanLiteral { .. } => {
                self.index += 1;
                Ok(Expression::BooleanLiteral {
                    token: Box::new(token),
                })
            }
            TokenType::NullLiteral => {
                self.index += 1;
                Ok(Expression::NullLiteral {
                    token: Box::new(token),
                })
            }
            TokenType::NullptrLiteral => {
                self.index += 1;
                Ok(Expression::NullptrLiteral {
                    token: Box::new(token),
                    ty: None,
                })
            }
            TokenType::CharLiteral { .. } => {
                self.index += 1;
                Ok(Expression::CharLiteral {
                    token: Box::new(token),
                })
            }
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::If => self.parse_if(),
                KeywordType::Case => self.parse_case_expression(),
                KeywordType::SelfKw => {
                    self.index += 1;
                    Ok(Expression::SelfKeyword {
                        token: Box::new(token),
                        ty: None,
                        symbol: None,
                    })
                }
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!("Unexpected keyword '{}'", token.value),
                        &token,
                    );
                    Err(())
                }
            },
            TokenType::Separator { separator } => match separator {
                SeparatorType::OpenBrace => self.parse_block(),
                SeparatorType::OpenParen => {
                    self.index += 1;
                    let left = token;

                    if let Some(t) = self.peek()
                        && SeparatorType::is_separator(&t, SeparatorType::CloseParen)
                    {
                        let right = self.next().unwrap();
                        Ok(Expression::VoidLiteral {
                            left: Box::new(left),
                            right: Box::new(right),
                        })
                    } else {
                        let (first_name, first_expr) = self.parse_maybe_named_expr()?;
                        let first = Rc::new(RefCell::new(first_expr));
                        let has_comma = self.peek().map_or(false, |t| {
                            SeparatorType::is_separator(&t, SeparatorType::Comma)
                        });

                        if first_name.is_some() || has_comma {
                            let mut elements = vec![(first_name, first)];

                            if has_comma {
                                self.index += 1;
                                loop {
                                    let (name, expr) = self.parse_maybe_named_expr()?;
                                    elements.push((name, Rc::new(RefCell::new(expr))));

                                    if let Some(t) = self.peek()
                                        && SeparatorType::is_separator(&t, SeparatorType::Comma)
                                    {
                                        self.index += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }

                            let Some(right) = self.next() else {
                                self.emit_error(
                                    TrussDiagnosticCode::UnexpectedToken,
                                    "Expected closing parenthesis",
                                    &left,
                                );
                                return Err(());
                            };
                            if !SeparatorType::is_separator(
                                &right,
                                SeparatorType::CloseParen,
                            ) {
                                self.emit_error(
                                    TrussDiagnosticCode::UnexpectedToken,
                                    format!("Expected ')' but found '{}'", right.value),
                                    &right,
                                );
                                return Err(());
                            }

                            Ok(Expression::TupleLiteral {
                                left: Box::new(left),
                                elements,
                                right: Box::new(right),
                                ty: None,
                            })
                        } else {
                            let Some(right) = self.next() else {
                                self.emit_error(
                                    TrussDiagnosticCode::UnexpectedToken,
                                    "Expected closing parenthesis",
                                    &left,
                                );
                                return Err(());
                            };
                            if !SeparatorType::is_separator(
                                &right,
                                SeparatorType::CloseParen,
                            ) {
                                self.emit_error(
                                    TrussDiagnosticCode::UnexpectedToken,
                                    format!("Expected ')' but found '{}'", right.value),
                                    &right,
                                );
                                return Err(());
                            }

                            Ok(Rc::try_unwrap(first).ok().unwrap().into_inner())
                        }
                    }
                }
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedExpression,
                        format!("Unexpected separator '{}'", token.value),
                        &token,
                    );
                    Err(())
                }
            },
            TokenType::Identifier => {
                self.index += 1;
                Ok(Expression::Variable {
                    name: Box::new(token),
                    ty: None,
                    symbol: None,
                })
            }
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedExpression,
                    format!("Unexpected token '{}'", token.value),
                    &token,
                );
                Err(())
            }
        }?;
        while !self.is_empty() {
            let Some(token) = self.peek() else { break };
            match token.ty {
                TokenType::Separator { separator } => match separator {
                    SeparatorType::OpenParen => expression = self.parse_call(expression)?,
                    _ => break,
                },
                TokenType::Operator { operator } => match operator {
                    OperatorType::Dot => {
                        self.index += 1;
                        let Some(member_token) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::ExpectedExpression,
                                "Expected member name after '.'",
                                &token,
                            );
                            return Err(());
                        };
                        if let TokenType::IntegerLiteral { value } = member_token.ty {
                            if value < 0 {
                                self.emit_error(
                                    TrussDiagnosticCode::ExpectedExpression,
                                    "Index cannot be negative",
                                    &member_token,
                                );
                                return Err(());
                            }
                            expression = Expression::TupleIndexAccess {
                                object: Rc::new(RefCell::new(expression)),
                                index: Box::new(member_token),
                                index_value: value as u64,
                                ty: None,
                            };
                        } else if let TokenType::DecimalLiteral { value } = member_token.ty {
                            if value.fract() != 0.0 || value < 0.0 {
                                self.emit_error(
                                    TrussDiagnosticCode::ExpectedExpression,
                                    "Tuple index must be an integer",
                                    &member_token,
                                );
                                return Err(());
                            }
                            expression = Expression::TupleIndexAccess {
                                object: Rc::new(RefCell::new(expression)),
                                index: Box::new(member_token),
                                index_value: value as u64,
                                ty: None,
                            };
                        } else if TokenType::Identifier == member_token.ty {
                            expression = Expression::MemberAccess {
                                object: Rc::new(RefCell::new(expression)),
                                member: Box::new(member_token),
                                ty: None,
                            };
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::ExpectedIdentifier,
                                format!("Expected member name or index but found '{}'", member_token.value),
                                &member_token,
                            );
                            return Err(());
                        }
                    }
                    OperatorType::Less
                        if matches!(
                            expression,
                            Expression::Variable { .. } | Expression::Type { .. }
                        ) =>
                    {
                        let mut temp_idx = self.index + 1;
                        let mut angle_count = 1;
                        while temp_idx < self.tokens.len() && angle_count > 0 {
                            if let TokenType::Operator { .. } = self.tokens[temp_idx].ty {
                                if OperatorType::is_operator(
                                    &self.tokens[temp_idx],
                                    OperatorType::Less,
                                ) {
                                    angle_count += 1;
                                } else if OperatorType::is_operator(
                                    &self.tokens[temp_idx],
                                    OperatorType::Greater,
                                ) {
                                    angle_count -= 1;
                                }
                            }
                            temp_idx += 1;
                        }
                        if angle_count == 0
                            && temp_idx < self.tokens.len()
                            && SeparatorType::is_separator(
                                &self.tokens[temp_idx],
                                SeparatorType::OpenParen,
                            )
                        {
                            expression = self.parse_call(expression)?;
                        } else {
                            break;
                        }
                    }
                    _ => {
                        break;
                    }
                },
                _ => break,
            }
        }
        Ok(expression)
    }

    fn parse_type_expression(&mut self) -> Result<Expression, ()> {
        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::Any)
        {
            self.index += 1;
            let inner = self.parse_type_expression()?;

            return Ok(Expression::AnyType {
                inner: Rc::new(RefCell::new(inner)),
                ty: None,
            });
        }

        let Some(token) = self.peek() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedType,
                "Expected type name",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };

        if SeparatorType::is_separator(&token, SeparatorType::OpenParen) {
            self.index += 1;
            let left = token;

            if let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::CloseParen)
            {
                let right = self.next().unwrap();
                let void_token = Token::new(
                    "Void".to_string(),
                    TokenType::Identifier,
                    right.position,
                    self.file.clone(),
                );
                return Ok(Expression::Type {
                    name: Box::new(void_token),
                    type_parameters: None,
                    ty: None,
                });
            }

            let (first_name, first_type) = self.parse_maybe_named_type()?;
            let first = Rc::new(RefCell::new(first_type));
            let has_comma = self.peek().map_or(false, |t| {
                SeparatorType::is_separator(&t, SeparatorType::Comma)
            });

            if first_name.is_some() || has_comma {
                let mut elements = vec![(first_name, first)];

                if has_comma {
                    self.index += 1;
                    loop {
                        let (name, type_expr) = self.parse_maybe_named_type()?;
                        elements.push((name, Rc::new(RefCell::new(type_expr))));

                        if let Some(t) = self.peek()
                            && SeparatorType::is_separator(&t, SeparatorType::Comma)
                        {
                            self.index += 1;
                        } else {
                            break;
                        }
                    }
                }

                let Some(right) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Expected closing parenthesis",
                        &left,
                    );
                    return Err(());
                };
                if !SeparatorType::is_separator(&right, SeparatorType::CloseParen) {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!("Expected ')' but found '{}'", right.value),
                        &right,
                    );
                    return Err(());
                }

                let mut tuple_type_expr: Expression = Expression::TupleType {
                    left: Box::new(left),
                    elements,
                    right: Box::new(right),
                };

                while let Some(token) = self.peek()
                    && OperatorType::is_operator(&token, OperatorType::Multiply)
                {
                    self.index += 1;
                    tuple_type_expr = Expression::PointerType {
                        base: Box::new(Rc::new(RefCell::new(tuple_type_expr))),
                        ty: None,
                    };
                }

                return Ok(tuple_type_expr);
            }

            let Some(right) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    "Expected closing parenthesis",
                    &left,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&right, SeparatorType::CloseParen) {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!("Expected ')' but found '{}'", right.value),
                    &right,
                );
                return Err(());
            }

            let mut type_expr = Rc::try_unwrap(first).ok().unwrap().into_inner();

            while let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::Multiply)
            {
                self.index += 1;
                type_expr = Expression::PointerType {
                    base: Box::new(Rc::new(RefCell::new(type_expr))),
                    ty: None,
                };
            }

            let mut types = vec![Rc::new(RefCell::new(type_expr))];
            while let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::BitAnd)
            {
                self.index += 1;
                let right = self.parse_type_expression()?;
                if let Expression::CompoundType { types: inner_types, .. } = right {
                    types.extend(inner_types);
                } else {
                    types.push(Rc::new(RefCell::new(right)));
                }
            }

            if types.len() > 1 {
                return Ok(Expression::CompoundType { types, ty: None });
            } else {
                return Ok(Rc::try_unwrap(types.into_iter().next().unwrap()).ok().unwrap().into_inner());
            }
        }

        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedType,
                "Expected type name",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if TokenType::Identifier != name.ty {
            self.emit_error(
                TrussDiagnosticCode::ExpectedType,
                format!("Expected type name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let type_parameters = self.parse_type_parameters()?;
        let mut type_expr = Expression::Type {
            name: Box::new(name),
            type_parameters,
            ty: None,
        };

        while let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Multiply)
        {
            self.index += 1;
            type_expr = Expression::PointerType {
                base: Box::new(Rc::new(RefCell::new(type_expr))),
                ty: None,
            };
        }

        let mut types = vec![Rc::new(RefCell::new(type_expr))];
        while let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::BitAnd)
        {
            self.index += 1;
            let right = self.parse_type_expression()?;
            if let Expression::CompoundType { types: inner_types, .. } = right {
                types.extend(inner_types);
            } else {
                types.push(Rc::new(RefCell::new(right)));
            }
        }

        if types.len() > 1 {
            Ok(Expression::CompoundType { types, ty: None })
        } else {
            Ok(Rc::try_unwrap(types.into_iter().next().unwrap()).ok().unwrap().into_inner())
        }
    }

    fn parse_function_decl(
        &mut self,
        is_extern: bool,
        modifiers: Vec<Modifier>,
    ) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let is_init = KeywordType::is_keyword(&token, KeywordType::Init);
        let name = if !is_init {
            let Some(name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::InvalidFunctionName,
                    "Expected function name after 'func'",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if TokenType::Identifier != name.ty {
                self.emit_error(
                    TrussDiagnosticCode::InvalidFunctionName,
                    format!("Expected function name but found '{}'", name.value),
                    &name,
                );
                return Err(());
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '(' after function name",
                    &name,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&next, SeparatorType::OpenParen) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '(' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
            Some(name)
        } else {
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '(' after 'init'",
                    &token,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&next, SeparatorType::OpenParen) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '(' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
            None
        };
        let mut parameters = Vec::new();
        let mut has_variadic = false;
        while let Some(t) = self.peek() {
            if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                break;
            }
            if let TokenType::Operator { .. } = t.ty
                && OperatorType::is_operator(&t, OperatorType::OpenRange)
            {
                self.index += 1;
                let variadic_token = Token::new(
                    "...".to_string(),
                    TokenType::Identifier,
                    t.position,
                    self.file.clone(),
                );
                parameters.push(Rc::new(RefCell::new(Parameter {
                    label: None,
                    name: Box::new(variadic_token),
                    type_expression: Rc::new(RefCell::new(Expression::Type {
                        name: Box::new(Token::new(
                            "Void".to_string(),
                            TokenType::Identifier,
                            t.position,
                            self.file.clone(),
                        )),
                        type_parameters: None,
                        ty: None,
                    })),
                    ty: None,
                    variadic_kind: VariadicKind::BareVariadic,
                })));
                if has_variadic {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Variadic parameter must be the last parameter and only one is allowed",
                        &t,
                    );
                    let Some(comma_or_close) = self.peek() else {
                        break;
                    };
                    if SeparatorType::is_separator(&comma_or_close, SeparatorType::Comma) {
                        self.index += 1;
                    }
                } else {
                    has_variadic = true;
                    let Some(comma_or_close) = self.peek() else {
                        break;
                    };
                    if SeparatorType::is_separator(&comma_or_close, SeparatorType::Comma) {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            "Variadic parameter must be the last parameter and only one is allowed",
                            &t,
                        );
                        self.index += 1;
                    }
                }
                continue;
            }
            let Some(first) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    "Expected parameter name",
                    &t,
                );
                return Err(());
            };
            if TokenType::Identifier != first.ty {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    format!("Expected parameter name but found '{}'", first.value),
                    &first,
                );
                return Err(());
            }

            let (label_token, name_token) = if let Some(peeked) = self.peek()
                && SeparatorType::is_separator(&peeked, SeparatorType::Colon)
            {
                (None, first)
            } else {
                let Some(second) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        "Expected parameter name after label",
                        &first,
                    );
                    return Err(());
                };
                if TokenType::Identifier != second.ty {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        format!("Expected parameter name but found '{}'", second.value),
                        &second,
                    );
                    return Err(());
                }
                (Some(first), second)
            };

            let Some(colon) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected ':' after parameter name",
                    &name_token,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&colon, SeparatorType::Colon) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected ':' but found '{}'", colon.value),
                    &colon,
                );
                return Err(());
            }
            let type_expression = self.parse_type_expression()?;
            let variadic_kind = if let Some(peeked) = self.peek()
                && OperatorType::is_operator(&peeked, OperatorType::OpenRange)
            {
                self.index += 1;
                if has_variadic {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Variadic parameter must be the last parameter and only one is allowed",
                        &peeked,
                    );
                } else {
                    has_variadic = true;
                    let Some(comma_or_close) = self.peek() else {
                        break;
                    };
                    if SeparatorType::is_separator(&comma_or_close, SeparatorType::Comma) {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            "Variadic parameter must be the last parameter and only one is allowed",
                            &peeked,
                        );
                    }
                }
                VariadicKind::TypedVariadic
            } else {
                VariadicKind::NotVariadic
            };
            parameters.push(Rc::new(RefCell::new(Parameter {
                label: label_token.map(Box::new),
                name: Box::new(name_token),
                type_expression: Rc::new(RefCell::new(type_expression)),
                ty: None,
                variadic_kind,
            })));
            let Some(t) = self.peek() else { break };
            if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                self.index += 1;
            } else {
                break;
            }
        }
        let Some(next) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected ')' to close parameter list",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !SeparatorType::is_separator(&next, SeparatorType::CloseParen) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected ')' but found '{}'", next.value),
                &next,
            );
            return Err(());
        }
        let return_type = if is_init {
            None
        } else if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Arrow)
        {
            self.index += 1;
            Some(self.parse_type_expression()?)
        } else {
            let current_token = self
                .peek()
                .unwrap_or_else(|| self.tokens[self.index.saturating_sub(1)].clone());
            let void_token = Token::new(
                "Void".to_string(),
                TokenType::Identifier,
                Position {
                    len: 1,
                    ..current_token.position
                },
                self.file.clone(),
            );
            Some(Expression::Type {
                name: Box::new(void_token),
                type_parameters: None,
                ty: None,
            })
        };

        let body = if is_extern {
            FunctionBody::None
        } else {
            if let Some(token) = self.peek()
                && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
            {
                self.index += 1;
                let mut statements = Vec::new();
                while let Some(t) = self.peek() {
                    if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                        break;
                    }
                    if let Ok(stmt) = self.parse_statement() {
                        statements.push(Rc::new(RefCell::new(stmt)));
                    } else {
                        self.skip();
                    }
                }
                let Some(next) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected '}' to close function body",
                        &self.tokens[self.index.saturating_sub(1)],
                    );
                    return Err(());
                };
                if !SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        format!("Expected '}}' but found '{}'", next.value),
                        &next,
                    );
                    return Err(());
                }
                FunctionBody::Statements(statements)
            } else if let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::Assign)
            {
                self.index += 1;
                let expression = self.parse_expression()?;
                FunctionBody::Expression(Rc::new(RefCell::new(expression)))
            } else {
                FunctionBody::None
            }
        };

        if is_init {
            Ok(Statement::InitDecl {
                modifiers,
                token: Box::new(token),
                parameters,
                body: Rc::new(RefCell::new(body)),
                scope: None,
                ty: None,
            })
        } else {
            Ok(Statement::FunctionDecl {
                modifiers,
                token: Box::new(token),
                name: Box::new(name.unwrap()),
                generic_parameters: vec![],
                parameters,
                return_type: return_type.map(RefCell::new).map(Rc::new),
                body: Rc::new(RefCell::new(body)),
                scope: None,
                ty: None,
            })
        }
    }

    fn parse_deinit_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };

        let body = if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            self.index += 1;
            let mut statements = Vec::new();
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                if let Ok(stmt) = self.parse_statement() {
                    statements.push(Rc::new(RefCell::new(stmt)));
                } else {
                    self.skip();
                }
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close deinit body",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
            FunctionBody::Statements(statements)
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' to open deinit body",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };

        Ok(Statement::DeinitDecl {
            modifiers,
            token: Box::new(token),
            body: Rc::new(RefCell::new(body)),
            scope: None,
            ty: None,
        })
    }

    fn parse_return(&mut self) -> Result<Statement, ()> {
        let Some(token) = self.peek() else {
            return Err(());
        };
        let current_line = token.position.line;
        self.index += 1;
        let return_token = token;
        let value = if let Some(token) = self.peek()
            && current_line == token.position.line
            && !SeparatorType::is_separator(&token, SeparatorType::CloseBrace)
            && !SeparatorType::is_separator(&token, SeparatorType::SemiColon)
        {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::Return {
            token: Box::new(return_token),
            value: value.map(RefCell::new).map(Rc::new),
        })
    }

    fn parse_loop(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::Loop {
                token: Box::new(token),
                body: Rc::new(RefCell::new(body)),
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after 'loop'",
                &self.tokens[self.index],
            );
            Err(())
        }
    }

    fn parse_while(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let condition = self.parse_expression()?;
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::While {
                token: Box::new(token),
                condition: Rc::new(RefCell::new(condition)),
                body: Rc::new(RefCell::new(body)),
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after while condition",
                &self.tokens[self.index],
            );
            Err(())
        }
    }

    fn parse_repeat_while(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        if let Some(token) = self.peek()
            && !SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after 'repeat'",
                &token,
            );
            return Err(());
        }
        let body = self.parse_block()?;
        if let Some(t) = self.peek()
            && KeywordType::is_keyword(&t, KeywordType::While)
        {
            self.index += 1;
            let condition = self.parse_expression()?;
            Ok(Statement::RepeatWhile {
                token: Box::new(token),
                body: Rc::new(RefCell::new(body)),
                condition: Rc::new(RefCell::new(condition)),
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                "Expected 'while' after repeat body",
                &self.tokens[self.index],
            );
            Err(())
        }
    }

    fn parse_for(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let pattern = self.parse_pattern()?;
        let Some(in_keyword) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                "Expected 'in' after for pattern",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !KeywordType::is_keyword(&in_keyword, KeywordType::In) {
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                format!("Expected 'in' but found '{}'", in_keyword.value),
                &in_keyword,
            );
            return Err(());
        }
        let iterator = self.parse_expression()?;
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::For {
                token: Box::new(token),
                pattern: Rc::new(pattern),
                iterator: Rc::new(RefCell::new(iterator)),
                body: Rc::new(RefCell::new(body)),
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after for iterator",
                &self.tokens[self.index],
            );
            Err(())
        }
    }

    fn parse_throw(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let exception = self.parse_expression()?;
        Ok(Statement::Throw {
            token: Box::new(token),
            exception: Rc::new(RefCell::new(exception)),
        })
    }

    fn parse_extern(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let Some(extern_token) = self.next() else {
            return Err(());
        };
        let Some(linkage_token) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                "Expected linkage specification after 'extern'",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if let TokenType::StringLiteral { .. } = linkage_token.ty {
        } else {
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                format!(
                    "Expected string literal for linkage, found '{}'",
                    linkage_token.value
                ),
                &linkage_token,
            );
            return Err(());
        }
        if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            self.index += 1;
            let mut items = Vec::new();
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                if let Ok(stmt) = self.parse_extern_item(modifiers.clone()) {
                    items.push(Rc::new(RefCell::new(stmt)));
                } else {
                    self.skip();
                }
            }
            let Some(close_brace) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close extern block",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close_brace, SeparatorType::CloseBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", close_brace.value),
                    &close_brace,
                );
                return Err(());
            }
            Ok(Statement::ExternBlock {
                token: Box::new(extern_token),
                linkage: Box::new(linkage_token),
                items,
            })
        } else {
            let statement = self.parse_extern_item(modifiers)?;
            Ok(Statement::ExternDecl {
                token: Box::new(extern_token),
                linkage: Box::new(linkage_token),
                statement: Rc::new(RefCell::new(statement)),
            })
        }
    }

    fn parse_extern_item(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let Some(token) = self.peek() else {
            return Err(());
        };
        match token.ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(true, modifiers),
                KeywordType::Let | KeywordType::Var => self.parse_variable_decl(true, modifiers),
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!(
                            "Expected 'func', 'let', or 'var' in extern block, found '{}'",
                            token.value
                        ),
                        &token,
                    );
                    Err(())
                }
            },
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!(
                        "Expected declaration in extern block, found '{}'",
                        token.value
                    ),
                    &token,
                );
                Err(())
            }
        }
    }

    fn parse_variable_decl(
        &mut self,
        is_extern: bool,
        modifiers: Vec<Modifier>,
    ) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::InvalidVariableName,
                "Expected variable name after 'let' or 'var'",
                &token,
            );
            return Err(());
        };
        if TokenType::Identifier != name.ty {
            self.emit_error(
                TrussDiagnosticCode::InvalidVariableName,
                format!("Expected variable name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let type_expression = if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::Colon)
        {
            self.index += 1;
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        let initializer = if !is_extern
            && let Some(t) = self.peek()
            && OperatorType::is_operator(&t, OperatorType::Assign)
        {
            self.index += 1;
            Some(self.parse_expression()?)
        } else {
            None
        };
        let accessors = if !is_extern
            && let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            self.index += 1;
            self.parse_accessor_body()?
        } else {
            Vec::new()
        };
        Ok(Statement::VariableDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            type_expression: type_expression.map(RefCell::new).map(Rc::new),
            initializer: initializer.map(RefCell::new).map(Rc::new),
            accessors,
            ty: None,
        })
    }

    fn parse_accessor_body(&mut self) -> Result<Vec<Accessor>, ()> {
        let Some(first) = self.peek() else {
            let last = &self.tokens[self.index.saturating_sub(1)];
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                "Expected accessor or getter body after '{'".to_string(),
                last,
            );
            return Err(());
        };
        let is_accessor_block = if let TokenType::Identifier = first.ty {
            matches!(first.value.as_str(), "get" | "set" | "willSet" | "didSet")
                && self.peek2().is_some()
                && (SeparatorType::is_separator(&self.peek2().unwrap(), SeparatorType::OpenBrace)
                    || SeparatorType::is_separator(
                        &self.peek2().unwrap(),
                        SeparatorType::OpenParen,
                    ))
        } else {
            false
        };
        if is_accessor_block {
            self.parse_accessors()
        } else {
            let mut body = Vec::new();
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                if let Ok(stmt) = self.parse_statement() {
                    body.push(Rc::new(RefCell::new(stmt)));
                } else {
                    self.skip();
                }
            }
            let Some(close) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close getter body".to_string(),
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }
            Ok(vec![Accessor {
                kind: AccessorKind::Get,
                parameter: None,
                body,
            }])
        }
    }

    fn parse_accessors(&mut self) -> Result<Vec<Accessor>, ()> {
        let mut accessors = Vec::new();
        loop {
            let Some(token) = self.peek() else {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    "Expected accessor or '}'".to_string(),
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if SeparatorType::is_separator(&token, SeparatorType::CloseBrace) {
                break;
            }
            if let TokenType::Identifier = token.ty {
            } else {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!("Expected accessor name but found '{}'", token.value),
                    &token,
                );
                return Err(());
            }
            let kind = match token.value.as_str() {
                "get" => AccessorKind::Get,
                "set" => AccessorKind::Set,
                "willSet" => AccessorKind::WillSet,
                "didSet" => AccessorKind::DidSet,
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!(
                            "Expected 'get', 'set', 'willSet', or 'didSet' but found '{}'",
                            token.value
                        ),
                        &token,
                    );
                    return Err(());
                }
            };
            self.index += 1;
            let parameter = if let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::OpenParen)
            {
                self.index += 1;
                let Some(param) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        "Expected parameter name".to_string(),
                        &self.tokens[self.index.saturating_sub(1)],
                    );
                    return Err(());
                };
                if TokenType::Identifier != param.ty {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        format!("Expected parameter name but found '{}'", param.value),
                        &param,
                    );
                    return Err(());
                }
                let Some(close) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected ')' after parameter name".to_string(),
                        &self.tokens[self.index.saturating_sub(1)],
                    );
                    return Err(());
                };
                if !SeparatorType::is_separator(&close, SeparatorType::CloseParen) {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        format!("Expected ')' but found '{}'", close.value),
                        &close,
                    );
                    return Err(());
                }
                Some(Box::new(param))
            } else {
                None
            };
            let Some(t) = self.peek() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '{' to open accessor body".to_string(),
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&t, SeparatorType::OpenBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '{{' but found '{}'", t.value),
                    &t,
                );
                return Err(());
            }
            self.index += 1;
            let mut body = Vec::new();
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                if let Ok(stmt) = self.parse_statement() {
                    body.push(Rc::new(RefCell::new(stmt)));
                } else {
                    self.skip();
                }
            }
            let Some(close) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close accessor body".to_string(),
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }
            accessors.push(Accessor {
                kind,
                parameter,
                body,
            });
        }
        let has_computed = accessors.iter().any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
        let has_willset_didset = accessors.iter().any(|a| matches!(a.kind, AccessorKind::WillSet | AccessorKind::DidSet));
        if has_computed && has_willset_didset {
            let conflict_token = &self.tokens[self.index.saturating_sub(1)];
            self.emit_error(
                TrussDiagnosticCode::IncompatibleAccessors,
                "A property cannot have both willSet/didSet and get/set — willSet/didSet are for stored properties, get/set are for computed properties",
                conflict_token,
            );
        }
        let Some(close) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '}' to close accessor block".to_string(),
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !SeparatorType::is_separator(&close, SeparatorType::CloseBrace) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '}}' but found '{}'", close.value),
                &close,
            );
            return Err(());
        }
        Ok(accessors)
    }

    fn ensure_memberwise_init(&self, body: &mut Vec<Rc<RefCell<Statement>>>, type_name: &Token) {
        let has_init = body.iter().any(|stmt| matches!(&*stmt.borrow(), Statement::InitDecl { .. }));
        if has_init {
            return;
        }
        let mut parameters = Vec::new();
        for stmt in body.iter() {
            if let Statement::VariableDecl {
                name,
                type_expression: Some(type_expr),
                ..
            } = &*stmt.borrow()
            {
                let param = Rc::new(RefCell::new(Parameter {
                    label: None,
                    name: Box::new(name.as_ref().clone()),
                    type_expression: type_expr.clone(),
                    ty: None,
                    variadic_kind: VariadicKind::NotVariadic,
                }));
                parameters.push(param);
            }
        }
        let init_token = Box::new(Token::new(
            "init".to_string(),
            TokenType::Keyword { keyword: KeywordType::Init },
            type_name.position.clone(),
            type_name.file.clone(),
        ));
        let init_decl = Statement::InitDecl {
            modifiers: vec![],
            token: init_token,
            parameters,
            body: Rc::new(RefCell::new(FunctionBody::None)),
            scope: None,
            ty: None,
        };
        body.push(Rc::new(RefCell::new(init_decl)));
    }

    fn parse_struct_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                "Expected struct name after 'struct'",
                &token,
            );
            return Err(());
        };
        if TokenType::Identifier != name.ty {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                format!("Expected struct name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let mut conformances = Vec::new();
        if let Some(next) = self.peek()
            && SeparatorType::is_separator(&next, SeparatorType::Colon)
        {
            self.index += 1;
            loop {
                conformances.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
                if let Some(t) = self.peek()
                    && SeparatorType::is_separator(&t, SeparatorType::Comma)
                {
                    self.index += 1;
                } else {
                    break;
                }
            }
        }
        let mut body = self.parse_brace_body()?;
        self.ensure_memberwise_init(&mut body, &name);
        Ok(Statement::StructDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            conformances,
            body,
            scope: None,
            ty: None,
        })
    }

    fn parse_class_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                "Expected class name after 'class'",
                &token,
            );
            return Err(());
        };
        if TokenType::Identifier != name.ty {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                format!("Expected class name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let mut superclass = None;
        let mut conformances = Vec::new();
        if let Some(next) = self.peek()
            && SeparatorType::is_separator(&next, SeparatorType::Colon)
        {
            self.index += 1;
            superclass = Some(Rc::new(RefCell::new(self.parse_type_expression()?)));
            while let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::Comma)
            {
                self.index += 1;
                conformances.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
            }
        }
        let body = self.parse_brace_body()?;
        Ok(Statement::ClassDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            superclass,
            conformances,
            body,
            scope: None,
            ty: None,
        })
    }

    fn parse_brace_body(&mut self) -> Result<Vec<Rc<RefCell<Statement>>>, ()> {
        let mut body = Vec::new();
        if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            self.index += 1;
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                if let Ok(stmt) = self.parse_statement() {
                    body.push(Rc::new(RefCell::new(stmt)));
                } else {
                    self.skip();
                }
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close body",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' to open body",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        }
        Ok(body)
    }

    fn parse_enum_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                "Expected enum name after 'enum'",
                &token,
            );
            return Err(());
        };
        if TokenType::Identifier != name.ty {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                format!("Expected enum name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let mut cases = Vec::new();
        let mut body = Vec::new();
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            self.index += 1;
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                if let TokenType::Keyword { keyword } = t.ty
                    && keyword == KeywordType::Case
                {
                    self.index += 1;
                    loop {
                        let Some(case_name) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::ExpectedIdentifier,
                                "Expected case name",
                                &t,
                            );
                            return Err(());
                        };
                        if TokenType::Identifier != case_name.ty {
                            self.emit_error(
                                TrussDiagnosticCode::ExpectedIdentifier,
                                format!("Expected case name but found '{}'", case_name.value),
                                &case_name,
                            );
                            return Err(());
                        }
                        let mut parameters = Vec::new();
                        if let Some(next) = self.peek()
                            && SeparatorType::is_separator(&next, SeparatorType::OpenParen)
                        {
                            self.index += 1;
                            while let Some(p) = self.peek() {
                                if SeparatorType::is_separator(&p, SeparatorType::CloseParen) {
                                    break;
                                }
                                let label = if let Some(peeked) = self.peek2()
                                    && SeparatorType::is_separator(&peeked, SeparatorType::Colon)
                                {
                                    let label = Box::new(self.next().unwrap());
                                    self.index += 1;
                                    Some(label)
                                } else {
                                    None
                                };
                                let type_expression = self.parse_type_expression()?;
                                parameters.push(EnumCaseParameter {
                                    label,
                                    type_expression: Rc::new(RefCell::new(type_expression)),
                                });
                                if let Some(comma) = self.peek()
                                    && SeparatorType::is_separator(&comma, SeparatorType::Comma)
                                {
                                    self.index += 1;
                                } else {
                                    break;
                                }
                            }
                            let Some(close_paren) = self.next() else {
                                self.emit_error(
                                    TrussDiagnosticCode::MissingSeparator,
                                    "Expected ')' to close case parameter list",
                                    &self.tokens[self.index.saturating_sub(1)],
                                );
                                return Err(());
                            };
                            if !SeparatorType::is_separator(&close_paren, SeparatorType::CloseParen) {
                                self.emit_error(
                                    TrussDiagnosticCode::MissingSeparator,
                                    format!("Expected ')' but found '{}'", close_paren.value),
                                    &close_paren,
                                );
                                return Err(());
                            }
                        }
                        cases.push(EnumCase {
                            token: Box::new(t.clone()),
                            name: Box::new(case_name),
                            parameters,
                        });
                        if let Some(comma_or_close) = self.peek() {
                            if SeparatorType::is_separator(&comma_or_close, SeparatorType::Comma) {
                                self.index += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if let Some(sep) = self.peek()
                        && SeparatorType::is_separator(&sep, SeparatorType::SemiColon)
                    {
                        self.index += 1;
                    }
                } else {
                    if let Ok(stmt) = self.parse_statement() {
                        body.push(Rc::new(RefCell::new(stmt)));
                    } else {
                        self.skip();
                    }
                }
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close enum body",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' to open enum body",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        }
        Ok(Statement::EnumDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            cases,
            body,
            scope: None,
            ty: None,
        })
    }

    fn parse_protocol_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                "Expected protocol name after 'protocol'",
                &token,
            );
            return Err(());
        };
        if TokenType::Identifier != name.ty {
            self.emit_error(
                TrussDiagnosticCode::InvalidStructName,
                format!("Expected protocol name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let generic_parameters = Vec::new();
        let mut conformances = Vec::new();
        if let Some(next) = self.peek()
            && SeparatorType::is_separator(&next, SeparatorType::Colon)
        {
            self.index += 1;
            conformances.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
            while let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::Comma)
            {
                self.index += 1;
                conformances.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
            }
        }
        let mut members = Vec::new();
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            self.index += 1;
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                let member_modifiers = self.parse_modifiers()?;
                let Some(peek_token) = self.peek() else { break };
                match peek_token.ty {
                    TokenType::Keyword { keyword } if keyword == KeywordType::Func => {
                        let func_decl = self.parse_function_decl(false, member_modifiers)?;
                        if let Statement::FunctionDecl { .. } = &func_decl {
                            members.push(ProtocolMember::Method {
                                modifiers: vec![],
                                decl: Rc::new(RefCell::new(func_decl)),
                            });
                        }
                    }
                    TokenType::Keyword { keyword }
                        if keyword == KeywordType::Let || keyword == KeywordType::Var =>
                    {
                        let _is_mutable = keyword == KeywordType::Var;
                        let prop_token = self.next().unwrap();
                        let Some(prop_name) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::InvalidVariableName,
                                "Expected property name",
                                &prop_token,
                            );
                            return Err(());
                        };
                        if TokenType::Identifier != prop_name.ty {
                            self.emit_error(
                                TrussDiagnosticCode::InvalidVariableName,
                                format!(
                                    "Expected property name but found '{}'",
                                    prop_name.value
                                ),
                                &prop_name,
                            );
                            return Err(());
                        }
                        let Some(colon) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                "Expected ':' after property name",
                                &prop_token,
                            );
                            return Err(());
                        };
                        if !SeparatorType::is_separator(&colon, SeparatorType::Colon) {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                format!("Expected ':' but found '{}'", colon.value),
                                &colon,
                            );
                            return Err(());
                        }
                        let type_expression = self.parse_type_expression()?;
                        let mut get = false;
                        let mut set = false;
                        if let Some(next) = self.peek()
                            && SeparatorType::is_separator(&next, SeparatorType::OpenBrace)
                        {
                            self.index += 1;
                            while let Some(t) = self.peek() {
                                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                                    break;
                                }
                                if let TokenType::Identifier = t.ty {
                                    match t.value.as_str() {
                                        "get" => {
                                            get = true;
                                            self.index += 1;
                                        }
                                        "set" => {
                                            set = true;
                                            self.index += 1;
                                        }
                                        _ => {
                                            self.emit_error(
                                                TrussDiagnosticCode::UnexpectedToken,
                                                format!(
                                                    "Expected 'get' or 'set' in protocol property accessor, found '{}'",
                                                    t.value
                                                ),
                                                &t,
                                            );
                                            return Err(());
                                        }
                                    }
                                } else {
                                    self.emit_error(
                                        TrussDiagnosticCode::UnexpectedToken,
                                        format!(
                                            "Expected 'get' or 'set' in protocol property accessor, found '{}'",
                                            t.value
                                        ),
                                        &t,
                                    );
                                    return Err(());
                                }
                            }
                            let Some(close) = self.next() else {
                                self.emit_error(
                                    TrussDiagnosticCode::MissingSeparator,
                                    "Expected '}' to close accessor requirements",
                                    &self.tokens[self.index.saturating_sub(1)],
                                );
                                return Err(());
                            };
                            if !SeparatorType::is_separator(&close, SeparatorType::CloseBrace) {
                                self.emit_error(
                                    TrussDiagnosticCode::MissingSeparator,
                                    format!("Expected '}}' but found '{}'", close.value),
                                    &close,
                                );
                                return Err(());
                            }
                        }
                        if !get && !set {
                            get = true;
                        }
                        members.push(ProtocolMember::Property {
                            modifiers: member_modifiers,
                            token: Box::new(prop_token),
                            name: Box::new(prop_name),
                            type_expression: Rc::new(RefCell::new(type_expression)),
                            accessors: ProtocolAccessorSet { get, set },
                        });
                    }
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            format!(
                                "Expected 'func' or 'let'/'var' in protocol body, found '{}'",
                                peek_token.value
                            ),
                            &peek_token,
                        );
                        return Err(());
                    }
                }
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close protocol body",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' to open protocol body",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        }
        Ok(Statement::ProtocolDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            generic_parameters,
            conformances,
            members,
            scope: None,
            ty: None,
        })
    }

    fn parse_block(&mut self) -> Result<Expression, ()> {
        self.index += 1;
        let mut statements = Vec::new();
        while let Some(token) = self.peek() {
            if SeparatorType::is_separator(&token, SeparatorType::CloseBrace) {
                break;
            }
            if let Ok(stmt) = self.parse_statement() {
                statements.push(Rc::new(RefCell::new(stmt)));
            } else {
                self.skip();
            }
        }
        let Some(next) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '}' to close block",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
            Ok(Expression::Block {
                statements,
                scope: None,
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '}}' but found '{}'", next.value),
                &next,
            );
            Err(())
        }
    }

    fn parse_call(&mut self, callee: Expression) -> Result<Expression, ()> {
        let type_parameters = self.parse_type_parameters()?;
        let Some(open) = self.next() else {
            return Err(());
        };
        if !SeparatorType::is_separator(&open, SeparatorType::OpenParen) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '(' but found '{}'", open.value),
                &open,
            );
            return Err(());
        }
        let mut parameters = Vec::new();
        while let Some(token) = self.peek() {
            if SeparatorType::is_separator(&token, SeparatorType::CloseParen) {
                break;
            }
            let label = if let Some(first) = self.peek()
                && let TokenType::Identifier = first.ty
                && let Some(second) = self.peek2()
                && SeparatorType::is_separator(&second, SeparatorType::Colon)
            {
                self.index += 2;
                Some(Box::new(first))
            } else {
                None
            };
            let expr = self.parse_expression()?;
            parameters.push(CallParameter {
                label,
                expression: Rc::new(RefCell::new(expr)),
            });
            let Some(t) = self.peek() else { break };
            if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                self.index += 1;
            } else {
                break;
            }
        }
        let Some(next) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected ')' to close call arguments",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if SeparatorType::is_separator(&next, SeparatorType::CloseParen) {
            Ok(Expression::Call {
                callee: Rc::new(RefCell::new(callee)),
                type_parameters,
                parameters,
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected ')' but found '{}'", next.value),
                &next,
            );
            Err(())
        }
    }

    fn parse_if(&mut self) -> Result<Expression, ()> {
        self.index += 1;
        let condition = self.parse_expression()?;
        if let Some(token) = self.peek()
            && !SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after if condition",
                &token,
            );
            return Err(());
        }
        let then = self.parse_block()?;
        let else_ = if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::Else)
        {
            self.index += 1;
            if let Some(token) = self.peek()
                && KeywordType::is_keyword(&token, KeywordType::If)
            {
                Some(self.parse_if()?)
            } else if let Some(token) = self.peek()
                && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
            {
                Some(self.parse_block()?)
            } else {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    "Expected 'if' or '{' after 'else'",
                    &self.tokens[self.index],
                );
                return Err(());
            }
        } else {
            None
        };
        Ok(Expression::If {
            condition: Rc::new(RefCell::new(condition)),
            then: Rc::new(RefCell::new(then)),
            else_: else_.map(RefCell::new).map(Rc::new),
        })
    }
    fn parse_case_expression(&mut self) -> Result<Expression, ()> {
        let case_token = self.next().unwrap();

        let Some(enum_type_token) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedType,
                "Expected enum type name after 'case'",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if enum_type_token.ty != TokenType::Identifier {
            self.emit_error(
                TrussDiagnosticCode::ExpectedType,
                format!("Expected enum type name but found '{}'", enum_type_token.value),
                &enum_type_token,
            );
            return Err(());
        }

        let Some(dot) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '.' after enum type name",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !OperatorType::is_operator(&dot, OperatorType::Dot) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '.' but found '{}'", dot.value),
                &dot,
            );
            return Err(());
        }

        let Some(case_name_token) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                "Expected case name after '.'",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if case_name_token.ty != TokenType::Identifier {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                format!("Expected case name but found '{}'", case_name_token.value),
                &case_name_token,
            );
            return Err(());
        }

        let mut bindings = Vec::new();
        if let Some(next) = self.peek()
            && SeparatorType::is_separator(&next, SeparatorType::OpenParen)
        {
            self.index += 1;
            loop {
                if let Some(next) = self.peek()
                    && SeparatorType::is_separator(&next, SeparatorType::CloseParen)
                {
                    break;
                }
                bindings.push(self.parse_pattern()?);
                if let Some(next) = self.peek()
                    && SeparatorType::is_separator(&next, SeparatorType::Comma)
                {
                    self.index += 1;
                } else {
                    break;
                }
            }
            let Some(close_paren) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected ')' to close case bindings",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close_paren, SeparatorType::CloseParen) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected ')' but found '{}'", close_paren.value),
                    &close_paren,
                );
                return Err(());
            }
        }

        let Some(equals) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '=' after case pattern",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !OperatorType::is_operator(&equals, OperatorType::Assign) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '=' but found '{}'", equals.value),
                &equals,
            );
            return Err(());
        }

        let expression = self.parse_expression()?;

        Ok(Expression::Case {
            token: Box::new(case_token),
            enum_type: Box::new(enum_type_token),
            case_name: Box::new(case_name_token),
            bindings,
            expression: Rc::new(RefCell::new(expression)),
            ty: None,
        })
    }

    fn parse_modifiers(&mut self) -> Result<Vec<Modifier>, ()> {
        let mut modifiers: Vec<Modifier> = Vec::new();
        while !self.is_empty() {
            let Some(token) = self.peek() else {
                return Err(());
            };
            let TokenType::Keyword { keyword } = token.ty else {
                break;
            };
            let ty = match keyword {
                KeywordType::Open => ModifierType::Access(AccessModifier::Open),
                KeywordType::Public => ModifierType::Access(AccessModifier::Public),
                KeywordType::Internal => ModifierType::Access(AccessModifier::Internal),
                KeywordType::Fileprivate => ModifierType::Access(AccessModifier::Fileprivate),
                KeywordType::Private => ModifierType::Access(AccessModifier::Private),
                _ => {
                    break;
                }
            };
            if modifiers
                .iter()
                .find(|m| {
                    m.ty == ty
                        || (matches!(m.ty, ModifierType::Access(_))
                            && matches!(ty, ModifierType::Access(_)))
                })
                .is_some()
            {
                self.emit_error(
                    TrussDiagnosticCode::DuplicateModifier,
                    format!("Duplicate modifier: '{}'", token.value),
                    &token,
                );
                self.index += 1;
                continue;
            }
            modifiers.push(Modifier {
                token: Box::new(token.clone()),
                ty,
            });
            self.index += 1;
        }
        Ok(modifiers)
    }

    #[allow(dead_code)]
    fn parse_statements(&mut self) -> Result<Vec<Rc<RefCell<Statement>>>, ()> {
        let mut statements = Vec::new();
        while let Some(token) = self.peek() {
            if SeparatorType::is_separator(&token, SeparatorType::CloseBrace) {
                break;
            }
            if let Ok(stmt) = self.parse_statement() {
                statements.push(Rc::new(RefCell::new(stmt)));
            } else {
                self.skip();
            }
        }
        let Some(next) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '}' to close statements",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
            Ok(statements)
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '}}' but found '{}'", next.value),
                &next,
            );
            Err(())
        }
    }

    fn parse_type_parameters(&mut self) -> Result<Option<Vec<Rc<RefCell<Expression>>>>, ()> {
        if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Less)
        {
            self.index += 1;
            let mut type_parameters = Vec::new();
            while let Some(token) = self.peek() {
                if OperatorType::is_operator(&token, OperatorType::Greater) {
                    break;
                }
                type_parameters.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
                let Some(t) = self.peek() else { break };
                if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                    self.index += 1;
                } else {
                    break;
                }
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '>' to close type parameters",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !OperatorType::is_operator(&next, OperatorType::Greater) {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '>' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
            Ok(Some(type_parameters))
        } else {
            Ok(None)
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        match token.ty {
            TokenType::Identifier => {
                if token.value == "_" {
                    Ok(Pattern::Ignore)
                } else {
                    Ok(Pattern::Identifier(Box::new(token)))
                }
            }
            TokenType::Separator { separator } => match separator {
                SeparatorType::OpenParen => {
                    let mut patterns = Vec::new();
                    while let Some(t) = self.peek() {
                        if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                            break;
                        }
                        patterns.push(self.parse_pattern()?);
                        let Some(t) = self.peek() else { break };
                        if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                            self.index += 1;
                        } else {
                            break;
                        }
                    }
                    Ok(Pattern::Tuple(patterns))
                }
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        format!("Unexpected separator in pattern '{}'", token.value),
                        &token,
                    );
                    Err(())
                }
            },
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    format!("Expected identifier in pattern but found '{}'", token.value),
                    &token,
                );
                Err(())
            }
        }
    }

    fn parse_maybe_named_expr(&mut self) -> Result<(Option<String>, Expression), ()> {
        if let Some(name_token) = self.peek()
            && let TokenType::Identifier = name_token.ty
            && let Some(colon_token) = self.peek2()
            && SeparatorType::is_separator(&colon_token, SeparatorType::Colon)
        {
            self.index += 2;
            let name = name_token.value.clone();
            let expr = self.parse_expression()?;
            return Ok((Some(name), expr));
        }
        let expr = self.parse_expression()?;
        Ok((None, expr))
    }

    fn parse_maybe_named_type(&mut self) -> Result<(Option<String>, Expression), ()> {
        if let Some(name_token) = self.peek()
            && let TokenType::Identifier = name_token.ty
            && let Some(colon_token) = self.peek2()
            && SeparatorType::is_separator(&colon_token, SeparatorType::Colon)
        {
            self.index += 2;
            let name = name_token.value.clone();
            let type_expr = self.parse_type_expression()?;
            return Ok((Some(name), type_expr));
        }
        let type_expr = self.parse_type_expression()?;
        Ok((None, type_expr))
    }

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>, token: &Token) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg));
        self.engine.borrow_mut().emit(diag);
    }
}
