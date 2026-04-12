use anyhow::Result;

use crate::{
    ast::{expression::Expression, node::Program, statement::Statement},
    lexer::{
        Lexer,
        token::{KeywordType, SeparatorType, TokenType},
    },
};

#[derive(Debug)]
pub struct Parser {
    lexer: Lexer,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Self {
        Self { lexer }
    }
    pub fn parse(&mut self) -> Result<Program> {
        let mut program = Program::new(self.lexer.get_file());
        while !self.lexer.is_empty() {
            program.statements.push(self.parse_statement()?);
        }
        Ok(program)
    }
    fn parse_statement(&mut self) -> Result<Statement> {
        match self.lexer.next().unwrap().ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
    fn parse_function_decl(&mut self) -> Result<Statement> {
        let token = self.lexer.peek();
        let name = self.lexer.next().unwrap();
        if let TokenType::Separator { separator } = self.lexer.next().unwrap().ty
            && separator == SeparatorType::OpenParen
        {}
        if let TokenType::Separator { separator } = self.lexer.next().unwrap().ty
            && separator == SeparatorType::CloseParen
        {}
        Ok(Statement::FunctionDecl {
            token,
            name,
            parameters: vec![],
            generic_parameters: vec![],
        })
    }
    fn parse_block(&mut self) -> Result<Expression> {
        while SeparatorType::is_separator(&self.lexer.peek(), SeparatorType::OpenBrace) {}
        Ok(Expression::Block { statements: vec![] })
    }
}
