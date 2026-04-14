use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{
    ast::{expression::Expression, node::Program, statement::Statement},
    lexer::token::{KeywordType, OperatorType, SeparatorType, Token, TokenType},
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
        match self.peek().ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(),
                KeywordType::Let | KeywordType::Var => self.parse_variable_decl(),
                _ => todo!(),
            },
            _ => Ok(Statement::ExpressionStatement {
                expression: Rc::new(RefCell::new(self.parse_expression()?)),
            }),
        }
    }
    fn parse_expression(&mut self) -> Result<Expression> {
        let token = self.next();
        match token.ty {
            TokenType::IntegerLiteral { .. } => Ok(Expression::IntegerLiteral { token }),
            TokenType::DecimalLiteral { .. } => Ok(Expression::DecimalLiteral { token }),
            TokenType::BooleanLiteral { .. } => Ok(Expression::BooleanLiteral { token }),
            TokenType::NullLiteral => Ok(Expression::NullLiteral { token }),
            TokenType::NullptrLiteral => Ok(Expression::NullptrLiteral { token }),
            TokenType::Separator { separator } => match separator {
                SeparatorType::OpenBrace => self.parse_block(),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
    fn parse_type_expression(&mut self) -> Result<Expression> {
        let name = self.next();
        let mut generic_parameters = Vec::new();
        if OperatorType::is_operator(&self.peek(), OperatorType::Less) {
            self.index += 1;
            while !self.is_empty()
                && !OperatorType::is_operator(&self.peek(), OperatorType::Greater)
            {
                generic_parameters.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
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
        }
        Ok(Expression::Type {
            name,
            generic_parameters,
            ty: None,
        })
    }
    fn parse_function_decl(&mut self) -> Result<Statement> {
        let token = self.next();
        let name = self.next();
        if !SeparatorType::is_separator(&self.next(), SeparatorType::OpenParen) {
            return Err(anyhow!(""));
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
        if !SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            return Err(anyhow!(""));
        }
        let body = self.parse_block()?;
        Ok(Statement::FunctionDecl {
            token: Box::new(token),
            name: Box::new(name),
            generic_parameters: vec![],
            parameters: vec![],
            return_type: return_type.map(RefCell::new).map(Rc::new),
            body: Rc::new(RefCell::new(body)),
        })
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
}
