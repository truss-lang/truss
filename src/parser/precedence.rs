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
    pub fn get_precedence(token: &Token) -> Result<Precedence> {
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
                | OperatorType::RightShiftAssign => Ok(Precedence::Assignment),
                OperatorType::Or => Ok(Precedence::Or),
                OperatorType::And => Ok(Precedence::And),
                OperatorType::BitOr => Ok(Precedence::BitOr),
                OperatorType::BitAnd => Ok(Precedence::BitAnd),
                OperatorType::Equal | OperatorType::NotEqual => Ok(Precedence::Equality),
                OperatorType::Less
                | OperatorType::LessEqual
                | OperatorType::Greater
                | OperatorType::GreaterEqual => Ok(Precedence::Relational),
                OperatorType::LeftShift | OperatorType::RightShift => Ok(Precedence::Shift),
                OperatorType::Plus | OperatorType::Minus => Ok(Precedence::Additive),
                OperatorType::Multiply | OperatorType::Divide | OperatorType::Modulus => {
                    Ok(Precedence::Multiplicative)
                }
                _ => Err(anyhow!("Not an operator token")),
            },
            TokenType::Separator { separator } => {
                if let SeparatorType::OpenBracket = separator {
                    Ok(Precedence::Postfix)
                } else {
                    Err(anyhow!("Not an operator token"))
                }
            }
            _ => Err(anyhow!("Not an operator token")),
        }
    }
}
