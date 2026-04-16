pub mod precedence;

use std::{cell::RefCell, rc::Rc};

use anyhow::{Ok, Result, anyhow};

use crate::{
    ast::{
        expression::{AssignmentOperator, BinaryOperator, Expression, UnaryOperator},
        node::Program,
        statement::{Parameter, Pattern, Statement},
    },
    lexer::token::{KeywordType, OperatorType, SeparatorType, Token, TokenType},
    parser::precedence::Precedence,
};

#[derive(Debug)]
pub struct Parser {
    file: Rc<String>,
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(file: Rc<String>, tokens: Vec<Token>) -> Self {
        Self {
            file,
            tokens,
            index: 0,
        }
    }
    pub fn get_file(&mut self) -> Rc<String> {
        self.file.clone()
    }
    fn is_empty(&self) -> bool {
        self.index >= self.tokens.len()
    }
    fn peek(&self) -> Token {
        self.tokens[self.index].clone()
    }
    fn peek2(&self) -> Token {
        self.tokens[self.index + 1].clone()
    }
    fn next(&mut self) -> Token {
        let token = self.tokens[self.index].clone();
        self.index += 1;
        token
    }
    pub fn parse(&mut self) -> Result<Program> {
        let mut program = Program::new(self.file.clone());
        while !self.is_empty() {
            program
                .statements
                .push(Rc::new(RefCell::new(self.parse_statement()?)));
        }
        Ok(program)
    }
    fn parse_statement(&mut self) -> Result<Statement> {
        let token = self.peek();
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
                _ => todo!(),
            },
            _ => Ok(Statement::ExpressionStatement {
                expression: Rc::new(RefCell::new(self.parse_expression()?)),
            }),
        }
    }
    fn parse_expression(&mut self) -> Result<Expression> {
        let left = self.parse_binary(Precedence::Assignment)?;
        if !self.is_empty()
            && let TokenType::Operator { operator } = self.peek().ty
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
    fn parse_binary(&mut self, precedence: Precedence) -> Result<Expression> {
        let mut left = self.parse_unary()?;
        let mut token = self.peek();
        while !self.is_empty()
            && let Some(prec) = Precedence::get_precedence(&token)
            && prec > precedence
        {
            self.index += 1;
            let right = self.parse_binary(prec)?;
            if let TokenType::Operator { operator } = token.ty {
                left = Expression::Binary {
                    left: Rc::new(RefCell::new(left)),
                    operator: BinaryOperator::from_operator(operator).unwrap(),
                    right: Rc::new(RefCell::new(right)),
                }
            } else if let TokenType::Separator { .. } = token.ty {
                todo!();
            } else {
                return Err(anyhow!("Not an operator token"));
            }
            token = self.peek();
        }
        Ok(left)
    }
    fn parse_unary(&mut self) -> Result<Expression> {
        if let TokenType::Operator { operator } = self.peek().ty
            && let OperatorType::Plus | OperatorType::Minus | OperatorType::Inc | OperatorType::Dec =
                operator
        {
            self.index += 1;
            let expression = self.parse_unary()?;
            Ok(Expression::Unary {
                expression: Rc::new(RefCell::new(expression)),
                operator: UnaryOperator::from_operator(operator).unwrap(),
                is_prefix: true,
            })
        } else {
            let expression = self.parse_primary()?;
            if let TokenType::Operator { operator } = self.peek().ty {
                match operator {
                    OperatorType::Inc | OperatorType::Dec => {
                        self.index += 1;
                        Ok(Expression::Unary {
                            expression: Rc::new(RefCell::new(expression)),
                            operator: UnaryOperator::from_operator(operator).unwrap(),
                            is_prefix: false,
                        })
                    }
                    OperatorType::Not => {
                        if let TokenType::Operator { operator } = self.peek2().ty
                            && let OperatorType::Not = operator
                        {
                            self.index += 2;
                            Ok(Expression::Unary {
                                expression: Rc::new(RefCell::new(expression)),
                                operator: UnaryOperator::NotNullAssertation,
                                is_prefix: false,
                            })
                        } else {
                            Err(anyhow!("Not an operator token"))
                        }
                    }
                    _ => Ok(expression),
                }
            } else {
                Ok(expression)
            }
        }
    }
    fn parse_primary(&mut self) -> Result<Expression> {
        let token = self.peek();
        let mut expression = match token.ty {
            TokenType::IntegerLiteral { .. } => {
                self.index += 1;
                Ok(Expression::IntegerLiteral {
                    token: Box::new(token),
                })
            }
            TokenType::DecimalLiteral { .. } => {
                self.index += 1;
                Ok(Expression::DecimalLiteral {
                    token: Box::new(token),
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
                })
            }
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::If => self.parse_if(),
                _ => todo!(),
            },
            TokenType::Separator { separator } => match separator {
                SeparatorType::OpenBrace => self.parse_block(),
                SeparatorType::OpenParen => {
                    self.index += 1;
                    let t = self.next();
                    if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                        Ok(Expression::UnitLiteral {
                            left: Box::new(token),
                            right: Box::new(t),
                        })
                    } else {
                        Err(anyhow!(""))
                    }
                }
                _ => todo!(),
            },
            TokenType::Identifier => {
                self.index += 1;
                Ok(Expression::Variable {
                    name: Box::new(token),
                    ty: None,
                    symbol: None,
                })
            }
            _ => todo!(),
        }?;
        let mut token = self.peek();
        loop {
            match token.ty {
                TokenType::Separator { separator } => match separator {
                    SeparatorType::OpenParen => expression = self.parse_call(expression)?,
                    _ => break,
                },
                TokenType::Operator { operator } => match operator {
                    OperatorType::Less => expression = self.parse_call(expression)?,
                    _ => break,
                },
                _ => break,
            }
            token = self.peek();
        }
        Ok(expression)
    }
    fn parse_type_expression(&mut self) -> Result<Expression> {
        let name = self.next();
        let type_parameters = self.parse_type_parameters()?;
        Ok(Expression::Type {
            name: Box::new(name),
            type_parameters,
            ty: None,
        })
    }
    fn parse_function_decl(&mut self) -> Result<Statement> {
        let token = self.next();
        let name = self.next();
        if !SeparatorType::is_separator(&self.next(), SeparatorType::OpenParen) {
            return Err(anyhow!(""));
        }
        let mut parameters = Vec::new();
        while !self.is_empty()
            && !SeparatorType::is_separator(&self.peek(), SeparatorType::CloseParen)
        {
            let name = self.next();
            if !SeparatorType::is_separator(&self.next(), SeparatorType::Colon) {
                return Err(anyhow!(""));
            }
            let type_expression = self.parse_type_expression()?;
            parameters.push(Rc::new(RefCell::new(Parameter {
                name: Box::new(name),
                type_expression: Rc::new(RefCell::new(type_expression)),
                ty: None,
            })));
            let t = self.peek();
            if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                self.index += 1;
            } else {
                break;
            }
        }
        if !SeparatorType::is_separator(&self.next(), SeparatorType::CloseParen) {
            return Err(anyhow!(""));
        }
        let return_type = if OperatorType::is_operator(&self.peek(), OperatorType::Arrow) {
            self.index += 1;
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        if SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            let body = self.parse_block()?;
            Ok(Statement::FunctionDecl {
                token: Box::new(token),
                name: Box::new(name),
                generic_parameters: vec![],
                parameters,
                return_type: return_type.map(RefCell::new).map(Rc::new),
                body: Rc::new(RefCell::new(body)),
            })
        } else if OperatorType::is_operator(&self.peek(), OperatorType::Assign) {
            let expression = self.parse_expression()?;
            Ok(Statement::FunctionDecl {
                token: Box::new(token),
                name: Box::new(name),
                generic_parameters: vec![],
                parameters,
                return_type: return_type.map(RefCell::new).map(Rc::new),
                body: Rc::new(RefCell::new(expression)),
            })
        } else {
            Err(anyhow!(""))
        }
    }
    fn parse_variable_decl(&mut self) -> Result<Statement> {
        let token = self.next();
        let name = self.next();
        let type_expression = if SeparatorType::is_separator(&self.peek(), SeparatorType::Colon) {
            self.index += 1;
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        let initializer = if OperatorType::is_operator(&self.peek(), OperatorType::Assign) {
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
    fn parse_return(&mut self) -> Result<Statement> {
        let current_line = self.peek().position.line;
        self.index += 1;
        let value = if !self.is_empty() && current_line == self.peek().position.line {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::Return {
            value: value.map(RefCell::new).map(Rc::new),
        })
    }
    fn parse_loop(&mut self) -> Result<Statement> {
        self.index += 1;
        if SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            let body = self.parse_block()?;
            Ok(Statement::Loop {
                body: Rc::new(RefCell::new(body)),
            })
        } else {
            Err(anyhow!(""))
        }
    }
    fn parse_while(&mut self) -> Result<Statement> {
        self.index += 1;
        let condition = self.parse_expression()?;
        if SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            let body = self.parse_block()?;
            Ok(Statement::While {
                condition: Rc::new(RefCell::new(condition)),
                body: Rc::new(RefCell::new(body)),
            })
        } else {
            Err(anyhow!(""))
        }
    }
    fn parse_repeat_while(&mut self) -> Result<Statement> {
        self.index += 1;
        if !SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            return Err(anyhow!(""));
        }
        let body = self.parse_block()?;
        if KeywordType::is_keyword(&self.peek(), KeywordType::While) {
            self.index += 1;
            let condition = self.parse_expression()?;
            Ok(Statement::RepeatWhile {
                body: Rc::new(RefCell::new(body)),
                condition: Rc::new(RefCell::new(condition)),
            })
        } else {
            Err(anyhow!(""))
        }
    }
    fn parse_for(&mut self) -> Result<Statement> {
        self.index += 1;
        let pattern = self.parse_pattern()?;
        if !KeywordType::is_keyword(&self.next(), KeywordType::In) {
            return Err(anyhow!(""));
        }
        let iterator = self.parse_expression()?;
        if SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            let body = self.parse_block()?;
            Ok(Statement::For {
                pattern: Rc::new(pattern),
                iterator: Rc::new(RefCell::new(iterator)),
                body: Rc::new(RefCell::new(body)),
            })
        } else {
            Err(anyhow!(""))
        }
    }
    fn parse_throw(&mut self) -> Result<Statement> {
        self.index += 1;
        let exception = self.parse_expression()?;
        Ok(Statement::Throw {
            exception: Rc::new(RefCell::new(exception)),
        })
    }
    fn parse_block(&mut self) -> Result<Expression> {
        self.index += 1;
        let mut statements = Vec::new();
        while !self.is_empty()
            && !SeparatorType::is_separator(&self.peek(), SeparatorType::CloseBrace)
        {
            statements.push(Rc::new(RefCell::new(self.parse_statement()?)));
        }
        if SeparatorType::is_separator(&self.next(), SeparatorType::CloseBrace) {
            Ok(Expression::Block { statements })
        } else {
            Err(anyhow!(""))
        }
    }
    fn parse_call(&mut self, callee: Expression) -> Result<Expression> {
        let type_parameters = self.parse_type_parameters()?;
        let mut parameters = Vec::new();
        if !SeparatorType::is_separator(&self.next(), SeparatorType::OpenParen) {
            return Err(anyhow!(""));
        }
        while !self.is_empty()
            && !SeparatorType::is_separator(&self.peek(), SeparatorType::CloseParen)
        {
            parameters.push(Rc::new(RefCell::new(self.parse_expression()?)));
            let t = self.peek();
            if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                self.index += 1;
            } else {
                break;
            }
        }
        if SeparatorType::is_separator(&self.next(), SeparatorType::CloseParen) {
            Ok(Expression::Call {
                callee: Rc::new(RefCell::new(callee)),
                type_parameters,
                parameters,
            })
        } else {
            Err(anyhow!(""))
        }
    }
    fn parse_if(&mut self) -> Result<Expression> {
        self.index += 1;
        let condition = self.parse_expression()?;
        if !SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            return Err(anyhow!(""));
        }
        let then = self.parse_block()?;
        let else_ = if KeywordType::is_keyword(&self.peek(), KeywordType::Else) {
            self.index += 1;
            if KeywordType::is_keyword(&self.peek(), KeywordType::If) {
                Some(self.parse_if()?)
            } else if SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
                Some(self.parse_block()?)
            } else {
                return Err(anyhow!(""));
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
    fn parse_type_parameters(&mut self) -> Result<Option<Vec<Rc<RefCell<Expression>>>>> {
        if OperatorType::is_operator(&self.peek(), OperatorType::Less) {
            self.index += 1;
            let mut type_parameters = Vec::new();
            while !self.is_empty()
                && !OperatorType::is_operator(&self.peek(), OperatorType::Greater)
            {
                type_parameters.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
                let t = self.peek();
                if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                    self.index += 1;
                } else {
                    break;
                }
            }
            if !OperatorType::is_operator(&self.next(), OperatorType::Greater) {
                return Err(anyhow!(""));
            }
            Ok(Some(type_parameters))
        } else {
            Ok(None)
        }
    }
    fn parse_pattern(&mut self) -> Result<Pattern> {
        let token = self.next();
        match token.ty {
            TokenType::Identifier => Ok(Pattern::Identifier(Box::new(token))),
            TokenType::Separator { separator } => match separator {
                SeparatorType::OpenParen => {
                    let mut patterns = Vec::new();
                    while !self.is_empty()
                        && !SeparatorType::is_separator(&self.peek(), SeparatorType::CloseParen)
                    {
                        patterns.push(self.parse_pattern()?);
                        let t = self.peek();
                        if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                            self.index += 1;
                        } else {
                            break;
                        }
                    }
                    Ok(Pattern::Tuple(patterns))
                }
                SeparatorType::Underscore => Ok(Pattern::Ignore),
                _ => Err(anyhow!("")),
            },
            _ => Err(anyhow!("")),
        }
    }
}
