pub mod precedence;

use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        expression::{
            AssignmentOperator, BinaryOperator, CallParameter, CastKind, ClosureCapture,
            ClosureParameter, ElseBranch, Expression, MacroDelimiter, TryKind, UnaryOperator,
        },
        node::Program,
        statement::{
            AccessModifier, Accessor, AccessorKind, AsmDirection, AsmOperand, Attribute,
            CatchClause, Condition, ConditionalClause, EnumCase, EnumCaseParameter, FunctionBody,
            GenericParameter, GenericParameterKind, ImportKind, MacroArm, MacroMetaVarType,
            MacroPatternFragment, MatchCase, Modifier, ModifierType, OperatorFixity,
            OwnershipModifier, Parameter, Pattern, ProtocolAccessorSet, ProtocolMember,
            SelectiveAlias, SelectiveMember, Statement, VariadicKind, WhereRequirement,
            WhereRequirementKind,
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
    pending_greater_count: u32,
    scope_nesting: usize,
    suppress_trailing_closure: bool,
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
            pending_greater_count: 0,
            scope_nesting: 0,
            suppress_trailing_closure: false,
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
        let attributes = self.parse_attributes()?;
        if let Some(token) = self.peek() {
            if SeparatorType::is_separator(&token, SeparatorType::Hash) {
                return self.parse_preprocessor_directive();
            }
        }
        let modifiers = self.parse_modifiers()?;
        let Some(token) = self.peek() else {
            return Err(());
        };
        match token.ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(false, attributes, modifiers),
                KeywordType::Let | KeywordType::Var => self.parse_variable_decl(false, modifiers),
                KeywordType::Struct => self.parse_struct_decl(attributes, modifiers),
                KeywordType::Class => self.parse_class_decl(modifiers),
                KeywordType::Protocol => self.parse_protocol_decl(modifiers),
                KeywordType::Enum => self.parse_enum_decl(modifiers),
                KeywordType::Extern => self.parse_extern(attributes, modifiers),
                KeywordType::Init => self.parse_function_decl(false, attributes, modifiers),
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
                KeywordType::Yield => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_yield()
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
                KeywordType::Guard => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_guard()
                }
                KeywordType::Fallthrough => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_fallthrough()
                }
                KeywordType::Break => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_break()
                }
                KeywordType::Defer => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_defer()
                }
                KeywordType::Extension => self.parse_extension_decl(modifiers),
                KeywordType::Typealias => self.parse_typealias(modifiers),
                KeywordType::Module => self.parse_module_decl(modifiers),
                KeywordType::Import => self.parse_import(),
                KeywordType::Subscript => self.parse_subscript_decl(modifiers),
                KeywordType::Macro => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_macro_decl()
                }
                KeywordType::Asm => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    self.parse_asm_block()
                }
                _ => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            format!("Modifiers are not allowed on '{}' declaration", token.value),
                            &modifiers[0].token,
                        );
                    }
                    let expr = self.parse_expression()?;
                    let expr = self.apply_trailing_closure(expr)?;
                    Ok(Statement::ExpressionStatement {
                        expression: Rc::new(RefCell::new(expr)),
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
                SeparatorType::OpenParen => {
                    if !modifiers.is_empty() {
                        self.emit_error(
                            TrussDiagnosticCode::ModifierNotAllowedHere,
                            "Modifiers are not allowed on expression statement.",
                            &modifiers[0].token,
                        );
                    }
                    let expr = self.parse_expression()?;
                    let expr = self.apply_trailing_closure(expr)?;
                    Ok(Statement::ExpressionStatement {
                        expression: Rc::new(RefCell::new(expr)),
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
                let expr = self.parse_expression()?;
                let expr = self.apply_trailing_closure(expr)?;
                Ok(Statement::ExpressionStatement {
                    expression: Rc::new(RefCell::new(expr)),
                })
            }
        }
    }

    fn apply_trailing_closure(&mut self, expr: Expression) -> Result<Expression, ()> {
        if !self.suppress_trailing_closure
            && let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
            && matches!(
                expr,
                Expression::Variable { .. }
                    | Expression::MemberAccess { .. }
                    | Expression::Call { .. }
            )
        {
            let closure = self.parse_closure_expression()?;
            Ok(Expression::Call {
                callee: Rc::new(RefCell::new(expr)),
                type_parameters: None,
                parameters: vec![CallParameter {
                    label: None,
                    expression: Rc::new(RefCell::new(closure)),
                }],
                overloads: vec![],
                selected_index: None,
            })
        } else {
            Ok(expr)
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
                    let op = BinaryOperator::from_operator(operator);
                    let Some(op) = op else {
                        self.emit_error(
                            TrussDiagnosticCode::InvalidOperator,
                            format!("Invalid binary operator '{}'", token.value),
                            &token,
                        );
                        return Err(());
                    };
                    if matches!(
                        op,
                        BinaryOperator::RangeTo
                            | BinaryOperator::RangeUntil
                            | BinaryOperator::OpenRange
                    ) {
                        let range_token = Token::new(
                            "range".to_string(),
                            TokenType::Identifier,
                            token.position.clone(),
                            token.file.clone(),
                        );
                        let from_label = Token::new(
                            "from".to_string(),
                            TokenType::Identifier,
                            token.position.clone(),
                            token.file.clone(),
                        );
                        let to_label = Token::new(
                            "to".to_string(),
                            TokenType::Identifier,
                            token.position.clone(),
                            token.file.clone(),
                        );
                        left = Expression::Call {
                            callee: Rc::new(RefCell::new(Expression::Variable {
                                name: Box::new(range_token),
                                ty: None,
                                symbol: None,
                            })),
                            type_parameters: None,
                            parameters: vec![
                                crate::ast::expression::CallParameter {
                                    label: Some(Box::new(from_label)),
                                    expression: Rc::new(RefCell::new(left)),
                                },
                                crate::ast::expression::CallParameter {
                                    label: Some(Box::new(to_label)),
                                    expression: Rc::new(RefCell::new(right)),
                                },
                            ],
                            overloads: vec![],
                            selected_index: None,
                        };
                    } else {
                        left = Expression::Binary {
                            left: Rc::new(RefCell::new(left)),
                            operator: op,
                            right: Rc::new(RefCell::new(right)),
                            overloads: vec![],
                            selected_index: None,
                        };
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
            | OperatorType::Not
            | OperatorType::BitNot
            | OperatorType::BitAnd
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
                overloads: vec![],
                selected_index: None,
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
                            overloads: vec![],
                            selected_index: None,
                        })
                    }
                    OperatorType::Not => Ok(expression),
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
                    ty: None,
                })
            }
            TokenType::StringLiteral { .. } => {
                self.index += 1;
                let value = match &token.ty {
                    TokenType::StringLiteral { value } => value.clone(),
                    _ => unreachable!(),
                };
                Ok(Expression::StringLiteral {
                    token: Box::new(token),
                    value,
                    ty: None,
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
                KeywordType::Match => self.parse_match(),
                KeywordType::SelfKw => {
                    self.index += 1;
                    Ok(Expression::SelfKeyword {
                        token: Box::new(token),
                        ty: None,
                        symbol: None,
                    })
                }
                KeywordType::SelfType => {
                    self.index += 1;
                    Ok(Expression::SelfType {
                        token: Box::new(token),
                        ty: None,
                    })
                }
                KeywordType::SuperKw => {
                    self.index += 1;
                    Ok(Expression::SuperKeyword {
                        token: Box::new(token),
                        ty: None,
                    })
                }
                KeywordType::SizeOf => {
                    self.index += 1;
                    if let Some(t) = self.peek()
                        && SeparatorType::is_separator(&t, SeparatorType::OpenParen)
                    {
                        self.index += 1;
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::MissingSeparator,
                            "Expected '(' after 'sizeof'".to_string(),
                            &token,
                        );
                        return Err(());
                    }
                    let argument = Rc::new(RefCell::new(self.parse_type_expression()?));
                    if let Some(t) = self.peek()
                        && SeparatorType::is_separator(&t, SeparatorType::CloseParen)
                    {
                        self.index += 1;
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::MissingSeparator,
                            "Expected ')' after sizeof type".to_string(),
                            &token,
                        );
                        return Err(());
                    }
                    Ok(Expression::SizeOf {
                        token: Box::new(token),
                        argument,
                        ty: None,
                    })
                }
                KeywordType::Do => self.parse_do_expression(),
                KeywordType::Try => self.parse_try_expression(),
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
                SeparatorType::OpenBrace => self.parse_closure_expression(),
                SeparatorType::OpenBracket => {
                    self.index += 1;
                    let left = token;
                    let mut elements = Vec::new();

                    if let Some(t) = self.peek()
                        && !SeparatorType::is_separator(&t, SeparatorType::CloseBracket)
                    {
                        let first_expr = self.parse_expression()?;
                        elements.push(Rc::new(RefCell::new(first_expr)));

                        while let Some(t) = self.peek()
                            && SeparatorType::is_separator(&t, SeparatorType::Comma)
                        {
                            self.index += 1;
                            if let Some(next) = self.peek()
                                && SeparatorType::is_separator(&next, SeparatorType::CloseBracket)
                            {
                                break;
                            }
                            let expr = self.parse_expression()?;
                            elements.push(Rc::new(RefCell::new(expr)));
                        }
                    }

                    let Some(right) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            "Expected closing bracket",
                            &left,
                        );
                        return Err(());
                    };
                    if !SeparatorType::is_separator(&right, SeparatorType::CloseBracket) {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            format!("Expected ']' but found '{}'", right.value),
                            &right,
                        );
                        return Err(());
                    }

                    Ok(Expression::ArrayLiteral {
                        left: Box::new(left),
                        elements,
                        right: Box::new(right),
                        ty: None,
                    })
                }
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
                            if !SeparatorType::is_separator(&right, SeparatorType::CloseParen) {
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
                            if !SeparatorType::is_separator(&right, SeparatorType::CloseParen) {
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
            TokenType::Operator { operator } => match operator {
                OperatorType::Dollar => {
                    self.index += 1;
                    let Some(idx_token) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedExpression,
                            "Expected integer after '$'",
                            &self.tokens[self.index.saturating_sub(1)],
                        );
                        return Err(());
                    };
                    if let TokenType::IntegerLiteral { value } = idx_token.ty {
                        Ok(Expression::ShorthandArgument {
                            index: value as u32,
                            ty: None,
                        })
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            format!("Expected integer after '$' but found '{}'", idx_token.value),
                            &idx_token,
                        );
                        Err(())
                    }
                }
                OperatorType::Dot => {
                    self.index += 1;
                    let Some(member_token) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedExpression,
                            "Expected member name after '.'",
                            &self.tokens[self.index.saturating_sub(1)],
                        );
                        return Err(());
                    };
                    if TokenType::Identifier != member_token.ty {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            format!(
                                "Expected identifier after '.' but found '{}'",
                                member_token.value
                            ),
                            &member_token,
                        );
                        return Err(());
                    }
                    Ok(Expression::ImplicitMemberAccess {
                        member: Box::new(member_token),
                        ty: None,
                    })
                }
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedExpression,
                        format!("Unexpected operator '{}'", token.value),
                        &token,
                    );
                    Err(())
                }
            },
            TokenType::Identifier => {
                self.index += 1;
                if let Some(next) = self.peek()
                    && OperatorType::is_operator(&next, OperatorType::Not)
                {
                    let is_macro = match self.peek2() {
                        Some(d)
                            if SeparatorType::is_separator(&d, SeparatorType::OpenParen)
                                || SeparatorType::is_separator(&d, SeparatorType::OpenBracket)
                                || SeparatorType::is_separator(&d, SeparatorType::OpenBrace) =>
                        {
                            true
                        }
                        _ => false,
                    };
                    if is_macro {
                        self.index += 1;
                        let delimiter = if let Some(d) = self.peek() {
                            if SeparatorType::is_separator(&d, SeparatorType::OpenParen) {
                                MacroDelimiter::Paren
                            } else if SeparatorType::is_separator(&d, SeparatorType::OpenBracket) {
                                MacroDelimiter::Bracket
                            } else {
                                MacroDelimiter::Brace
                            }
                        } else {
                            unreachable!()
                        };
                        let close_delim = match delimiter {
                            MacroDelimiter::Paren => SeparatorType::CloseParen,
                            MacroDelimiter::Bracket => SeparatorType::CloseBracket,
                            MacroDelimiter::Brace => SeparatorType::CloseBrace,
                        };
                        self.next().unwrap();
                        let mut depth = 1u32;
                        let mut arguments = Vec::new();
                        while let Some(ref t) = self.peek() {
                            if SeparatorType::is_separator(t, close_delim) && depth == 1 {
                                self.index += 1;
                                break;
                            }
                            if SeparatorType::is_separator(t, SeparatorType::OpenParen)
                                || SeparatorType::is_separator(t, SeparatorType::OpenBracket)
                                || SeparatorType::is_separator(t, SeparatorType::OpenBrace)
                            {
                                depth += 1;
                            } else if SeparatorType::is_separator(t, SeparatorType::CloseParen)
                                || SeparatorType::is_separator(t, SeparatorType::CloseBracket)
                                || SeparatorType::is_separator(t, SeparatorType::CloseBrace)
                            {
                                depth -= 1;
                                if depth == 0 {
                                    self.index += 1;
                                    break;
                                }
                            }
                            arguments.push(t.clone());
                            self.index += 1;
                        }
                        Ok(Expression::MacroInvocation {
                            name: Box::new(token),
                            delimiter,
                            arguments,
                            ty: None,
                        })
                    } else {
                        Ok(Expression::Variable {
                            name: Box::new(token),
                            ty: None,
                            symbol: None,
                        })
                    }
                } else {
                    Ok(Expression::Variable {
                        name: Box::new(token),
                        ty: None,
                        symbol: None,
                    })
                }
            }
        }?;
        while !self.is_empty() {
            let Some(token) = self.peek() else { break };
            match token.ty {
                TokenType::Separator { separator } => match separator {
                    SeparatorType::OpenParen => expression = self.parse_call(expression)?,
                    SeparatorType::OpenBracket => {
                        self.index += 1;
                        let mut parameters = Vec::new();
                        while let Some(token) = self.peek() {
                            if SeparatorType::is_separator(&token, SeparatorType::CloseBracket) {
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
                        let Some(close) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                "Expected ']' to close subscript",
                                &self.tokens[self.index.saturating_sub(1)],
                            );
                            return Err(());
                        };
                        if !SeparatorType::is_separator(&close, SeparatorType::CloseBracket) {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                format!("Expected ']' but found '{}'", close.value),
                                &close,
                            );
                            return Err(());
                        }
                        expression = Expression::SubscriptAccess {
                            object: Rc::new(RefCell::new(expression)),
                            parameters,
                            ty: None,
                        };
                    }
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
                        } else if TokenType::Identifier == member_token.ty
                            || matches!(
                                member_token.ty,
                                TokenType::Keyword {
                                    keyword: KeywordType::Deinit
                                }
                            )
                            || matches!(
                                member_token.ty,
                                TokenType::Keyword {
                                    keyword: KeywordType::Init
                                }
                            )
                        {
                            expression = Expression::MemberAccess {
                                object: Rc::new(RefCell::new(expression)),
                                member: Box::new(member_token),
                                ty: None,
                            };
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::ExpectedIdentifier,
                                format!(
                                    "Expected member name or index but found '{}'",
                                    member_token.value
                                ),
                                &member_token,
                            );
                            return Err(());
                        }
                    }
                    OperatorType::Less
                        if matches!(
                            expression,
                            Expression::Variable { .. }
                                | Expression::Type { .. }
                                | Expression::MemberAccess { .. }
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
                                } else if OperatorType::is_operator(
                                    &self.tokens[temp_idx],
                                    OperatorType::RightShift,
                                ) {
                                    angle_count -= 2;
                                } else if OperatorType::is_operator(
                                    &self.tokens[temp_idx],
                                    OperatorType::RightShiftAssign,
                                ) {
                                    angle_count -= 2;
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
                    OperatorType::QuestionMark => {
                        self.index += 1;
                        if let Some(dot) = self.peek()
                            && OperatorType::is_operator(&dot, OperatorType::Dot)
                        {
                            self.index += 1;
                            let Some(member_token) = self.next() else {
                                self.emit_error(
                                    TrussDiagnosticCode::ExpectedExpression,
                                    "Expected member name after '?.'",
                                    &token,
                                );
                                return Err(());
                            };
                            if TokenType::Identifier == member_token.ty
                                || matches!(member_token.ty, TokenType::IntegerLiteral { .. })
                                || matches!(member_token.ty, TokenType::DecimalLiteral { .. })
                                || matches!(
                                    member_token.ty,
                                    TokenType::Keyword {
                                        keyword: KeywordType::Deinit
                                    }
                                )
                                || matches!(
                                    member_token.ty,
                                    TokenType::Keyword {
                                        keyword: KeywordType::Init
                                    }
                                )
                            {
                                expression = Expression::OptionalChain {
                                    token: Box::new(token),
                                    object: Rc::new(RefCell::new(expression)),
                                    member: Box::new(member_token),
                                    ty: None,
                                };
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::ExpectedIdentifier,
                                    format!(
                                        "Expected member name or index after '?.' but found '{}'",
                                        member_token.value
                                    ),
                                    &member_token,
                                );
                                return Err(());
                            }
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UnexpectedToken,
                                "Expected '.' after '?' for optional chaining",
                                &token,
                            );
                            return Err(());
                        }
                    }
                    OperatorType::Not => {
                        self.index += 1;
                        expression = Expression::Unary {
                            expression: Rc::new(RefCell::new(expression)),
                            operator: UnaryOperator::NotNullAssertation,
                            is_prefix: false,
                            overloads: vec![],
                            selected_index: None,
                        };
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
            && SeparatorType::is_separator(&token, SeparatorType::OpenBracket)
        {
            self.index += 1;
            let element = self.parse_type_expression()?;
            let Some(close) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    "Expected ']' in array type",
                    &token,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseBracket) {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    format!("Expected ']' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }
            return Ok(Expression::ArrayType {
                inner: Rc::new(RefCell::new(element)),
                ty: None,
            });
        }

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

        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::Some)
        {
            self.index += 1;
            let inner = self.parse_type_expression()?;

            return Ok(Expression::SomeType {
                inner: Rc::new(RefCell::new(inner)),
                ty: None,
            });
        }

        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::SelfType)
        {
            self.index += 1;
            return Ok(Expression::SelfType {
                token: Box::new(token),
                ty: None,
            });
        }

        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::Func)
        {
            self.index += 1;
            let Some(open) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    "Expected '(' after 'func' in function pointer type",
                    &token,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&open, SeparatorType::OpenParen) {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    format!("Expected '(' but found '{}'", open.value),
                    &open,
                );
                return Err(());
            }

            let mut param_types = Vec::new();
            loop {
                if let Some(t) = self.peek()
                    && SeparatorType::is_separator(&t, SeparatorType::CloseParen)
                {
                    break;
                }
                let param_type = self.parse_type_expression()?;
                param_types.push(Rc::new(RefCell::new(param_type)));
                if let Some(t) = self.peek()
                    && SeparatorType::is_separator(&t, SeparatorType::Comma)
                {
                    self.index += 1;
                } else {
                    break;
                }
            }

            let Some(close) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    "Expected ')' in function pointer type",
                    &token,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseParen) {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    format!("Expected ')' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }

            let Some(arrow) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    "Expected '->' in function pointer type",
                    &close,
                );
                return Err(());
            };
            if !OperatorType::is_operator(&arrow, OperatorType::Arrow) {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    format!("Expected '->' but found '{}'", arrow.value),
                    &arrow,
                );
                return Err(());
            }

            let return_type = self.parse_type_expression()?;
            return Ok(Expression::FunctionType {
                param_types,
                return_type: Rc::new(RefCell::new(return_type)),
                ty: None,
            });
        }

        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::Inline)
        {
            self.index += 1;
            let inline_token = token;

            let size = if let Some(t) = self.peek()
                && OperatorType::is_operator(&t, OperatorType::Less)
            {
                self.index += 1;
                if let Some(t) = self.peek()
                    && OperatorType::is_operator(&t, OperatorType::Greater)
                {
                    self.index += 1;
                    None
                } else {
                    let size_expr = Rc::new(RefCell::new(self.parse_primary()?));
                    let Some(next) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::MissingSeparator,
                            "Expected '>' to close inline size".to_string(),
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
                    Some(size_expr)
                }
            } else {
                None
            };

            let base = Rc::new(RefCell::new(self.parse_type_expression()?));
            return Ok(Expression::InlineType {
                token: Box::new(inline_token),
                size,
                base,
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
                if let Some(token) = self.peek()
                    && OperatorType::is_operator(&token, OperatorType::Arrow)
                {
                    self.index += 1;
                    let return_type = self.parse_type_expression()?;
                    return Ok(Expression::ClosureType {
                        param_types: vec![],
                        return_type: Rc::new(RefCell::new(return_type)),
                        ty: None,
                    });
                }
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

                if let Some(token) = self.peek()
                    && OperatorType::is_operator(&token, OperatorType::Arrow)
                {
                    self.index += 1;
                    let return_type = self.parse_type_expression()?;
                    return Ok(Expression::ClosureType {
                        param_types: elements.into_iter().map(|(_, t)| t).collect(),
                        return_type: Rc::new(RefCell::new(return_type)),
                        ty: None,
                    });
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
                    let mut non_null = false;
                    if let Some(next) = self.peek()
                        && OperatorType::is_operator(&next, OperatorType::Not)
                    {
                        self.index += 1;
                        non_null = true;
                    }
                    tuple_type_expr = Expression::PointerType {
                        base: Box::new(Rc::new(RefCell::new(tuple_type_expr))),
                        non_null,
                        ty: None,
                    };
                }

                if let Some(token) = self.peek()
                    && OperatorType::is_operator(&token, OperatorType::QuestionMark)
                {
                    self.index += 1;
                    tuple_type_expr = Expression::OptionalType {
                        inner: Rc::new(RefCell::new(tuple_type_expr)),
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

            if let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::Arrow)
            {
                self.index += 1;
                let return_type = self.parse_type_expression()?;
                return Ok(Expression::ClosureType {
                    param_types: vec![first],
                    return_type: Rc::new(RefCell::new(return_type)),
                    ty: None,
                });
            }

            let mut type_expr = Rc::try_unwrap(first).ok().unwrap().into_inner();

            while let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::Dot)
            {
                self.index += 1;
                let Some(member_token) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedType,
                        "Expected associated type name after '.'",
                        &self.tokens[self.index.saturating_sub(1)],
                    );
                    return Err(());
                };
                if TokenType::Identifier != member_token.ty {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedType,
                        format!(
                            "Expected associated type name but found '{}'",
                            member_token.value
                        ),
                        &member_token,
                    );
                    return Err(());
                }
                type_expr = Expression::AssociatedTypeAccess {
                    object: Rc::new(RefCell::new(type_expr)),
                    member: Box::new(member_token),
                    ty: None,
                };
            }

            while let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::Multiply)
            {
                self.index += 1;
                let mut non_null = false;
                if let Some(next) = self.peek()
                    && OperatorType::is_operator(&next, OperatorType::Not)
                {
                    self.index += 1;
                    non_null = true;
                }
                type_expr = Expression::PointerType {
                    base: Box::new(Rc::new(RefCell::new(type_expr))),
                    non_null,
                    ty: None,
                };
            }

            if let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::QuestionMark)
            {
                self.index += 1;
                type_expr = Expression::OptionalType {
                    inner: Rc::new(RefCell::new(type_expr)),
                    ty: None,
                };
            }

            let mut types = vec![Rc::new(RefCell::new(type_expr))];
            while let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::BitAnd)
            {
                self.index += 1;
                let right = self.parse_type_expression()?;
                if let Expression::CompoundType {
                    types: inner_types, ..
                } = right
                {
                    types.extend(inner_types);
                } else {
                    types.push(Rc::new(RefCell::new(right)));
                }
            }

            if types.len() > 1 {
                return Ok(Expression::CompoundType { types, ty: None });
            } else {
                return Ok(Rc::try_unwrap(types.into_iter().next().unwrap())
                    .ok()
                    .unwrap()
                    .into_inner());
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
            && OperatorType::is_operator(&token, OperatorType::Dot)
        {
            self.index += 1;
            let Some(member_token) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    "Expected associated type name after '.'",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if TokenType::Identifier != member_token.ty {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    format!(
                        "Expected associated type name but found '{}'",
                        member_token.value
                    ),
                    &member_token,
                );
                return Err(());
            }
            type_expr = Expression::AssociatedTypeAccess {
                object: Rc::new(RefCell::new(type_expr)),
                member: Box::new(member_token),
                ty: None,
            };
        }

        while let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Multiply)
        {
            self.index += 1;
            let mut non_null = false;
            if let Some(next) = self.peek()
                && OperatorType::is_operator(&next, OperatorType::Not)
            {
                self.index += 1;
                non_null = true;
            }
            type_expr = Expression::PointerType {
                base: Box::new(Rc::new(RefCell::new(type_expr))),
                non_null,
                ty: None,
            };
        }

        if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::QuestionMark)
        {
            self.index += 1;
            type_expr = Expression::OptionalType {
                inner: Rc::new(RefCell::new(type_expr)),
                ty: None,
            };
        }

        let mut types = vec![Rc::new(RefCell::new(type_expr))];
        while let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::BitAnd)
        {
            self.index += 1;
            let right = self.parse_type_expression()?;
            if let Expression::CompoundType {
                types: inner_types, ..
            } = right
            {
                types.extend(inner_types);
            } else {
                types.push(Rc::new(RefCell::new(right)));
            }
        }

        if types.len() > 1 {
            Ok(Expression::CompoundType { types, ty: None })
        } else {
            Ok(Rc::try_unwrap(types.into_iter().next().unwrap())
                .ok()
                .unwrap()
                .into_inner())
        }
    }

    fn parse_failable_init_qualifier(&mut self) -> bool {
        self.peek().is_some_and(|t| {
            if OperatorType::is_operator(&t, OperatorType::QuestionMark) {
                self.index += 1;
                true
            } else {
                false
            }
        })
    }

    fn parse_function_decl(
        &mut self,
        is_extern: bool,
        attributes: Vec<Attribute>,
        modifiers: Vec<Modifier>,
    ) -> Result<Statement, ()> {
        let Some(token) = self.next() else {
            return Err(());
        };
        let is_init = KeywordType::is_keyword(&token, KeywordType::Init);
        let is_failable = is_init && self.parse_failable_init_qualifier();
        let (name, generic_parameters) = if !is_init {
            let Some(name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::InvalidFunctionName,
                    "Expected function name after 'func'",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !matches!(name.ty, TokenType::Identifier | TokenType::Operator { .. }) {
                self.emit_error(
                    TrussDiagnosticCode::InvalidFunctionName,
                    format!("Expected function name but found '{}'", name.value),
                    &name,
                );
                return Err(());
            }
            let generic_parameters = self.parse_generic_parameters()?.unwrap_or_default();
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
            (Some(name), generic_parameters)
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
            (None, vec![])
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
                    default_value: None,
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
            let default_value = if variadic_kind == VariadicKind::NotVariadic
                && let Some(t) = self.peek()
                && let TokenType::Operator { .. } = t.ty
                && OperatorType::is_operator(&t, OperatorType::Assign)
            {
                self.index += 1;
                Some(Rc::new(RefCell::new(self.parse_expression()?)))
            } else {
                None
            };
            parameters.push(Rc::new(RefCell::new(Parameter {
                label: label_token.map(Box::new),
                name: Box::new(name_token),
                type_expression: Rc::new(RefCell::new(type_expression)),
                ty: None,
                variadic_kind,
                default_value,
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
        let throws_types = if let Some(t) = self.peek()
            && KeywordType::is_keyword(&t, KeywordType::Throws)
        {
            self.index += 1;
            if is_init {
                self.emit_error(
                    TrussDiagnosticCode::InvalidFunctionName,
                    "'init' cannot be declared throws".to_string(),
                    &t,
                );
                return Err(());
            }
            if let Some(open_paren) = self.peek()
                && SeparatorType::is_separator(&open_paren, SeparatorType::OpenParen)
            {
                self.index += 1;
                let mut types = Vec::new();
                loop {
                    let ty = self.parse_type_expression()?;
                    types.push(Rc::new(RefCell::new(ty)));
                    if let Some(comma) = self.peek()
                        && SeparatorType::is_separator(&comma, SeparatorType::Comma)
                    {
                        self.index += 1;
                        continue;
                    }
                    break;
                }
                let Some(close_paren) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected ')' after throws types".to_string(),
                        &t,
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
                Some(types)
            } else {
                Some(vec![])
            }
        } else {
            None
        };
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

        let where_clause = self.parse_where_clause()?;

        let body = if is_extern {
            FunctionBody::None
        } else {
            if let Some(token) = self.peek()
                && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
            {
                self.index += 1;
                let mut statements = Vec::new();
                self.scope_nesting += 1;
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
                    self.scope_nesting -= 1;
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected '}' to close function body",
                        &self.tokens[self.index.saturating_sub(1)],
                    );
                    return Err(());
                };
                if !SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                    self.scope_nesting -= 1;
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        format!("Expected '}}' but found '{}'", next.value),
                        &next,
                    );
                    return Err(());
                }
                self.scope_nesting -= 1;
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
                is_failable,
                scope: None,
                ty: None,
            })
        } else {
            let static_method = modifiers.iter().any(|m| m.ty == ModifierType::Static);
            let operator_fixity = modifiers.iter().find_map(|m| {
                if let ModifierType::OperatorFixity(fixity) = &m.ty {
                    Some(*fixity)
                } else {
                    None
                }
            });
            let mutating = modifiers.iter().any(|m| m.ty == ModifierType::Mutating);
            Ok(Statement::FunctionDecl {
                attributes,
                modifiers,
                token: Box::new(token),
                name: Box::new(name.unwrap()),
                generic_parameters,
                parameters,
                return_type: return_type.map(RefCell::new).map(Rc::new),
                throws_types,
                body: Rc::new(RefCell::new(body)),
                where_clause,
                scope: None,
                ty: None,
                static_method,
                mutating,
                operator_fixity,
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
            self.scope_nesting += 1;
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
                self.scope_nesting -= 1;
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close deinit body",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&next, SeparatorType::CloseBrace) {
                self.scope_nesting -= 1;
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", next.value),
                    &next,
                );
                return Err(());
            }
            self.scope_nesting -= 1;
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

    fn parse_subscript_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let generic_parameters = self.parse_generic_parameters()?.unwrap_or_default();
        let Some(open) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '(' after 'subscript'",
                &token,
            );
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
                    default_value: None,
                })));
                if has_variadic {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Variadic parameter must be the last parameter and only one is allowed",
                        &t,
                    );
                } else {
                    has_variadic = true;
                }
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
                }
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
                VariadicKind::TypedVariadic
            } else {
                VariadicKind::NotVariadic
            };
            let default_value = if variadic_kind == VariadicKind::NotVariadic
                && let Some(t) = self.peek()
                && let TokenType::Operator { .. } = t.ty
                && OperatorType::is_operator(&t, OperatorType::Assign)
            {
                self.index += 1;
                Some(Rc::new(RefCell::new(self.parse_expression()?)))
            } else {
                None
            };
            parameters.push(Rc::new(RefCell::new(Parameter {
                label: label_token.map(Box::new),
                name: Box::new(name_token),
                type_expression: Rc::new(RefCell::new(type_expression)),
                ty: None,
                variadic_kind,
                default_value,
            })));
            let Some(t) = self.peek() else { break };
            if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                self.index += 1;
            } else {
                break;
            }
        }
        let Some(close) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected ')' to close subscript parameter list",
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
        let return_type_expression = if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Arrow)
        {
            self.index += 1;
            self.parse_type_expression()?
        } else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedType,
                "Expected return type after '->' in subscript declaration",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        let where_clause = self.parse_where_clause()?;
        let accessors = if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            self.index += 1;
            self.parse_accessor_body()?
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' with accessor body in subscript declaration",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !accessors
            .iter()
            .any(|a| matches!(a.kind, AccessorKind::Get))
        {
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                "Subscript must have at least a get accessor",
                &token,
            );
            return Err(());
        }
        Ok(Statement::SubscriptDecl {
            modifiers,
            token: Box::new(token),
            generic_parameters,
            parameters,
            return_type_expression: Rc::new(RefCell::new(return_type_expression)),
            where_clause,
            accessors,
            scope: None,
            ty: None,
        })
    }

    fn parse_macro_decl(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                "Expected macro name after 'macro'",
                &token,
            );
            return Err(());
        };
        if !matches!(name.ty, TokenType::Identifier) {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                format!("Expected macro name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let Some(open_brace) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after macro name",
                &name,
            );
            return Err(());
        };
        if !SeparatorType::is_separator(&open_brace, SeparatorType::OpenBrace) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '{{' but found '{}'", open_brace.value),
                &open_brace,
            );
            return Err(());
        }
        let mut arms = Vec::new();
        while let Some(t) = self.peek() {
            if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                break;
            }
            let arm = self.parse_macro_arm()?;
            arms.push(arm);
        }
        let Some(close_brace) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '}' to close macro body",
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
        Ok(Statement::MacroDecl {
            token: Box::new(token),
            name: Box::new(name),
            arms,
        })
    }

    fn parse_macro_arm(&mut self) -> Result<MacroArm, ()> {
        let Some(open) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '(' to start macro pattern",
                &self.tokens[self.index.saturating_sub(1)],
            );
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
        let pattern = self.parse_macro_pattern()?;
        let Some(close) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected ')' to close macro pattern",
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
        let Some(arrow) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '=>' after macro pattern",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !OperatorType::is_operator(&arrow, OperatorType::Assign)
            || !self.peek().map_or(false, |t| {
                OperatorType::is_operator(&t, OperatorType::Greater)
            })
        {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '=>' but found '{}'", arrow.value),
                &arrow,
            );
            return Err(());
        }
        self.index += 1;
        let expansion = self.parse_macro_expansion()?;
        Ok(MacroArm { pattern, expansion })
    }

    fn parse_macro_pattern(&mut self) -> Result<Vec<MacroPatternFragment>, ()> {
        let mut fragments = Vec::new();
        while let Some(t) = self.peek() {
            if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                break;
            }
            if OperatorType::is_operator(&t, OperatorType::Dollar) {
                self.index += 1;
                let Some(name) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        "Expected metavariable name after '$'",
                        &t,
                    );
                    return Err(());
                };
                if !matches!(name.ty, TokenType::Identifier) {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        format!("Expected identifier but found '{}'", name.value),
                        &name,
                    );
                    return Err(());
                }
                let Some(colon) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected ':' after metavariable name",
                        &name,
                    );
                    return Err(());
                };
                if !SeparatorType::is_separator(&colon, SeparatorType::Colon) {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        format!(
                            "Expected ':' after metavariable name but found '{}'",
                            colon.value
                        ),
                        &colon,
                    );
                    return Err(());
                }
                let Some(type_tok) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedType,
                        "Expected metavariable type after ':'",
                        &colon,
                    );
                    return Err(());
                };
                let var_type = match type_tok.value.as_str() {
                    "expr" => MacroMetaVarType::Expr,
                    "ty" => MacroMetaVarType::Ty,
                    "ident" => MacroMetaVarType::Ident,
                    "stmt" => MacroMetaVarType::Stmt,
                    "block" => MacroMetaVarType::Block,
                    "literal" => MacroMetaVarType::Literal,
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedType,
                            format!("Unknown metavariable type '{}'. Expected one of: expr, ty, ident, stmt, block, literal", type_tok.value),
                            &type_tok,
                        );
                        return Err(());
                    }
                };
                fragments.push(MacroPatternFragment::MetaVar {
                    name: name.value.clone(),
                    var_type,
                });
            } else if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                fragments.push(MacroPatternFragment::Lit(t.clone()));
                self.index += 1;
            } else {
                fragments.push(MacroPatternFragment::Lit(t.clone()));
                self.index += 1;
            }
        }
        Ok(fragments)
    }

    fn parse_macro_expansion(&mut self) -> Result<Vec<Token>, ()> {
        let Some(open) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected delimiter for macro expansion",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        let close_delim = if SeparatorType::is_separator(&open, SeparatorType::OpenBrace) {
            SeparatorType::CloseBrace
        } else if SeparatorType::is_separator(&open, SeparatorType::OpenParen) {
            SeparatorType::CloseParen
        } else if SeparatorType::is_separator(&open, SeparatorType::OpenBracket) {
            SeparatorType::CloseBracket
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '(' '[' or '{{' but found '{}'", open.value),
                &open,
            );
            return Err(());
        };
        let mut depth = 1u32;
        let mut tokens = Vec::new();
        while let Some(ref t) = self.peek() {
            if SeparatorType::is_separator(t, close_delim) && depth == 1 {
                self.index += 1;
                break;
            }
            if SeparatorType::is_separator(t, SeparatorType::OpenBrace)
                || SeparatorType::is_separator(t, SeparatorType::OpenParen)
                || SeparatorType::is_separator(t, SeparatorType::OpenBracket)
            {
                depth += 1;
            } else if SeparatorType::is_separator(t, SeparatorType::CloseBrace)
                || SeparatorType::is_separator(t, SeparatorType::CloseParen)
                || SeparatorType::is_separator(t, SeparatorType::CloseBracket)
            {
                depth -= 1;
                if depth == 0 {
                    self.index += 1;
                    break;
                }
            }
            tokens.push(t.clone());
            self.index += 1;
        }
        Ok(tokens)
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

    fn parse_yield(&mut self) -> Result<Statement, ()> {
        let Some(token) = self.peek() else {
            return Err(());
        };
        let current_line = token.position.line;
        self.index += 1;
        let yield_token = token;
        let value = if let Some(token) = self.peek()
            && current_line == token.position.line
            && !SeparatorType::is_separator(&token, SeparatorType::CloseBrace)
            && !SeparatorType::is_separator(&token, SeparatorType::SemiColon)
        {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::Yield {
            token: Box::new(yield_token),
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
                body,
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
        let saved_suppress = self.suppress_trailing_closure;
        self.suppress_trailing_closure = true;
        let condition = if let Some(t) = self.peek()
            && KeywordType::is_keyword(&t, KeywordType::Let)
        {
            let cond = self.parse_if_let_condition()?;
            self.suppress_trailing_closure = saved_suppress;
            cond
        } else {
            let cond = self.parse_expression()?;
            self.suppress_trailing_closure = saved_suppress;
            cond
        };
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::While {
                token: Box::new(token),
                condition: Rc::new(RefCell::new(condition)),
                body,
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
                body,
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
        let iterator = {
            let saved_suppress = self.suppress_trailing_closure;
            self.suppress_trailing_closure = true;
            let iter = self.parse_expression()?;
            self.suppress_trailing_closure = saved_suppress;
            iter
        };
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            Ok(Statement::For {
                token: Box::new(token),
                pattern: Rc::new(pattern),
                iterator: Rc::new(RefCell::new(iterator)),
                body,
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

    fn parse_extern(
        &mut self,
        _attributes: Vec<Attribute>,
        modifiers: Vec<Modifier>,
    ) -> Result<Statement, ()> {
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
        let attrs = self.parse_attributes()?;
        let Some(token) = self.peek() else {
            return Err(());
        };
        match token.ty {
            TokenType::Keyword { keyword } => match keyword {
                KeywordType::Func => self.parse_function_decl(true, attrs, modifiers),
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
        let pattern = self.parse_pattern()?;
        let name = Self::extract_first_name(&pattern)
            .cloned()
            .unwrap_or(Token::new(
                "_".to_string(),
                TokenType::Identifier,
                token.position.clone(),
                token.file.clone(),
            ));
        let pattern = Some(pattern);
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
            let init_expr = self.parse_expression()?;
            Some(self.apply_trailing_closure(init_expr)?)
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
        let ownership = Self::extract_ownership(&modifiers);
        Ok(Statement::VariableDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            pattern,
            type_expression: type_expression.map(RefCell::new).map(Rc::new),
            initializer: initializer.map(RefCell::new).map(Rc::new),
            accessors,
            ownership,
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
        } else if let TokenType::Keyword { keyword } = &first.ty {
            matches!(
                keyword,
                KeywordType::Open
                    | KeywordType::Public
                    | KeywordType::Internal
                    | KeywordType::Fileprivate
                    | KeywordType::Private
                    | KeywordType::Package
            ) && self.index + 2 < self.tokens.len()
                && self.tokens[self.index + 1].value == "set"
                && self.tokens[self.index + 1].ty == TokenType::Identifier
                && (SeparatorType::is_separator(
                    &self.tokens[self.index + 2],
                    SeparatorType::OpenBrace,
                ) || SeparatorType::is_separator(
                    &self.tokens[self.index + 2],
                    SeparatorType::OpenParen,
                ))
        } else {
            false
        };
        if is_accessor_block {
            self.parse_accessors()
        } else {
            let mut body = Vec::new();
            self.scope_nesting += 1;
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
                self.scope_nesting -= 1;
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close getter body".to_string(),
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseBrace) {
                self.scope_nesting -= 1;
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }
            self.scope_nesting -= 1;
            Ok(vec![Accessor {
                kind: AccessorKind::Get,
                parameter: None,
                body,
                set_access_modifier: None,
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
            let mut set_access_modifier = None;
            if let TokenType::Keyword { keyword } = &token.ty {
                match keyword {
                    KeywordType::Open
                    | KeywordType::Public
                    | KeywordType::Internal
                    | KeywordType::Fileprivate
                    | KeywordType::Private
                    | KeywordType::Package => {
                        if let Some(next) = self.peek2()
                            && next.value == "set"
                            && next.ty == TokenType::Identifier
                        {
                            let modifier = match keyword {
                                KeywordType::Open => AccessModifier::Open,
                                KeywordType::Public => AccessModifier::Public,
                                KeywordType::Internal => AccessModifier::Internal,
                                KeywordType::Fileprivate => AccessModifier::Fileprivate,
                                KeywordType::Private => AccessModifier::Private,
                                KeywordType::Package => AccessModifier::Package,
                                _ => unreachable!(),
                            };
                            self.index += 1;
                            set_access_modifier = Some(modifier);
                        }
                    }
                    _ => {}
                }
            }
            let Some(token) = self.peek() else {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    "Expected accessor or '}'".to_string(),
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
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
            self.scope_nesting += 1;
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
                self.scope_nesting -= 1;
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '}' to close accessor body".to_string(),
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseBrace) {
                self.scope_nesting -= 1;
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '}}' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }
            self.scope_nesting -= 1;
            accessors.push(Accessor {
                kind,
                parameter,
                body,
                set_access_modifier,
            });
        }
        let has_computed = accessors
            .iter()
            .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
        let has_willset_didset = accessors
            .iter()
            .any(|a| matches!(a.kind, AccessorKind::WillSet | AccessorKind::DidSet));
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
        let has_init = body
            .iter()
            .any(|stmt| matches!(&*stmt.borrow(), Statement::InitDecl { .. }));
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
                    default_value: None,
                }));
                parameters.push(param);
            }
        }
        let init_token = Box::new(Token::new(
            "init".to_string(),
            TokenType::Keyword {
                keyword: KeywordType::Init,
            },
            type_name.position.clone(),
            type_name.file.clone(),
        ));
        let init_decl = Statement::InitDecl {
            modifiers: vec![],
            token: init_token,
            parameters,
            body: Rc::new(RefCell::new(FunctionBody::None)),
            is_failable: false,
            scope: None,
            ty: None,
        };
        body.push(Rc::new(RefCell::new(init_decl)));
    }

    fn parse_struct_decl(
        &mut self,
        attributes: Vec<Attribute>,
        modifiers: Vec<Modifier>,
    ) -> Result<Statement, ()> {
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
        let generic_parameters = self.parse_generic_parameters()?.unwrap_or_default();
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
        let where_clause = self.parse_where_clause()?;
        self.scope_nesting += 1;
        let mut body = self.parse_brace_body()?;
        self.scope_nesting -= 1;
        self.ensure_memberwise_init(&mut body, &name);
        Ok(Statement::StructDecl {
            attributes,
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            generic_parameters,
            conformances,
            body,
            where_clause,
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
        let generic_parameters = self.parse_generic_parameters()?.unwrap_or_default();
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
        let where_clause = self.parse_where_clause()?;
        self.scope_nesting += 1;
        let body = self.parse_brace_body()?;
        self.scope_nesting -= 1;
        Ok(Statement::ClassDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            generic_parameters,
            superclass,
            conformances,
            body,
            where_clause,
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

    fn parse_extension_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        if !modifiers.is_empty() {
            self.emit_error(
                TrussDiagnosticCode::ModifierNotAllowedHere,
                "Modifiers are not allowed on 'extension' declaration",
                &modifiers[0].token,
            );
        }
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(type_name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                "Expected type name after 'extension'",
                &token,
            );
            return Err(());
        };
        if TokenType::Identifier != type_name.ty
            && !matches!(
                type_name.ty,
                TokenType::Keyword {
                    keyword: KeywordType::SelfType
                }
            )
        {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                format!(
                    "Expected type name or 'Self' but found '{}'",
                    type_name.value
                ),
                &type_name,
            );
            return Err(());
        }
        let type_arguments = self.parse_type_parameters()?;
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
        let where_clause = self.parse_where_clause()?;
        self.scope_nesting += 1;
        let body = self.parse_brace_body()?;
        self.scope_nesting -= 1;
        Ok(Statement::ExtensionDecl {
            token: Box::new(token),
            type_name: Box::new(type_name),
            type_arguments,
            conformances,
            body,
            where_clause,
            scope: None,
        })
    }

    fn parse_typealias(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        if !modifiers.is_empty() {
            self.emit_error(
                TrussDiagnosticCode::ModifierNotAllowedHere,
                "Modifiers are not allowed on 'typealias' declaration",
                &modifiers[0].token,
            );
        }
        let Some(token) = self.next() else {
            return Err(());
        };
        let Some(name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                "Expected alias name after 'typealias'",
                &token,
            );
            return Err(());
        };
        if TokenType::Identifier != name.ty {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                format!("Expected alias name but found '{}'", name.value),
                &name,
            );
            return Err(());
        }
        let Some(assign) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '=' after typealias name",
                &name,
            );
            return Err(());
        };
        if !OperatorType::is_operator(&assign, OperatorType::Assign) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '=' but found '{}'", assign.value),
                &assign,
            );
            return Err(());
        }
        let type_expression = self.parse_type_expression()?;
        Ok(Statement::TypeAlias {
            token: Box::new(token),
            name: Box::new(name),
            type_expression: Rc::new(RefCell::new(type_expression)),
        })
    }

    fn parse_module_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let Some(first_name) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                "Expected module name after 'module'",
                &token,
            );
            return Err(());
        };
        if !matches!(first_name.ty, TokenType::Identifier) {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                format!("Expected module name but found '{}'", first_name.value),
                &first_name,
            );
            return Err(());
        }

        let mut path_segments = vec![first_name];
        while let Some(dot) = self.peek() {
            if !OperatorType::is_operator(&dot, OperatorType::Dot) {
                break;
            }
            self.index += 1;
            let Some(name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    "Expected module name after '.'",
                    &dot,
                );
                return Err(());
            };
            if !matches!(name.ty, TokenType::Identifier) {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    format!("Expected module name but found '{}'", name.value),
                    &name,
                );
                return Err(());
            }
            path_segments.push(name);
        }

        let body = self.parse_brace_body()?;

        if path_segments.len() == 1 {
            Ok(Statement::ModuleDecl {
                modifiers,
                token: Box::new(token),
                name: Box::new(path_segments.into_iter().next().unwrap()),
                body,
                scope: None,
            })
        } else {
            let mut inner = Statement::ModuleDecl {
                modifiers: vec![],
                token: Box::new(token.clone()),
                name: Box::new(path_segments.pop().unwrap()),
                body,
                scope: None,
            };
            while let Some(segment) = path_segments.pop() {
                inner = Statement::ModuleDecl {
                    modifiers: vec![],
                    token: Box::new(token.clone()),
                    name: Box::new(segment),
                    body: vec![Rc::new(RefCell::new(inner))],
                    scope: None,
                };
            }
            Ok(inner)
        }
    }

    fn parse_selective_members(&mut self) -> Result<Vec<SelectiveMember>, ()> {
        let mut members = Vec::new();
        let mut first = true;
        loop {
            let Some(name_token) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    "Expected member name or '}' in selective import list",
                    &Token::new(
                        String::new(),
                        TokenType::Identifier,
                        Position {
                            pos: 0,
                            line: 0,
                            col: 0,
                            len: 0,
                        },
                        Rc::new(String::new()),
                    ),
                );
                return Err(());
            };
            if SeparatorType::is_separator(&name_token, SeparatorType::CloseBrace) {
                if first {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected at least one member in selective import list",
                        &name_token,
                    );
                    return Err(());
                }
                break;
            }
            first = false;
            if !matches!(name_token.ty, TokenType::Identifier) {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    format!("Expected identifier but found '{}'", name_token.value),
                    &name_token,
                );
                return Err(());
            }
            let name = name_token.value.clone();
            let alias = if let Some(ref as_kw) = self.peek() {
                if KeywordType::is_keyword(as_kw, KeywordType::As) {
                    self.index += 1;
                    let Some(alias_token) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            "Expected alias name after 'as'",
                            &name_token,
                        );
                        return Err(());
                    };
                    if !matches!(alias_token.ty, TokenType::Identifier) {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            format!("Expected identifier but found '{}'", alias_token.value),
                            &alias_token,
                        );
                        return Err(());
                    }
                    if alias_token.value == "_" {
                        SelectiveAlias::Skip
                    } else {
                        SelectiveAlias::Named(alias_token.value)
                    }
                } else {
                    SelectiveAlias::Direct
                }
            } else {
                SelectiveAlias::Direct
            };
            members.push(SelectiveMember { name, alias });
            match self.peek() {
                Some(ref comma) if SeparatorType::is_separator(comma, SeparatorType::Comma) => {
                    self.index += 1;
                }
                Some(ref brace)
                    if SeparatorType::is_separator(brace, SeparatorType::CloseBrace) =>
                {
                    self.index += 1;
                    break;
                }
                Some(other) => {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        format!("Expected ',' or '}}' but found '{}'", other.value),
                        &other,
                    );
                    return Err(());
                }
                None => {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected ',' or '}' but reached end of input",
                        &Token::new(
                            String::new(),
                            TokenType::Identifier,
                            Position {
                                pos: 0,
                                line: 0,
                                col: 0,
                                len: 0,
                            },
                            Rc::new(String::new()),
                        ),
                    );
                    return Err(());
                }
            }
        }
        Ok(members)
    }

    fn parse_import(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        if self.scope_nesting > 0 {
            self.emit_error(
                TrussDiagnosticCode::ParserError,
                "'import' declarations are only allowed at file scope",
                &token,
            );
            return Err(());
        }
        let Some(first) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                "Expected module or member name after 'import'",
                &token,
            );
            return Err(());
        };
        let mut path: Vec<String> = Vec::new();
        let mut is_current_package = false;
        if KeywordType::is_keyword(&first, KeywordType::Package) {
            match self.peek() {
                Some(ref dot) if OperatorType::is_operator(dot, OperatorType::Dot) => {
                    self.index += 1;
                    let Some(name) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            "Expected module or member name after 'package.'",
                            &first,
                        );
                        return Err(());
                    };
                    if !matches!(name.ty, TokenType::Identifier) {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            format!(
                                "Expected identifier after 'package.' but found '{}'",
                                name.value
                            ),
                            &name,
                        );
                        return Err(());
                    }
                    is_current_package = true;
                    path.push(name.value);
                }
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        format!("Expected identifier but found '{}'", first.value),
                        &first,
                    );
                    return Err(());
                }
            }
        } else if !matches!(first.ty, TokenType::Identifier) {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                format!("Expected identifier but found '{}'", first.value),
                &first,
            );
            return Err(());
        } else {
            path.push(first.value.clone());
        }
        let mut wildcard = false;
        loop {
            match self.peek() {
                Some(ref dot) if OperatorType::is_operator(dot, OperatorType::Dot) => {
                    if let Some(next) = self.peek2() {
                        if SeparatorType::is_separator(&next, SeparatorType::OpenBrace) {
                            break;
                        }
                    }
                    self.index += 1;
                    if let Some(ref star) = self.peek() {
                        if let TokenType::Operator {
                            operator: OperatorType::Multiply,
                        } = star.ty
                        {
                            self.index += 1;
                            wildcard = true;
                            break;
                        }
                    }
                    let Some(name) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            "Expected identifier after '.'",
                            &first,
                        );
                        return Err(());
                    };
                    if !matches!(name.ty, TokenType::Identifier) {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            format!("Expected identifier but found '{}'", name.value),
                            &name,
                        );
                        return Err(());
                    }
                    path.push(name.value);
                }
                _ => break,
            }
        }
        let (selective_members, kind) = if let Some(ref brace_or_dot) = self.peek() {
            if SeparatorType::is_separator(brace_or_dot, SeparatorType::OpenBrace) {
                self.index += 1;
                let members = self.parse_selective_members()?;
                (Some(members), ImportKind::Module)
            } else if OperatorType::is_operator(brace_or_dot, OperatorType::Dot) {
                if let Some(ref brace) = self.peek2() {
                    if SeparatorType::is_separator(brace, SeparatorType::OpenBrace) {
                        self.index += 1;
                        self.index += 1;
                        let members = self.parse_selective_members()?;
                        (Some(members), ImportKind::Module)
                    } else {
                        let kind = if wildcard {
                            ImportKind::Wildcard
                        } else if path.len() >= 3 {
                            ImportKind::Member
                        } else {
                            ImportKind::Module
                        };
                        (None, kind)
                    }
                } else {
                    let kind = if wildcard {
                        ImportKind::Wildcard
                    } else if path.len() >= 3 {
                        ImportKind::Member
                    } else {
                        ImportKind::Module
                    };
                    (None, kind)
                }
            } else if KeywordType::is_keyword(brace_or_dot, KeywordType::As) {
                self.index += 1;
                if path.len() < 2 {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "'as' requires a dotted path with at least a module and a member name",
                        brace_or_dot,
                    );
                    return Err(());
                }
                let member_name = path.pop().unwrap();
                let Some(alias_token) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        "Expected alias name after 'as'",
                        brace_or_dot,
                    );
                    return Err(());
                };
                let alias = if matches!(alias_token.ty, TokenType::Identifier) {
                    if alias_token.value == "_" {
                        SelectiveAlias::Skip
                    } else {
                        SelectiveAlias::Named(alias_token.value)
                    }
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::ExpectedIdentifier,
                        format!("Expected identifier but found '{}'", alias_token.value),
                        &alias_token,
                    );
                    return Err(());
                };
                let member = SelectiveMember {
                    name: member_name,
                    alias,
                };
                (Some(vec![member]), ImportKind::Module)
            } else {
                let kind = if wildcard {
                    ImportKind::Wildcard
                } else if path.len() >= 3 {
                    ImportKind::Member
                } else {
                    ImportKind::Module
                };
                (None, kind)
            }
        } else {
            let kind = if wildcard {
                ImportKind::Wildcard
            } else if path.len() >= 3 {
                ImportKind::Member
            } else {
                ImportKind::Module
            };
            (None, kind)
        };
        Ok(Statement::ImportDecl {
            token: Box::new(token),
            path,
            kind,
            selective_members,
            is_current_package,
        })
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
        let generic_parameters = self.parse_generic_parameters()?.unwrap_or_default();
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
        let where_clause = self.parse_where_clause()?;
        let mut cases = Vec::new();
        let mut body = Vec::new();
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            self.index += 1;
            self.scope_nesting += 1;
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
                            if !SeparatorType::is_separator(&close_paren, SeparatorType::CloseParen)
                            {
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
            self.scope_nesting -= 1;
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
            generic_parameters,
            conformances,
            raw_value_type: None,
            cases,
            body,
            where_clause,
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
        let generic_params = self.parse_generic_parameters()?.unwrap_or_default();
        let mut associated_members: Vec<ProtocolMember> = Vec::new();
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
        let where_clause = self.parse_where_clause()?;
        let mut members = Vec::new();
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            self.index += 1;
            while let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    break;
                }
                let member_attributes = self.parse_attributes()?;
                let member_modifiers = self.parse_modifiers()?;
                let Some(peek_token) = self.peek() else { break };
                match peek_token.ty {
                    TokenType::Keyword { keyword } if keyword == KeywordType::Func => {
                        let func_decl = self.parse_function_decl(
                            false,
                            member_attributes.clone(),
                            member_modifiers,
                        )?;
                        if let Statement::FunctionDecl { .. } = &func_decl {
                            members.push(ProtocolMember::Method {
                                attributes: member_attributes,
                                modifiers: vec![],
                                decl: Rc::new(RefCell::new(func_decl)),
                            });
                        }
                    }
                    TokenType::Keyword { keyword } if keyword == KeywordType::Associatedtype => {
                        self.index += 1;
                        let Some(assoc_name) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::InvalidVariableName,
                                "Expected associated type name",
                                &peek_token,
                            );
                            return Err(());
                        };
                        if TokenType::Identifier != assoc_name.ty {
                            self.emit_error(
                                TrussDiagnosticCode::InvalidVariableName,
                                format!(
                                    "Expected associated type name but found '{}'",
                                    assoc_name.value
                                ),
                                &assoc_name,
                            );
                            return Err(());
                        }
                        let mut constraints = Vec::new();
                        if let Some(t) = self.peek()
                            && SeparatorType::is_separator(&t, SeparatorType::Colon)
                        {
                            self.index += 1;
                            constraints.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
                            while let Some(t) = self.peek()
                                && OperatorType::is_operator(&t, OperatorType::BitAnd)
                            {
                                self.index += 1;
                                constraints
                                    .push(Rc::new(RefCell::new(self.parse_type_expression()?)));
                            }
                        }
                        members.push(ProtocolMember::AssociatedType {
                            token: Box::new(assoc_name.clone()),
                            name: Box::new(assoc_name),
                            constraints,
                        });
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
                                format!("Expected property name but found '{}'", prop_name.value),
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
                    TokenType::Keyword { keyword } if keyword == KeywordType::Typealias => {
                        let token = self.next().unwrap();
                        let Some(name) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::ExpectedIdentifier,
                                "Expected alias name after 'typealias'",
                                &token,
                            );
                            return Err(());
                        };
                        if TokenType::Identifier != name.ty {
                            self.emit_error(
                                TrussDiagnosticCode::ExpectedIdentifier,
                                format!("Expected alias name but found '{}'", name.value),
                                &name,
                            );
                            return Err(());
                        }
                        let Some(assign) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                "Expected '=' after typealias name",
                                &name,
                            );
                            return Err(());
                        };
                        if !OperatorType::is_operator(&assign, OperatorType::Assign) {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                format!("Expected '=' but found '{}'", assign.value),
                                &assign,
                            );
                            return Err(());
                        }
                        let type_expression = self.parse_type_expression()?;
                        members.push(ProtocolMember::TypeAlias {
                            token: Box::new(token),
                            name: Box::new(name),
                            type_expression: Rc::new(RefCell::new(type_expression)),
                        });
                    }
                    TokenType::Keyword { keyword } if keyword == KeywordType::Subscript => {
                        let sub_token = self.next().unwrap();
                        let generic_parameters =
                            self.parse_generic_parameters()?.unwrap_or_default();
                        let Some(open) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                "Expected '(' after 'subscript'",
                                &sub_token,
                            );
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
                        while let Some(t) = self.peek() {
                            if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
                                break;
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
                                        format!(
                                            "Expected parameter name but found '{}'",
                                            second.value
                                        ),
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
                            parameters.push(Rc::new(RefCell::new(Parameter {
                                label: label_token.map(Box::new),
                                name: Box::new(name_token),
                                type_expression: Rc::new(RefCell::new(type_expression)),
                                ty: None,
                                variadic_kind: VariadicKind::NotVariadic,
                                default_value: None,
                            })));
                            let Some(t) = self.peek() else { break };
                            if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                                self.index += 1;
                            } else {
                                break;
                            }
                        }
                        let Some(close) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                "Expected ')' to close subscript parameter list",
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
                        let Some(arrow) = self.next() else {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                "Expected '->' after subscript parameters",
                                &self.tokens[self.index.saturating_sub(1)],
                            );
                            return Err(());
                        };
                        if !OperatorType::is_operator(&arrow, OperatorType::Arrow) {
                            self.emit_error(
                                TrussDiagnosticCode::MissingSeparator,
                                format!("Expected '->' but found '{}'", arrow.value),
                                &arrow,
                            );
                            return Err(());
                        }
                        let return_type_expression = self.parse_type_expression()?;
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
                                                format!("Expected 'get' or 'set' in protocol subscript accessor, found '{}'", t.value),
                                                &t,
                                            );
                                            return Err(());
                                        }
                                    }
                                } else {
                                    self.emit_error(
                                        TrussDiagnosticCode::UnexpectedToken,
                                        format!("Expected 'get' or 'set' in protocol subscript accessor, found '{}'", t.value),
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
                        members.push(ProtocolMember::Subscript {
                            modifiers: member_modifiers,
                            token: Box::new(sub_token),
                            generic_parameters,
                            parameters,
                            return_type_expression: Rc::new(RefCell::new(return_type_expression)),
                            accessors: ProtocolAccessorSet { get, set },
                        });
                    }
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            format!(
                                "Expected 'func', 'associatedtype', 'typealias', 'let'/'var', or 'subscript' in protocol body, found '{}'",
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
        associated_members.append(&mut members);
        Ok(Statement::ProtocolDecl {
            modifiers,
            token: Box::new(token),
            name: Box::new(name),
            generic_parameters: generic_params,
            conformances,
            members: associated_members,
            where_clause,
            scope: None,
            ty: None,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Rc<RefCell<Statement>>>, ()> {
        self.index += 1;
        let mut statements = Vec::new();
        self.scope_nesting += 1;
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
            self.scope_nesting -= 1;
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '}' to close block",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        self.scope_nesting -= 1;
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

    fn parse_closure_capture_list(&mut self) -> Result<Vec<ClosureCapture>, ()> {
        let mut captures = Vec::new();
        loop {
            if let Some(token) = self.peek()
                && SeparatorType::is_separator(&token, SeparatorType::CloseBracket)
            {
                break;
            }

            let ownership = if let Some(token) = self.peek()
                && KeywordType::is_keyword(&token, KeywordType::Weak)
            {
                self.index += 1;
                OwnershipModifier::Weak
            } else if let Some(token) = self.peek()
                && KeywordType::is_keyword(&token, KeywordType::Unowned)
            {
                self.index += 1;
                OwnershipModifier::Unowned
            } else {
                OwnershipModifier::Strong
            };

            let is_var = if let Some(token) = self.peek()
                && KeywordType::is_keyword(&token, KeywordType::Var)
            {
                self.index += 1;
                true
            } else if let Some(token) = self.peek()
                && KeywordType::is_keyword(&token, KeywordType::Let)
            {
                self.index += 1;
                false
            } else {
                false
            };

            let Some(name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedExpression,
                    "Expected capture name",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if TokenType::Identifier != name.ty {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!("Expected capture name but found '{}'", name.value),
                    &name,
                );
                return Err(());
            }

            let expression = if let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::Assign)
            {
                self.index += 1;
                Some(Rc::new(RefCell::new(self.parse_expression()?)))
            } else {
                None
            };

            captures.push(ClosureCapture {
                name: Box::new(name),
                expression,
                is_var,
                ownership,
            });

            if let Some(token) = self.peek()
                && SeparatorType::is_separator(&token, SeparatorType::Comma)
            {
                self.index += 1;
            } else {
                break;
            }
        }
        Ok(captures)
    }

    fn parse_closure_expression(&mut self) -> Result<Expression, ()> {
        self.index += 1;
        let captures: Vec<ClosureCapture>;
        let parameters: Vec<Rc<RefCell<ClosureParameter>>>;
        let return_type: Option<Rc<RefCell<Expression>>>;

        if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenBracket)
        {
            self.index += 1;
            captures = self.parse_closure_capture_list()?;
            let Some(close) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected ']' to close capture list",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseBracket) {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!("Expected ']' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }
        } else {
            captures = Vec::new();
        }

        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::In)
        {
            parameters = Vec::new();
            return_type = None;
            self.index += 1;
        } else if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Dollar)
        {
            parameters = Vec::new();
            return_type = None;
        } else if let Some(token) = self.peek()
            && SeparatorType::is_separator(&token, SeparatorType::OpenParen)
        {
            let Some(open) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '(' for closure parameters",
                    &self.tokens[self.index.saturating_sub(1)],
                );
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

            let mut params = Vec::new();
            if let Some(token) = self.peek()
                && !SeparatorType::is_separator(&token, SeparatorType::CloseParen)
            {
                loop {
                    let Some(name) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedExpression,
                            "Expected parameter name",
                            &self.tokens[self.index.saturating_sub(1)],
                        );
                        return Err(());
                    };
                    if TokenType::Identifier != name.ty {
                        self.emit_error(
                            TrussDiagnosticCode::UnexpectedToken,
                            format!("Expected parameter name but found '{}'", name.value),
                            &name,
                        );
                        return Err(());
                    }

                    let type_annotation = if let Some(token) = self.peek()
                        && SeparatorType::is_separator(&token, SeparatorType::Colon)
                    {
                        self.index += 1;
                        Some(Rc::new(RefCell::new(self.parse_type_expression()?)))
                    } else {
                        None
                    };

                    params.push(Rc::new(RefCell::new(ClosureParameter {
                        name: Box::new(name),
                        type_annotation,
                    })));

                    if let Some(token) = self.peek()
                        && SeparatorType::is_separator(&token, SeparatorType::Comma)
                    {
                        self.index += 1;
                    } else {
                        break;
                    }
                }
            }

            let Some(close) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected ')' to close closure parameters",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseParen) {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!("Expected ')' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }

            parameters = params;

            let ret = if let Some(token) = self.peek()
                && OperatorType::is_operator(&token, OperatorType::Arrow)
            {
                self.index += 1;
                Some(Rc::new(RefCell::new(self.parse_type_expression()?)))
            } else {
                None
            };
            return_type = ret;

            let Some(in_token) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected 'in' in closure expression",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if !KeywordType::is_keyword(&in_token, KeywordType::In) {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!("Expected 'in' but found '{}'", in_token.value),
                    &in_token,
                );
                return Err(());
            }
        } else {
            parameters = Vec::new();
            return_type = None;
        }

        let mut body = Vec::new();
        self.scope_nesting += 1;
        while let Some(token) = self.peek() {
            if SeparatorType::is_separator(&token, SeparatorType::CloseBrace) {
                break;
            }
            if let Ok(stmt) = self.parse_statement() {
                body.push(Rc::new(RefCell::new(stmt)));
            } else {
                self.skip();
            }
        }

        let Some(close) = self.next() else {
            self.scope_nesting -= 1;
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '}' to close closure",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        self.scope_nesting -= 1;
        if SeparatorType::is_separator(&close, SeparatorType::CloseBrace) {
            Ok(Expression::Closure {
                captures,
                parameters,
                return_type,
                body,
                scope: None,
                ty: None,
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '}}' but found '{}'", close.value),
                &close,
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
            if !self.suppress_trailing_closure
                && let Some(token) = self.peek()
                && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
            {
                let closure = self.parse_closure_expression()?;
                parameters.push(CallParameter {
                    label: None,
                    expression: Rc::new(RefCell::new(closure)),
                });
            }
            Ok(Expression::Call {
                callee: Rc::new(RefCell::new(callee)),
                type_parameters,
                parameters,
                overloads: vec![],
                selected_index: None,
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
        let saved_suppress = self.suppress_trailing_closure;
        self.suppress_trailing_closure = true;
        let condition = if let Some(t) = self.peek()
            && KeywordType::is_keyword(&t, KeywordType::Let)
        {
            let cond = self.parse_if_let_condition()?;
            self.suppress_trailing_closure = saved_suppress;
            cond
        } else {
            let cond = self.parse_expression()?;
            self.suppress_trailing_closure = saved_suppress;
            cond
        };
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
                Some(ElseBranch::If(Rc::new(RefCell::new(self.parse_if()?))))
            } else if let Some(token) = self.peek()
                && SeparatorType::is_separator(&token, SeparatorType::OpenBrace)
            {
                Some(ElseBranch::Block(self.parse_block()?))
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
            then,
            else_,
            ty: None,
        })
    }

    fn parse_case_expression(&mut self) -> Result<Expression, ()> {
        let case_token = self.next().unwrap();

        let (enum_type, case_name_token) = if let Some(t) = self.peek()
            && OperatorType::is_operator(&t, OperatorType::Dot)
        {
            self.index += 1;
            let Some(name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    "Expected case name after '.'",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            (None, name)
        } else {
            let Some(type_name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedType,
                    "Expected enum type name or '.' after 'case'",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
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
            let Some(name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    "Expected case name after '.'",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            (Some(Box::new(type_name)), name)
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

        let expression = self.parse_binary(Precedence::And)?;

        Ok(Expression::Case {
            token: Box::new(case_token),
            enum_type,
            case_name: Box::new(case_name_token),
            bindings,
            expression: Rc::new(RefCell::new(expression)),
            ty: None,
        })
    }

    fn parse_if_let_condition(&mut self) -> Result<Expression, ()> {
        let let_token = self.next().unwrap();

        if !KeywordType::is_keyword(&let_token, KeywordType::Let) {
            self.emit_error(
                TrussDiagnosticCode::ParserError,
                "Expected 'let' in if-let condition",
                &let_token,
            );
            return Err(());
        }

        let name = self.next().ok_or_else(|| {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                "Expected variable name after 'let'",
                &self.tokens[self.index.saturating_sub(1)],
            );
        })?;

        if name.ty != TokenType::Identifier {
            self.emit_error(
                TrussDiagnosticCode::ExpectedIdentifier,
                format!(
                    "Expected variable name after 'let' but found '{}'",
                    name.value
                ),
                &name,
            );
            return Err(());
        }

        let equals = self.next().ok_or_else(|| {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '=' in if-let condition",
                &self.tokens[self.index.saturating_sub(1)],
            );
        })?;

        if !OperatorType::is_operator(&equals, OperatorType::Assign) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '=' but found '{}'", equals.value),
                &equals,
            );
            return Err(());
        }

        let saved_suppress = self.suppress_trailing_closure;
        self.suppress_trailing_closure = true;
        let value = self.parse_binary(Precedence::And)?;
        self.suppress_trailing_closure = saved_suppress;

        let pattern = if name.value == "_" {
            crate::ast::statement::Pattern::Ignore
        } else {
            crate::ast::statement::Pattern::Identifier(Box::new(name))
        };

        let some_token = Token::new(
            "Some".to_string(),
            TokenType::Identifier,
            let_token.position.clone(),
            let_token.file.clone(),
        );

        let case_expr = Expression::Case {
            token: Box::new(let_token),
            enum_type: None,
            case_name: Box::new(some_token),
            bindings: vec![pattern],
            expression: Rc::new(RefCell::new(value)),
            ty: None,
        };

        if let Some(t) = self.peek()
            && OperatorType::is_operator(&t, OperatorType::And)
        {
            let _and_token = self.next().unwrap();
            if let Some(next) = self.peek()
                && KeywordType::is_keyword(&next, KeywordType::Let)
            {
                let right = self.parse_if_let_condition()?;
                Ok(Expression::Binary {
                    left: Rc::new(RefCell::new(case_expr)),
                    operator: crate::ast::expression::BinaryOperator::And,
                    right: Rc::new(RefCell::new(right)),
                    overloads: vec![],
                    selected_index: None,
                })
            } else {
                let right = self.parse_expression()?;
                Ok(Expression::Binary {
                    left: Rc::new(RefCell::new(case_expr)),
                    operator: crate::ast::expression::BinaryOperator::And,
                    right: Rc::new(RefCell::new(right)),
                    overloads: vec![],
                    selected_index: None,
                })
            }
        } else {
            Ok(case_expr)
        }
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, ()> {
        let mut attributes: Vec<Attribute> = Vec::new();
        loop {
            let Some(tok) = self.peek() else {
                break;
            };
            if !SeparatorType::is_separator(&tok, SeparatorType::Hash) {
                break;
            }
            let Some(next) = self.peek2() else {
                break;
            };
            if !SeparatorType::is_separator(&next, SeparatorType::OpenBracket) {
                break;
            }
            self.next().unwrap();
            self.next().unwrap();
            let Some(name_tok) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    "Expected attribute name after '#['",
                    &tok,
                );
                return Err(());
            };
            if !matches!(name_tok.ty, TokenType::Identifier) {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    format!("Expected attribute name but found '{}'", name_tok.value),
                    &name_tok,
                );
                return Err(());
            }
            let attr_value = if let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::OpenParen)
            {
                self.index += 1;
                let Some(val_tok) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected string literal in attribute value",
                        &tok,
                    );
                    return Err(());
                };
                let value = match &val_tok.ty {
                    TokenType::StringLiteral { value } => value.clone(),
                    TokenType::Identifier => val_tok.value.clone(),
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::ParserError,
                            format!(
                                "Expected string literal or identifier but found '{}'",
                                val_tok.value
                            ),
                            &val_tok,
                        );
                        return Err(());
                    }
                };
                let Some(close_paren) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected ')' to close attribute value",
                        &tok,
                    );
                    return Err(());
                };
                if !SeparatorType::is_separator(&close_paren, SeparatorType::CloseParen) {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        format!("Expected ')' but found '{}'", close_paren.value),
                        &close_paren,
                    );
                    return Err(());
                }
                Some(value)
            } else {
                None
            };
            let Some(close) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    "Expected ']' to close attribute",
                    &tok,
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close, SeparatorType::CloseBracket) {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    format!("Expected ']' but found '{}'", close.value),
                    &close,
                );
                return Err(());
            }
            attributes.push(Attribute {
                name: name_tok.value,
                value: attr_value,
            });
        }
        Ok(attributes)
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
            let is_set_syntax = matches!(
                keyword,
                KeywordType::Open
                    | KeywordType::Public
                    | KeywordType::Internal
                    | KeywordType::Fileprivate
                    | KeywordType::Private
                    | KeywordType::Package
            ) && self.is_set_modifier_syntax();
            let ty = if is_set_syntax {
                let modifier = match keyword {
                    KeywordType::Open => AccessModifier::Open,
                    KeywordType::Public => AccessModifier::Public,
                    KeywordType::Internal => AccessModifier::Internal,
                    KeywordType::Fileprivate => AccessModifier::Fileprivate,
                    KeywordType::Private => AccessModifier::Private,
                    KeywordType::Package => AccessModifier::Package,
                    _ => unreachable!(),
                };
                ModifierType::AccessSet(modifier)
            } else {
                match keyword {
                    KeywordType::Open => ModifierType::Access(AccessModifier::Open),
                    KeywordType::Public => ModifierType::Access(AccessModifier::Public),
                    KeywordType::Internal => ModifierType::Access(AccessModifier::Internal),
                    KeywordType::Fileprivate => ModifierType::Access(AccessModifier::Fileprivate),
                    KeywordType::Private => ModifierType::Access(AccessModifier::Private),
                    KeywordType::Package => ModifierType::Access(AccessModifier::Package),
                    KeywordType::Static => ModifierType::Static,
                    KeywordType::Mutating => ModifierType::Mutating,
                    KeywordType::Prefix => ModifierType::OperatorFixity(OperatorFixity::Prefix),
                    KeywordType::Postfix => ModifierType::OperatorFixity(OperatorFixity::Postfix),
                    KeywordType::Override => ModifierType::Override,
                    KeywordType::Abstract => ModifierType::Abstract,
                    KeywordType::Final => ModifierType::Final,
                    KeywordType::Weak => ModifierType::Weak,
                    KeywordType::Unowned => ModifierType::Unowned,
                    _ => {
                        break;
                    }
                }
            };
            let duplicate = if matches!(ty, ModifierType::AccessSet(_)) {
                modifiers
                    .iter()
                    .any(|m| matches!(m.ty, ModifierType::AccessSet(_)))
            } else {
                modifiers.iter().any(|m| {
                    m.ty == ty
                        || (matches!(m.ty, ModifierType::Access(_))
                            && matches!(ty, ModifierType::Access(_)))
                })
            };
            if duplicate {
                self.emit_error(
                    TrussDiagnosticCode::DuplicateModifier,
                    format!("Duplicate modifier: '{}'", token.value),
                    &token,
                );
                self.index += 1;
                if is_set_syntax {
                    self.index += 3;
                }
                continue;
            }
            modifiers.push(Modifier {
                token: Box::new(token.clone()),
                ty,
            });
            self.index += 1;
            if is_set_syntax {
                self.index += 3;
            }
        }
        Ok(modifiers)
    }

    fn is_set_modifier_syntax(&self) -> bool {
        self.index + 3 < self.tokens.len()
            && self.tokens[self.index + 1].ty
                == TokenType::Separator {
                    separator: SeparatorType::OpenParen,
                }
            && self.tokens[self.index + 2].value == "set"
            && self.tokens[self.index + 3].ty
                == TokenType::Separator {
                    separator: SeparatorType::CloseParen,
                }
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
        if self.pending_greater_count > 0 {
            self.pending_greater_count -= 1;
            return Ok(Some(Vec::new()));
        }
        if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Less)
        {
            self.index += 1;
            let mut type_parameters = Vec::new();
            while let Some(token) = self.peek() {
                if OperatorType::is_operator(&token, OperatorType::Greater) {
                    break;
                }
                type_parameters.push(Rc::new(RefCell::new(
                    if matches!(
                        token.ty,
                        TokenType::IntegerLiteral { .. }
                            | TokenType::BooleanLiteral { .. }
                            | TokenType::CharLiteral { .. }
                            | TokenType::StringLiteral { .. }
                            | TokenType::NullLiteral
                            | TokenType::NullptrLiteral
                    ) {
                        self.parse_primary()?
                    } else {
                        self.parse_type_expression()?
                    },
                )));
                let Some(t) = self.peek() else { break };
                if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                    self.index += 1;
                } else {
                    break;
                }
            }
            if self.pending_greater_count > 0 {
                self.pending_greater_count -= 1;
                return Ok(Some(type_parameters));
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '>' to close type parameters",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if OperatorType::is_operator(&next, OperatorType::Greater) {
                Ok(Some(type_parameters))
            } else if OperatorType::is_operator(&next, OperatorType::RightShift) {
                self.pending_greater_count += 1;
                Ok(Some(type_parameters))
            } else if OperatorType::is_operator(&next, OperatorType::RightShiftAssign) {
                self.index -= 1;
                Ok(Some(type_parameters))
            } else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '>' but found '{}'", next.value),
                    &next,
                );
                Err(())
            }
        } else {
            Ok(None)
        }
    }

    fn extract_first_name(pattern: &Pattern) -> Option<&Token> {
        match pattern {
            Pattern::Identifier(tok) => Some(tok.as_ref()),
            Pattern::Tuple(items) => items.first().and_then(Self::extract_first_name),
            Pattern::ValueBinding(inner) => Self::extract_first_name(inner.as_ref()),
            Pattern::EnumCase { case_name, .. } => Some(case_name.as_ref()),
            _ => None,
        }
    }

    fn extract_ownership(modifiers: &[Modifier]) -> OwnershipModifier {
        for m in modifiers {
            match m.ty {
                ModifierType::Weak => return OwnershipModifier::Weak,
                ModifierType::Unowned => return OwnershipModifier::Unowned,
                _ => {}
            }
        }
        OwnershipModifier::Strong
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ()> {
        if let Some(token) = self.peek()
            && let TokenType::Keyword { keyword } = token.ty
            && (keyword == KeywordType::Let || keyword == KeywordType::Var)
        {
            self.index += 1;
            let inner = self.parse_pattern()?;
            return Ok(Pattern::ValueBinding(Box::new(inner)));
        }

        if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Dot)
        {
            self.index += 1;
            let Some(case_name) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    "Expected case name after '.'",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if case_name.ty != TokenType::Identifier {
                self.emit_error(
                    TrussDiagnosticCode::ExpectedIdentifier,
                    format!("Expected case name but found '{}'", case_name.value),
                    &case_name,
                );
                return Err(());
            }
            let mut bindings = Vec::new();
            if let Some(next) = self.peek() {
                if SeparatorType::is_separator(&next, SeparatorType::OpenParen) {
                    self.index += 1;
                    loop {
                        if let Some(t) = self.peek()
                            && SeparatorType::is_separator(&t, SeparatorType::CloseParen)
                        {
                            break;
                        }
                        bindings.push(self.parse_pattern()?);
                        if let Some(t) = self.peek()
                            && SeparatorType::is_separator(&t, SeparatorType::Comma)
                        {
                            self.index += 1;
                        } else {
                            break;
                        }
                    }
                    let Some(_close_paren) = self.next() else {
                        self.emit_error(
                            TrussDiagnosticCode::MissingSeparator,
                            "Expected ')' to close enum case pattern",
                            &self.tokens[self.index.saturating_sub(1)],
                        );
                        return Err(());
                    };
                }
            }
            return Ok(Pattern::EnumCase {
                case_name: Box::new(case_name),
                bindings,
            });
        }

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
            TokenType::IntegerLiteral { .. }
            | TokenType::DecimalLiteral { .. }
            | TokenType::BooleanLiteral { .. }
            | TokenType::NullLiteral
            | TokenType::NullptrLiteral
            | TokenType::CharLiteral { .. } => {
                self.index -= 1;
                let expr = self.parse_expression()?;
                Ok(Pattern::Expr(Rc::new(RefCell::new(expr))))
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
                    let Some(_close) = self.next() else {
                        return Err(());
                    };
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

    fn parse_match(&mut self) -> Result<Expression, ()> {
        let token = self.next().unwrap();
        let value = self.parse_expression()?;

        let Some(open_brace) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after match expression",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !SeparatorType::is_separator(&open_brace, SeparatorType::OpenBrace) {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                format!("Expected '{{' but found '{}'", open_brace.value),
                &open_brace,
            );
            return Err(());
        }

        let mut cases = Vec::new();
        loop {
            if let Some(t) = self.peek() {
                if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                    self.index += 1;
                    break;
                }
            } else {
                break;
            }

            if let Some(t) = self.peek()
                && KeywordType::is_keyword(&t, KeywordType::Default)
            {
                self.index += 1;
                let Some(colon) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected ':' after 'default'",
                        &self.tokens[self.index.saturating_sub(1)],
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
                let body = self.parse_match_case_body()?;
                cases.push(MatchCase {
                    token: Box::new(t),
                    patterns: vec![Rc::new(Pattern::Ignore)],
                    guard: None,
                    body,
                });
                continue;
            }

            let Some(case_token) = self.next() else {
                break;
            };
            if !KeywordType::is_keyword(&case_token, KeywordType::Case) {
                self.emit_error(
                    TrussDiagnosticCode::UnexpectedToken,
                    format!(
                        "Expected 'case' or 'default' in match, found '{}'",
                        case_token.value
                    ),
                    &case_token,
                );
                return Err(());
            }

            let mut patterns = vec![Rc::new(self.parse_pattern()?)];
            while let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::Comma)
            {
                self.index += 1;
                patterns.push(Rc::new(self.parse_pattern()?));
            }

            let guard = if let Some(t) = self.peek()
                && KeywordType::is_keyword(&t, KeywordType::Where)
            {
                self.index += 1;
                Some(Rc::new(RefCell::new(self.parse_expression()?)))
            } else {
                None
            };

            let Some(colon) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected ':' after case pattern",
                    &self.tokens[self.index.saturating_sub(1)],
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

            let body = self.parse_match_case_body()?;
            cases.push(MatchCase {
                token: Box::new(case_token),
                patterns,
                guard,
                body,
            });
        }

        Ok(Expression::Match {
            token: Box::new(token),
            value: Rc::new(RefCell::new(value)),
            cases,
            ty: None,
        })
    }

    fn parse_match_case_body(&mut self) -> Result<Vec<Rc<RefCell<Statement>>>, ()> {
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            return self.parse_block();
        }
        if let Some(t) = self.peek() {
            if let TokenType::Keyword { keyword } = &t.ty {
                match keyword {
                    KeywordType::Fallthrough => {
                        let stmt = self.parse_fallthrough()?;
                        return Ok(vec![Rc::new(RefCell::new(stmt))]);
                    }
                    KeywordType::Break => {
                        let stmt = self.parse_break()?;
                        return Ok(vec![Rc::new(RefCell::new(stmt))]);
                    }
                    KeywordType::Case | KeywordType::Default => {
                        return Ok(vec![]);
                    }
                    _ => {}
                }
            }
            if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
                return Ok(vec![]);
            }
        }
        let expr = self.parse_expression()?;
        Ok(vec![Rc::new(RefCell::new(
            Statement::ExpressionStatement {
                expression: Rc::new(RefCell::new(expr)),
            },
        ))])
    }

    fn parse_guard(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let saved_suppress = self.suppress_trailing_closure;
        self.suppress_trailing_closure = true;
        let condition = if let Some(t) = self.peek()
            && KeywordType::is_keyword(&t, KeywordType::Let)
        {
            let cond = self.parse_if_let_condition()?;
            self.suppress_trailing_closure = saved_suppress;
            cond
        } else {
            let cond = self.parse_expression()?;
            self.suppress_trailing_closure = saved_suppress;
            cond
        };

        let Some(else_token) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected 'else' after guard condition",
                &self.tokens[self.index.saturating_sub(1)],
            );
            return Err(());
        };
        if !KeywordType::is_keyword(&else_token, KeywordType::Else) {
            self.emit_error(
                TrussDiagnosticCode::UnexpectedToken,
                format!("Expected 'else' but found '{}'", else_token.value),
                &else_token,
            );
            return Err(());
        }

        let else_body = self.parse_block()?;

        Ok(Statement::Guard {
            token: Box::new(token),
            condition: Rc::new(RefCell::new(condition)),
            else_body,
        })
    }

    fn parse_fallthrough(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        Ok(Statement::Fallthrough {
            token: Box::new(token),
        })
    }

    fn parse_do_expression(&mut self) -> Result<Expression, ()> {
        let token = self.next().unwrap();
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            let mut catch_clauses = Vec::new();
            while let Some(t) = self.peek()
                && KeywordType::is_keyword(&t, KeywordType::Catch)
            {
                self.index += 1;
                let pattern = if let Some(t) = self.peek()
                    && !SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
                    && !KeywordType::is_keyword(&t, KeywordType::Where)
                    && !KeywordType::is_keyword(&t, KeywordType::Finally)
                {
                    Some(self.parse_pattern()?)
                } else {
                    None
                };
                let guard = if let Some(t) = self.peek()
                    && KeywordType::is_keyword(&t, KeywordType::Where)
                {
                    self.index += 1;
                    Some(Rc::new(RefCell::new(self.parse_expression()?)))
                } else {
                    None
                };
                if let Some(t) = self.peek()
                    && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
                {
                    let catch_body = self.parse_block()?;
                    catch_clauses.push(CatchClause {
                        pattern,
                        guard,
                        body: catch_body,
                    });
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected '{' after 'catch'".to_string(),
                        &t,
                    );
                    return Err(());
                }
            }
            let finally_body = if let Some(t) = self.peek()
                && KeywordType::is_keyword(&t, KeywordType::Finally)
            {
                self.index += 1;
                if let Some(t) = self.peek()
                    && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
                {
                    self.parse_block()?
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::MissingSeparator,
                        "Expected '{' after 'finally'".to_string(),
                        &t,
                    );
                    return Err(());
                }
            } else {
                Vec::new()
            };
            Ok(Expression::Do {
                token: Box::new(token),
                body,
                catch_clauses,
                finally_body,
                scope: None,
                ty: None,
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::MissingSeparator,
                "Expected '{' after 'do'".to_string(),
                &token,
            );
            Err(())
        }
    }

    fn parse_try_expression(&mut self) -> Result<Expression, ()> {
        let token = self.next().unwrap();
        let kind = if let Some(t) = self.peek()
            && OperatorType::is_operator(&t, OperatorType::Not)
        {
            self.index += 1;
            TryKind::Force
        } else if let Some(t) = self.peek()
            && OperatorType::is_operator(&t, OperatorType::QuestionMark)
        {
            self.index += 1;
            TryKind::Optional
        } else {
            TryKind::Plain
        };
        let expression = Rc::new(RefCell::new(self.parse_expression()?));
        Ok(Expression::Try {
            token: Box::new(token),
            kind,
            expression,
            ty: None,
        })
    }

    fn parse_break(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        Ok(Statement::Break {
            token: Box::new(token),
        })
    }

    fn parse_defer(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
        {
            let body = self.parse_block()?;
            for stmt in &body {
                if Self::is_forbidden_in_defer(&*stmt.borrow()) {
                    self.emit_error(
                        TrussDiagnosticCode::ControlFlowNotAllowedInDefer,
                        format!(
                            "'{}' is not allowed in defer body",
                            stmt.borrow().token().value
                        ),
                        &stmt.borrow().token(),
                    );
                }
            }
            Ok(Statement::Defer {
                token: Box::new(token),
                body,
            })
        } else {
            self.emit_error(
                TrussDiagnosticCode::ExpectedBlockAfterDefer,
                "Expected '{' after 'defer'",
                &self.tokens[self.index],
            );
            Err(())
        }
    }

    fn is_forbidden_in_defer(stmt: &Statement) -> bool {
        matches!(
            stmt,
            Statement::Return { .. }
                | Statement::Yield { .. }
                | Statement::Throw { .. }
                | Statement::Break { .. }
                | Statement::Fallthrough { .. }
        )
    }

    fn parse_asm_block(&mut self) -> Result<Statement, ()> {
        let token = self.next().unwrap();
        let Some(open_brace) = self.peek() else {
            self.emit_error(
                TrussDiagnosticCode::ParserAsmBlockError,
                "Expected '{' after 'asm'",
                &token,
            );
            return Err(());
        };
        if !SeparatorType::is_separator(&open_brace, SeparatorType::OpenBrace) {
            self.emit_error(
                TrussDiagnosticCode::ParserAsmBlockError,
                "Expected '{' after 'asm'",
                &open_brace,
            );
            return Err(());
        }
        self.index += 1;

        let mut instructions = Vec::new();
        loop {
            match self.peek() {
                Some(t)
                    if SeparatorType::is_separator(&t, SeparatorType::Colon)
                        || SeparatorType::is_separator(&t, SeparatorType::CloseBrace) =>
                {
                    break;
                }
                Some(t) => {
                    if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                        self.index += 1;
                        continue;
                    }
                    if let TokenType::StringLiteral { .. } = &t.ty {
                        instructions.push(self.next().unwrap());
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::ParserAsmBlockError,
                            "Expected string literal for assembly instruction",
                            &t,
                        );
                        return Err(());
                    }
                }
                None => {
                    self.emit_error(
                        TrussDiagnosticCode::ParserAsmBlockError,
                        "Expected '}' after assembly block",
                        &token,
                    );
                    return Err(());
                }
            }
        }

        let mut outputs = Vec::new();
        let mut inputs = Vec::new();
        let mut clobbers = Vec::new();

        if let Some(t) = self.peek()
            && SeparatorType::is_separator(&t, SeparatorType::Colon)
        {
            self.index += 1;
            self.parse_asm_operands(&mut outputs, AsmDirection::Out)?;

            if let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::Colon)
            {
                self.index += 1;
                self.parse_asm_operands(&mut inputs, AsmDirection::In)?;

                if let Some(t) = self.peek()
                    && SeparatorType::is_separator(&t, SeparatorType::Colon)
                {
                    self.index += 1;
                    self.parse_asm_clobbers(&mut clobbers)?;
                }
            }
        }

        let Some(close_brace) = self.peek() else {
            self.emit_error(
                TrussDiagnosticCode::ParserAsmBlockError,
                "Expected '}' at end of inline assembly block",
                &token,
            );
            return Err(());
        };
        if !SeparatorType::is_separator(&close_brace, SeparatorType::CloseBrace) {
            self.emit_error(
                TrussDiagnosticCode::ParserAsmBlockError,
                "Expected '}' at end of inline assembly block",
                &close_brace,
            );
            return Err(());
        }
        self.index += 1;

        Ok(Statement::AsmBlock {
            token: Box::new(token),
            instructions,
            outputs,
            inputs,
            clobbers,
        })
    }

    fn parse_asm_operands(
        &mut self,
        operands: &mut Vec<AsmOperand>,
        direction: AsmDirection,
    ) -> Result<(), ()> {
        loop {
            match self.peek() {
                Some(t)
                    if SeparatorType::is_separator(&t, SeparatorType::Colon)
                        || SeparatorType::is_separator(&t, SeparatorType::CloseBrace) =>
                {
                    break;
                }
                None => break,
                _ => {}
            }

            let label = self.next().unwrap();

            let Some(eq_token) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    "Expected '=' after operand label",
                    &self.tokens[self.index],
                );
                return Err(());
            };
            if !OperatorType::is_operator(&eq_token, OperatorType::Assign) {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    "Expected '=' after operand label",
                    &eq_token,
                );
                return Err(());
            }

            let Some(dir_token) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    "Expected 'in' or 'out' as operand direction",
                    &self.tokens[self.index],
                );
                return Err(());
            };
            let dir = match &dir_token.ty {
                TokenType::Keyword { keyword } if *keyword == KeywordType::In => AsmDirection::In,
                TokenType::Identifier if dir_token.value == "out" => AsmDirection::Out,
                _ => {
                    self.emit_error(
                        TrussDiagnosticCode::ParserAsmBlockError,
                        "Expected 'in' or 'out' as operand direction",
                        &dir_token,
                    );
                    return Err(());
                }
            };
            if dir != direction {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    &format!(
                        "Expected '{:?}' operand direction, got '{:?}'",
                        direction, dir
                    ),
                    &dir_token,
                );
                return Err(());
            }

            let Some(open_paren) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    "Expected '(' after operand direction",
                    &self.tokens[self.index],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&open_paren, SeparatorType::OpenParen) {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    "Expected '(' after operand direction",
                    &open_paren,
                );
                return Err(());
            }

            let constraint = self.next().unwrap();

            let Some(close_paren) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    "Expected ')' after constraint",
                    &self.tokens[self.index],
                );
                return Err(());
            };
            if !SeparatorType::is_separator(&close_paren, SeparatorType::CloseParen) {
                self.emit_error(
                    TrussDiagnosticCode::ParserAsmBlockError,
                    "Expected ')' after constraint",
                    &close_paren,
                );
                return Err(());
            }

            let expression = self.parse_expression()?;

            operands.push(AsmOperand {
                label: Box::new(label),
                direction: dir,
                constraint: Box::new(constraint),
                expression: Rc::new(RefCell::new(expression)),
            });

            if let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::Comma)
            {
                self.index += 1;
            }
        }
        Ok(())
    }

    fn parse_asm_clobbers(&mut self, clobbers: &mut Vec<Token>) -> Result<(), ()> {
        loop {
            match self.peek() {
                Some(t) if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) => {
                    break;
                }
                None => break,
                Some(t) => {
                    if let TokenType::StringLiteral { .. } = &t.ty {
                        clobbers.push(self.next().unwrap());
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::ParserAsmBlockError,
                            "Expected string literal for clobber register",
                            &t,
                        );
                        return Err(());
                    }
                }
            }

            if let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::Comma)
            {
                self.index += 1;
            }
        }
        Ok(())
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

    fn parse_generic_parameters(&mut self) -> Result<Option<Vec<GenericParameter>>, ()> {
        if let Some(token) = self.peek()
            && OperatorType::is_operator(&token, OperatorType::Less)
        {
            self.index += 1;
            let mut params = Vec::new();
            loop {
                if let Some(t) = self.peek()
                    && KeywordType::is_keyword(&t, KeywordType::Let)
                {
                    self.index += 1;
                    let Some(name) = self.next() else { break };
                    if TokenType::Identifier != name.ty {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            format!(
                                "Expected constant generic parameter name but found '{}'",
                                name.value
                            ),
                            &name,
                        );
                        return Err(());
                    }
                    let has_colon = if let Some(ct) = self.peek()
                        && SeparatorType::is_separator(&ct, SeparatorType::Colon)
                    {
                        self.index += 1;
                        true
                    } else {
                        false
                    };
                    if !has_colon {
                        self.emit_error(
                            TrussDiagnosticCode::MissingSeparator,
                            "Expected ':' after constant generic parameter name".to_string(),
                            &name,
                        );
                        return Err(());
                    }
                    let const_type = Rc::new(RefCell::new(self.parse_type_expression()?));
                    let default_value = if let Some(t) = self.peek()
                        && let TokenType::Operator { .. } = t.ty
                        && OperatorType::is_operator(&t, OperatorType::Assign)
                    {
                        self.index += 1;
                        Some(Rc::new(RefCell::new(self.parse_binary(Precedence::Range)?)))
                    } else {
                        None
                    };
                    params.push(GenericParameter {
                        name: Box::new(name),
                        kind: GenericParameterKind::Const { const_type },
                        default_value,
                    });
                } else {
                    let Some(name) = self.next() else { break };
                    if TokenType::Identifier != name.ty {
                        self.emit_error(
                            TrussDiagnosticCode::ExpectedIdentifier,
                            format!("Expected generic parameter name but found '{}'", name.value),
                            &name,
                        );
                        return Err(());
                    }
                    let mut constraints = Vec::new();
                    if let Some(t) = self.peek()
                        && SeparatorType::is_separator(&t, SeparatorType::Colon)
                    {
                        self.index += 1;
                        constraints.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
                    }
                    let default_value = if let Some(t) = self.peek()
                        && let TokenType::Operator { .. } = t.ty
                        && OperatorType::is_operator(&t, OperatorType::Assign)
                    {
                        self.index += 1;
                        Some(Rc::new(RefCell::new(self.parse_type_expression()?)))
                    } else {
                        None
                    };
                    params.push(GenericParameter {
                        name: Box::new(name),
                        kind: GenericParameterKind::Type { constraints },
                        default_value,
                    });
                }
                let Some(t) = self.peek() else { break };
                if OperatorType::is_operator(&t, OperatorType::Greater) {
                    break;
                } else if OperatorType::is_operator(&t, OperatorType::RightShift) {
                    self.pending_greater_count += 1;
                    break;
                } else if OperatorType::is_operator(&t, OperatorType::RightShiftAssign) {
                    break;
                } else if SeparatorType::is_separator(&t, SeparatorType::Comma) {
                    self.index += 1;
                } else {
                    break;
                }
            }
            if self.pending_greater_count > 0 {
                self.pending_greater_count -= 1;
                return Ok(Some(params));
            }
            let Some(next) = self.next() else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    "Expected '>' to close generic parameter list",
                    &self.tokens[self.index.saturating_sub(1)],
                );
                return Err(());
            };
            if OperatorType::is_operator(&next, OperatorType::Greater) {
                Ok(Some(params))
            } else if OperatorType::is_operator(&next, OperatorType::RightShift) {
                self.pending_greater_count += 1;
                Ok(Some(params))
            } else if OperatorType::is_operator(&next, OperatorType::RightShiftAssign) {
                self.index -= 1;
                Ok(Some(params))
            } else {
                self.emit_error(
                    TrussDiagnosticCode::MissingSeparator,
                    format!("Expected '>' but found '{}'", next.value),
                    &next,
                );
                Err(())
            }
        } else {
            Ok(None)
        }
    }

    fn parse_where_clause(&mut self) -> Result<Option<Vec<WhereRequirement>>, ()> {
        if let Some(token) = self.peek()
            && KeywordType::is_keyword(&token, KeywordType::Where)
        {
            self.index += 1;
            let mut requirements = Vec::new();
            loop {
                let type_expr = self.parse_type_expression()?;
                if let Some(t) = self.peek()
                    && SeparatorType::is_separator(&t, SeparatorType::Colon)
                {
                    self.index += 1;
                    let constraint = self.parse_type_expression()?;
                    requirements.push(WhereRequirement {
                        kind: WhereRequirementKind::Conformance {
                            type_expr: Rc::new(RefCell::new(type_expr)),
                            constraint: Rc::new(RefCell::new(constraint)),
                        },
                    });
                } else if let Some(t) = self.peek()
                    && OperatorType::is_operator(&t, OperatorType::Equal)
                {
                    self.index += 1;
                    let right = self.parse_type_expression()?;
                    requirements.push(WhereRequirement {
                        kind: WhereRequirementKind::Equality {
                            left: Rc::new(RefCell::new(type_expr)),
                            right: Rc::new(RefCell::new(right)),
                        },
                    });
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UnexpectedToken,
                        "Expected ':' or '==' in where clause requirement",
                        &self.tokens[self.index.saturating_sub(1)],
                    );
                    return Err(());
                }
                let Some(t) = self.peek() else { break };
                if OperatorType::is_operator(&t, OperatorType::And) {
                    self.index += 1;
                } else {
                    break;
                }
            }
            Ok(Some(requirements))
        } else {
            Ok(None)
        }
    }

    fn parse_preprocessor_directive(&mut self) -> Result<Statement, ()> {
        let hash_token = self.next().unwrap();
        let Some(directive_token) = self.next() else {
            self.emit_error(
                TrussDiagnosticCode::ParserError,
                "Expected preprocessor directive name after '#'",
                &hash_token,
            );
            return Err(());
        };
        match directive_token.value.as_str() {
            "if" => {
                let condition = self.parse_condition()?;
                self.parse_conditional_block_rest(Some(condition), directive_token)
            }
            "ifdef" => {
                let Some(ident) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected identifier after '#ifdef'",
                        &directive_token,
                    );
                    return Err(());
                };
                let condition = Condition::Defined(ident);
                self.parse_conditional_block_rest(Some(condition), directive_token)
            }
            "ifndef" => {
                let Some(ident) = self.next() else {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected identifier after '#ifndef'",
                        &directive_token,
                    );
                    return Err(());
                };
                let condition = Condition::Not(Box::new(Condition::Defined(ident)));
                self.parse_conditional_block_rest(Some(condition), directive_token)
            }
            "else" | "elseif" | "endif" => {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    format!("#{} without matching #if", directive_token.value),
                    &directive_token,
                );
                Err(())
            }
            "error" => self.parse_pragma_directive(true, directive_token),
            "warning" => self.parse_pragma_directive(false, directive_token),
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    format!(
                        "Unknown preprocessor directive '#{}'",
                        directive_token.value
                    ),
                    &directive_token,
                );
                Err(())
            }
        }
    }

    fn parse_pragma_directive(
        &mut self,
        is_error: bool,
        directive_token: Token,
    ) -> Result<Statement, ()> {
        let Some(msg_token) = self.peek() else {
            self.emit_error(
                TrussDiagnosticCode::ParserError,
                format!("Expected string literal after '#{}'", directive_token.value),
                &directive_token,
            );
            return Err(());
        };
        let msg = match &msg_token.ty {
            TokenType::StringLiteral { value } => value.clone(),
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    format!("Expected string literal after '#{}'", directive_token.value),
                    &msg_token,
                );
                return Err(());
            }
        };
        self.index += 1;
        if is_error {
            Ok(Statement::PragmaError {
                token: Box::new(directive_token),
                message: msg,
            })
        } else {
            Ok(Statement::PragmaWarning {
                token: Box::new(directive_token),
                message: msg,
            })
        }
    }

    fn parse_conditional_block_rest(
        &mut self,
        first_condition: Option<Condition>,
        first_token: Token,
    ) -> Result<Statement, ()> {
        let mut clauses: Vec<ConditionalClause> = Vec::new();

        let body = self.parse_conditional_body()?;
        let mut has_else = false;
        clauses.push(ConditionalClause {
            token: Box::new(first_token),
            condition: first_condition,
            body,
        });

        loop {
            if self.is_empty() {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    "Expected #endif".to_string(),
                    &clauses.last().unwrap().token,
                );
                return Err(());
            }
            if !self.check_hash_next() {
                break;
            }
            let Some(next_val) = self.peek_hash_next_value() else {
                break;
            };
            match next_val.as_str() {
                "elseif" => {
                    if has_else {
                        self.emit_error(
                            TrussDiagnosticCode::ParserError,
                            "#elseif after #else".to_string(),
                            &clauses.last().unwrap().token,
                        );
                        return Err(());
                    }
                    self.index += 2;
                    let condition = self.parse_condition()?;
                    let body = self.parse_conditional_body()?;
                    clauses.push(ConditionalClause {
                        token: Box::new(self.tokens[self.index - 2].clone()),
                        condition: Some(condition),
                        body,
                    });
                }
                "else" => {
                    if has_else {
                        self.emit_error(
                            TrussDiagnosticCode::ParserError,
                            "Multiple #else clauses".to_string(),
                            &clauses.last().unwrap().token,
                        );
                        return Err(());
                    }
                    has_else = true;
                    self.index += 2;
                    let body = self.parse_conditional_body()?;
                    clauses.push(ConditionalClause {
                        token: Box::new(self.tokens[self.index - 2].clone()),
                        condition: None,
                        body,
                    });
                }
                "endif" => {
                    self.index += 2;
                    return Ok(Statement::ConditionalBlock { clauses });
                }
                _ => break,
            }
        }

        self.emit_error(
            TrussDiagnosticCode::ParserError,
            "Expected #endif".to_string(),
            &clauses.last().unwrap().token,
        );
        Err(())
    }

    fn parse_conditional_body(&mut self) -> Result<Vec<Rc<RefCell<Statement>>>, ()> {
        let mut statements = Vec::new();
        self.scope_nesting += 1;
        loop {
            if self.is_empty() {
                break;
            }
            if self.check_hash_next() {
                if let Some(val) = self.peek_hash_next_value() {
                    match val.as_str() {
                        "elseif" | "else" | "endif" => break,
                        _ => {}
                    }
                }
            }
            let stmt = self.parse_statement()?;
            statements.push(Rc::new(RefCell::new(stmt)));
        }
        self.scope_nesting -= 1;
        Ok(statements)
    }

    fn check_hash_next(&self) -> bool {
        if let Some(token) = self.peek() {
            SeparatorType::is_separator(&token, SeparatorType::Hash)
        } else {
            false
        }
    }

    fn peek_hash_next_value(&self) -> Option<String> {
        if self.check_hash_next() {
            self.peek2().map(|t| t.value.clone())
        } else {
            None
        }
    }

    fn parse_condition(&mut self) -> Result<Condition, ()> {
        self.parse_condition_or()
    }

    fn parse_condition_or(&mut self) -> Result<Condition, ()> {
        let mut left = self.parse_condition_and()?;
        loop {
            if let Some(token) = self.peek() {
                if OperatorType::is_operator(&token, OperatorType::Or) {
                    self.index += 1;
                    let right = self.parse_condition_and()?;
                    left = Condition::Or(Box::new(left), Box::new(right));
                    continue;
                }
            }
            break;
        }
        Ok(left)
    }

    fn parse_condition_and(&mut self) -> Result<Condition, ()> {
        let mut left = self.parse_condition_unary()?;
        loop {
            if let Some(token) = self.peek() {
                if OperatorType::is_operator(&token, OperatorType::And) {
                    self.index += 1;
                    let right = self.parse_condition_unary()?;
                    left = Condition::And(Box::new(left), Box::new(right));
                    continue;
                }
            }
            break;
        }
        Ok(left)
    }

    fn parse_condition_unary(&mut self) -> Result<Condition, ()> {
        if let Some(token) = self.peek() {
            if OperatorType::is_operator(&token, OperatorType::Not) {
                self.index += 1;
                let inner = self.parse_condition_unary()?;
                return Ok(Condition::Not(Box::new(inner)));
            }
        }
        self.parse_condition_primary()
    }

    fn parse_condition_primary(&mut self) -> Result<Condition, ()> {
        let Some(token) = self.peek() else {
            return Err(());
        };
        match &token.ty {
            TokenType::BooleanLiteral { value } => {
                self.index += 1;
                Ok(Condition::Bool(*value))
            }
            TokenType::Identifier => {
                let ident = self.next().unwrap();
                if ident.value == "defined" || ident.value == "os" || ident.value == "arch" {
                    if let Some(paren) = self.peek() {
                        if SeparatorType::is_separator(&paren, SeparatorType::OpenParen) {
                            self.index += 1;
                            let Some(inner) = self.peek() else {
                                self.emit_error(
                                    TrussDiagnosticCode::ParserError,
                                    format!("Expected identifier in '{}()'", ident.value),
                                    &ident,
                                );
                                return Err(());
                            };
                            if !matches!(inner.ty, TokenType::Identifier) {
                                self.emit_error(
                                    TrussDiagnosticCode::ParserError,
                                    format!("Expected identifier in '{}()'", ident.value),
                                    &inner,
                                );
                                return Err(());
                            }
                            let inner_ident = self.next().unwrap();
                            if let Some(close) = self.peek() {
                                if SeparatorType::is_separator(&close, SeparatorType::CloseParen) {
                                    self.index += 1;
                                    return Ok(match ident.value.as_str() {
                                        "os" => Condition::Os(inner_ident),
                                        "arch" => Condition::Arch(inner_ident),
                                        _ => Condition::Defined(inner_ident),
                                    });
                                }
                            }
                            self.emit_error(
                                TrussDiagnosticCode::ParserError,
                                format!("Expected ')' after '{}('", ident.value),
                                &ident,
                            );
                            return Err(());
                        }
                    }
                }
                Ok(Condition::Platform(ident))
            }
            TokenType::Separator { separator } if *separator == SeparatorType::OpenParen => {
                self.index += 1;
                let inner = self.parse_condition()?;
                let Some(close) = self.peek() else {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected ')'".to_string(),
                        &token,
                    );
                    return Err(());
                };
                if !SeparatorType::is_separator(&close, SeparatorType::CloseParen) {
                    self.emit_error(
                        TrussDiagnosticCode::ParserError,
                        "Expected ')'".to_string(),
                        &close,
                    );
                    return Err(());
                }
                self.index += 1;
                Ok(Condition::Group(Box::new(inner)))
            }
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::ParserError,
                    format!("Unexpected token in condition: '{}'", token.value),
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
