use std::rc::Rc;

use anyhow::{Result, anyhow};

use crate::{
    ast::{expression::Expression, node::Program, statement::Statement},
    lexer::{
        Lexer,
        token::{KeywordType, SeparatorType, Token, TokenType},
    },
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
            program.statements.push(self.parse_statement()?);
        }
        Ok(program)
    }
    fn parse_statement(&mut self) -> Result<Statement> {
        match self.peek().ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(),
                _ => todo!(),
            },
            _ => Ok(Statement::ExpressionStatement {
                expression: Box::new(self.parse_expression()?),
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
    fn parse_function_decl(&mut self) -> Result<Statement> {
        let token = self.next();
        let name = self.next();
        if !SeparatorType::is_separator(&self.next(), SeparatorType::OpenParen) {
            return Err(anyhow!(""));
        }
        if !SeparatorType::is_separator(&self.next(), SeparatorType::CloseParen) {
            return Err(anyhow!(""));
        }

        if !SeparatorType::is_separator(&self.peek(), SeparatorType::OpenBrace) {
            return Err(anyhow!(""));
        }
        let body = self.parse_block()?;
        Ok(Statement::FunctionDecl {
            token: Box::new(token),
            name: Box::new(name),
            parameters: vec![],
            generic_parameters: vec![],
            body: Box::new(body),
        })
    }
    fn parse_block(&mut self) -> Result<Expression> {
        self.index += 1;
        let mut statements: Vec<Statement> = Vec::new();
        while !self.is_empty()
            && !SeparatorType::is_separator(&self.peek(), SeparatorType::CloseBrace)
        {
            statements.push(self.parse_statement()?);
        }
        if SeparatorType::is_separator(&self.next(), SeparatorType::CloseBrace) {
            Ok(Expression::Block { statements })
        } else {
            Err(anyhow!(""))
        }
    }
}
