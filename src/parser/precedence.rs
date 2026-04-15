use anyhow::{Result, anyhow};

use crate::lexer::token::{OperatorType, SeparatorType, Token, TokenType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Assignment,
    Or,
    And,
    BitOr,
    BitAnd,
    Equality,
    Relational,
    Shift,
    Additive,
    Multiplicative,
    Postfix,
    Cast,
}

impl Precedence {
    pub fn get_precedence(token: &Token) -> Option<Precedence> {
        match token.ty {
            TokenType::Operator { operator } => match operator {
                OperatorType::Assign
                | OperatorType::PlusAssign
                | OperatorType::MinusAssign
                | OperatorType::MultiplyAssign
                | OperatorType::DivideAssign
                | OperatorType::ModulusAssign
                | OperatorType::BitAndAssign
                | OperatorType::BitOrAssign
                | OperatorType::LeftShiftAssign
                | OperatorType::RightShiftAssign => Some(Precedence::Assignment),
                OperatorType::Or => Some(Precedence::Or),
                OperatorType::And => Some(Precedence::And),
                OperatorType::BitOr => Some(Precedence::BitOr),
                OperatorType::BitAnd => Some(Precedence::BitAnd),
                OperatorType::Equal | OperatorType::NotEqual => Some(Precedence::Equality),
                OperatorType::Less
                | OperatorType::LessEqual
                | OperatorType::Greater
                | OperatorType::GreaterEqual => Some(Precedence::Relational),
                OperatorType::LeftShift | OperatorType::RightShift => Some(Precedence::Shift),
                OperatorType::Plus | OperatorType::Minus => Some(Precedence::Additive),
                OperatorType::Multiply | OperatorType::Divide | OperatorType::Modulus => {
                    Some(Precedence::Multiplicative)
                }
                _ => None,
            },
            TokenType::Separator { separator } => {
                if let SeparatorType::OpenBracket = separator {
                    Some(Precedence::Postfix)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
