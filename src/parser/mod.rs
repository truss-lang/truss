pub mod precedence;

use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        expression::{
            AssignmentOperator, BinaryOperator, CallParameter, Expression, UnaryOperator,
        },
        node::Program,
        statement::{FunctionBody, Parameter, Pattern, Statement, VariadicKind},
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
        let Some(token) = self.peek() else {
            return Err(());
        };
        match token.ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(),
                KeywordType::Let | KeywordType::Var => self.parse_variable_decl(),
                KeywordType::Return => self.parse_return(),
                KeywordType::Loop => self.parse_loop(),
                KeywordType::While => self.parse_while(),
                KeywordType::Repeat => self.parse_repeat_while(),
                KeywordType::For => self.parse_for(),
                KeywordType::Throw => self.parse_throw(),
                KeywordType::Extern => self.parse_extern(),
                _ => Ok(Statement::ExpressionStatement {
                    expression: Rc::new(RefCell::new(self.parse_expression()?)),
                }),
            },
            TokenType::Separator { separator } => match separator {
                SeparatorType::SemiColon => {
                    self.index += 1;
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
            _ => Ok(Statement::ExpressionStatement {
                expression: Rc::new(RefCell::new(self.parse_expression()?)),
            }),
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
                let right = self.parse_binary(prec)?;
                if let TokenType::Operator { operator } = token.ty {
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
                    let Some(t) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            "Expected closing parenthesis",
                            &token,
                        );
                        return Err(());
                    };
                    if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                        Ok(Expression::VoidLiteral {
                            left: Box::new(token),
                            right: Box::new(t),
                        })
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            format!("Expected ')' but found '{}'", t.value),
                            &t,
                        );
                        Err(())
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
                TokenType::Operator { .. } => {
                    if OperatorType::is_operator(&token, OperatorType::Less)
                        && matches!(
                            expression,
                            Expression::Variable { .. } | Expression::Type { .. }
                        )
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
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(expression)
    }

    fn parse_type_expression(&mut self) -> Result<Expression, ()> {
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
            let Some(t) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    "Expected closing parenthesis",
                    &token,
                );
                return Err(());
            };
            if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                let void_token = Token::new(
                    "Void".to_string(),
                    TokenType::Identifier,
                    token.position,
                    self.file.clone(),
                );
                return Ok(Expression::Type {
                    name: Box::new(void_token),
                    type_parameters: None,
                    ty: None,
                });
            } else {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!("Expected ')' but found '{}'", t.value),
                    &t,
                );
                return Err(());
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

        Ok(type_expr)
    }

    fn parse_function_decl(&mut self) -> Result<Statement, ()> {
        let Some(_token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::InvalidFunctionName,
                "Expected function name after 'func'",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
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
        let mut parameters = Vec::new();
        let mut has_variadic = false;
        while let Some(t) = self.peek() {
            if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                break;
            }
            if let TokenType::Operator { .. } = t.ty
                && OperatorType::is_operator(&t, OperatorType::OpenRange)
            {
                if has_variadic {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Variadic parameter must be the last parameter and only one is allowed",
                        &t,
                    );
                    return Err(());
                }
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
                has_variadic = true;
                let Some(comma_or_close) = self.peek() else {
                    break;
                };
                if SeparatorType::is_separator(&comma_or_close, SeparatorType::Comma) {
                    self.index += 1;
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
                if has_variadic {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Variadic parameter must be the last parameter and only one is allowed",
                        &peeked,
                    );
                    return Err(());
                }
                self.index += 1;
                has_variadic = true;
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
        let return_type = if let Some(token) = self.peek()
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
            if SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                Ok(Statement::FunctionDecl {
                    token: Box::new(token),
                    name: Box::new(name),
                    generic_parameters: vec![],
                    parameters,
                    return_type: return_type.map(RefCell::new).map(Rc::new),
                    body: Rc::new(RefCell::new(FunctionBody::Statements(statements))),
                    ty: None,
                })
            } else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", next.value),
                    &next,
                );
                Err(())
            }
        } else if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Assign)
        {
            self.index += 1;
            let expression = self.parse_expression()?;
            Ok(Statement::FunctionDecl {
                token: Box::new(token),
                name: Box::new(name),
                generic_parameters: vec![],
                parameters,
                return_type: return_type.map(RefCell::new).map(Rc::new),
                body: Rc::new(RefCell::new(FunctionBody::Expression(Rc::new(
                    RefCell::new(expression),
                )))),
                ty: None,
            })
        } else {
            Ok(Statement::FunctionDecl {
                token: Box::new(_token),
                name: Box::new(name),
                generic_parameters: vec![],
                parameters,
                return_type: return_type.map(RefCell::new).map(Rc::new),
                body: Rc::new(RefCell::new(FunctionBody::None)),
                ty: None,
            })
        }
    }

    fn parse_variable_decl(&mut self) -> Result<Statement, ()> {
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
        let type_expression = if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::Colon)
        {
            self.index += 1;
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        let initializer = if let Some(t) = self.peek()
            && OperatorType::is_operator(&t, OperatorType::Assign)
        {
            self.index += 1;
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::VariableDecl {
            token: Box::new(token),
            name: Box::new(name),
            type_expression: type_expression.map(RefCell::new).map(Rc::new),
            initializer: initializer.map(RefCell::new).map(Rc::new),
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
        self.index += 1;
        if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::Loop {
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
        self.index += 1;
        let condition = self.parse_expression()?;
        if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::While {
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
        self.index += 1;
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
        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::While)
        {
            self.index += 1;
            let condition = self.parse_expression()?;
            Ok(Statement::RepeatWhile {
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
        self.index += 1;
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
        if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::For {
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
        self.index += 1;
        let exception = self.parse_expression()?;
        Ok(Statement::Throw {
            exception: Rc::new(RefCell::new(exception)),
        })
    }

    fn parse_extern(&mut self) -> Result<Statement, ()> {
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
                if let Ok(stmt) = self.parse_extern_item() {
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
            let statement = self.parse_extern_item()?;
            Ok(Statement::ExternDecl {
                token: Box::new(extern_token),
                linkage: Box::new(linkage_token),
                statement: Rc::new(RefCell::new(statement)),
            })
        }
    }

    fn parse_extern_item(&mut self) -> Result<Statement, ()> {
        let Some(token) = self.peek() else {
            return Err(());
        };
        match token.ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl_in_extern(),
                KeywordType::Let | KeywordType::Var => self.parse_variable_decl_in_extern(),
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

    fn parse_function_decl_in_extern(&mut self) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::InvalidFunctionName,
                "Expected function name after 'func'",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
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
        let mut parameters = Vec::new();
        let mut has_variadic = false;
        while let Some(t) = self.peek() {
            if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                break;
            }
            if let TokenType::Operator { .. } = t.ty
                && OperatorType::is_operator(&t, OperatorType::OpenRange)
            {
                if has_variadic {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Variadic parameter must be the last parameter and only one is allowed",
                        &t,
                    );
                    return Err(());
                }
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
                has_variadic = true;
                let Some(comma_or_close) = self.peek() else {
                    break;
                };
                if SeparatorType::is_separator(&comma_or_close, SeparatorType::Comma) {
                    self.index += 1;
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
                if has_variadic {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Variadic parameter must be the last parameter and only one is allowed",
                        &peeked,
                    );
                    return Err(());
                }
                self.index += 1;
                has_variadic = true;
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
        let return_type = if let Some(token) = self.peek()
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
        Ok(Statement::FunctionDecl {
            token: Box::new(token),
            name: Box::new(name),
            generic_parameters: vec![],
            parameters,
            return_type: return_type.map(RefCell::new).map(Rc::new),
            body: Rc::new(RefCell::new(FunctionBody::None)),
            ty: None,
        })
    }

    fn parse_variable_decl_in_extern(&mut self) -> Result<Statement, ()> {
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
        let type_expression = if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::Colon)
        {
            self.index += 1;
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        Ok(Statement::VariableDecl {
            token: Box::new(token),
            name: Box::new(name),
            type_expression: type_expression.map(RefCell::new).map(Rc::new),
            initializer: None,
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
            Ok(Expression::Block { statements })
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

    fn emit_error(&self, code: TrussDiagnosticCode, message: impl Into<String>, token: &Token) {
        let msg = message.into();
        let diag = new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg));
        self.engine.borrow_mut().emit(diag);
    }
}
