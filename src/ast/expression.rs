use crate::lexer::token::Token;

use super::statement::Statement;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Block {
        statements: Vec<Statement>,
    },
    IntegerLiteral {
        token: Token,
    },
    DecimalLiteral {
        token: Token,
    },
    BooleanLiteral {
        token: Token,
    },
    NullLiteral {
        token: Token,
    },
    NullptrLiteral {
        token: Token,
    },
    Type {
        name: Token,
        generic_parameters: Vec<Expression>,
    },
}
