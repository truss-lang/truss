use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::{
        expression::{Expression, MacroDelimiter},
        node::Program,
        statement::{MacroPatternFragment, Statement},
    },
    diag::TrussDiagnosticEngine,
    lexer::token::{OperatorType, Token, TokenType},
    parser::Parser,
};

#[derive(Debug)]
pub struct MacroExpander;

impl MacroExpander {
    pub fn new(_engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        Self
    }

    pub fn expand(&mut self, program: &mut Program) {
        let macros = self.collect_macros(program);
        if macros.is_empty() {
            return;
        }
        self.expand_stmts(&mut program.statements, &macros);
    }

    fn collect_macros(
        &self,
        program: &Program,
    ) -> HashMap<String, Vec<crate::ast::statement::MacroArm>> {
        let mut macros = HashMap::new();
        for stmt in &program.statements {
            if let Statement::MacroDecl { name, arms, .. } = &*stmt.borrow() {
                macros.insert(name.value.clone(), arms.clone());
            }
        }
        macros
    }

    fn expand_stmts(
        &mut self,
        stmts: &mut Vec<Rc<RefCell<Statement>>>,
        macros: &HashMap<String, Vec<crate::ast::statement::MacroArm>>,
    ) {
        let expansions: Vec<(usize, Vec<Rc<RefCell<Statement>>>)> = (0..stmts.len())
            .filter_map(|i| {
                let expanded = self.try_expand_statement(stmts[i].clone(), macros);
                expanded.map(|new_stmts| (i, new_stmts))
            })
            .collect();

        for (idx, new_stmts) in expansions.into_iter().rev() {
            stmts.splice(idx..=idx, new_stmts);
        }

        let mut i = 0;
        while i < stmts.len() {
            self.expand_nested_statement(stmts[i].clone(), macros);
            i += 1;
        }
    }

    fn try_expand_statement(
        &mut self,
        stmt: Rc<RefCell<Statement>>,
        macros: &HashMap<String, Vec<crate::ast::statement::MacroArm>>,
    ) -> Option<Vec<Rc<RefCell<Statement>>>> {
        let s = stmt.borrow();
        match &*s {
            Statement::ExpressionStatement { expression } => {
                let expr = expression.clone();
                drop(s);
                let expanded = self.try_expand_expr(expr, macros);
                expanded.map(|new_expr| {
                    vec![Rc::new(RefCell::new(Statement::ExpressionStatement {
                        expression: Rc::new(RefCell::new(new_expr)),
                    }))]
                })
            }
            _ => None,
        }
    }

    fn expand_nested_statement(
        &mut self,
        stmt: Rc<RefCell<Statement>>,
        macros: &HashMap<String, Vec<crate::ast::statement::MacroArm>>,
    ) {
        let mut stmt_mut = stmt.borrow_mut();
        match &mut *stmt_mut {
            Statement::FunctionDecl { body, .. } => {
                let body_mut = &mut *body.borrow_mut();
                match body_mut {
                    crate::ast::statement::FunctionBody::Statements(inner) => {
                        self.expand_stmts(inner, macros);
                    }
                    crate::ast::statement::FunctionBody::Expression(expr) => {
                        let expanded = self.try_expand_expr_owned(expr, macros);
                        if let Some(new_expr) = expanded {
                            *expr = Rc::new(RefCell::new(new_expr));
                        }
                    }
                    crate::ast::statement::FunctionBody::None => {}
                }
            }
            Statement::StructDecl { body, .. }
            | Statement::ClassDecl { body, .. }
            | Statement::EnumDecl { body, .. }
            | Statement::ExtensionDecl { body, .. }
            | Statement::ModuleDecl { body, .. } => {
                self.expand_stmts(body, macros);
            }
            _ => {}
        }
    }

    fn try_expand_expr(
        &self,
        expr: Rc<RefCell<Expression>>,
        macros: &HashMap<String, Vec<crate::ast::statement::MacroArm>>,
    ) -> Option<Expression> {
        let clone = expr.borrow().clone();
        if let Expression::MacroInvocation {
            name,
            delimiter,
            arguments,
            ..
        } = &clone
        {
            if let Some(arms) = macros.get(&name.value) {
                return Self::try_expand_macro(name, arms, *delimiter, arguments);
            }
        }
        Self::expand_expr_children_raw(expr, macros);
        None
    }

    fn try_expand_expr_owned(
        &self,
        expr: &mut Rc<RefCell<Expression>>,
        macros: &HashMap<String, Vec<crate::ast::statement::MacroArm>>,
    ) -> Option<Expression> {
        let clone = expr.borrow().clone();
        if let Expression::MacroInvocation {
            name,
            delimiter,
            arguments,
            ..
        } = &clone
        {
            if let Some(arms) = macros.get(&name.value) {
                return Self::try_expand_macro(name, arms, *delimiter, arguments);
            }
        }
        Self::expand_expr_children_raw(expr.clone(), macros);
        None
    }

    fn expand_expr_children_raw(
        expr: Rc<RefCell<Expression>>,
        macros: &HashMap<String, Vec<crate::ast::statement::MacroArm>>,
    ) {
        let children: Vec<Rc<RefCell<Expression>>> = {
            let e = expr.borrow();
            match &*e {
                Expression::Binary { left, right, .. } => vec![left.clone(), right.clone()],
                Expression::Unary { expression, .. } => vec![expression.clone()],
                Expression::Call {
                    callee, parameters, ..
                } => {
                    let mut v = vec![callee.clone()];
                    for p in parameters {
                        v.push(p.expression.clone());
                    }
                    v
                }
                Expression::MemberAccess { object, .. } => vec![object.clone()],
                Expression::Assignment { left, right, .. } => vec![left.clone(), right.clone()],
                Expression::SubscriptAccess { object, parameters, .. } => {
                    let mut v = vec![object.clone()];
                    for p in parameters {
                        v.push(p.expression.clone());
                    }
                    v
                }
                _ => vec![],
            }
        };
        for child in children {
            let clone = child.borrow().clone();
            if let Expression::MacroInvocation {
                name,
                delimiter,
                arguments,
                ..
            } = &clone
            {
                if let Some(arms) = macros.get(&name.value) {
                    if let Some(expanded) =
                        Self::try_expand_macro(name, arms, *delimiter, arguments)
                    {
                        let mut child_mut = child.borrow_mut();
                        let _ = std::mem::replace(&mut *child_mut, expanded);
                    }
                }
            }
        }
    }

    fn try_expand_macro(
        name_token: &Token,
        arms: &[crate::ast::statement::MacroArm],
        _delimiter: MacroDelimiter,
        arguments: &[Token],
    ) -> Option<Expression> {
        for arm in arms {
            if let Some(captures) = Self::match_pattern(&arm.pattern, arguments) {
                let expanded_tokens = Self::substitute(&arm.expansion, &captures);
                let result = Self::parse_expanded_tokens(expanded_tokens, name_token);
                return result;
            }
        }
        None
    }

    fn match_pattern(
        pattern: &[MacroPatternFragment],
        arguments: &[Token],
    ) -> Option<HashMap<String, Vec<Token>>> {
        let mut captures: HashMap<String, Vec<Token>> = HashMap::new();
        let mut arg_idx = 0;
        let mut pat_idx = 0;

        while pat_idx < pattern.len() {
            let fragment = &pattern[pat_idx];
            match fragment {
                MacroPatternFragment::Lit(expected) => {
                    if arg_idx >= arguments.len()
                        || !Self::tokens_match(expected, &arguments[arg_idx])
                    {
                        return None;
                    }
                    arg_idx += 1;
                    pat_idx += 1;
                }
                MacroPatternFragment::MetaVar { name, var_type } => {
                    if arg_idx >= arguments.len() {
                        return None;
                    }
                    let remaining = &arguments[arg_idx..];
                    let captured = match var_type {
                        crate::ast::statement::MacroMetaVarType::Ident
                        | crate::ast::statement::MacroMetaVarType::Literal => {
                            vec![remaining[0].clone()]
                        }
                        _ => {
                            if let Some(MacroPatternFragment::Lit(next_lit)) =
                                pattern.get(pat_idx + 1)
                            {
                                let mut tokens = Vec::new();
                                for t in remaining {
                                    if Self::tokens_match(next_lit, t) {
                                        break;
                                    }
                                    tokens.push(t.clone());
                                }
                                if tokens.is_empty() {
                                    return None;
                                }
                                tokens
                            } else {
                                remaining.to_vec()
                            }
                        }
                    };
                    if captured.is_empty() {
                        return None;
                    }
                    let captured_len = captured.len();
                    captures.insert(name.clone(), captured);
                    arg_idx += captured_len;
                    pat_idx += 1;
                }
            }
        }

        if arg_idx != arguments.len() {
            return None;
        }

        Some(captures)
    }

    fn tokens_match(a: &Token, b: &Token) -> bool {
        a.ty == b.ty && a.value == b.value
    }

    fn substitute(
        expansion: &[Token],
        captures: &HashMap<String, Vec<Token>>,
    ) -> Vec<Token> {
        let mut result = Vec::new();
        let mut i = 0;
        while i < expansion.len() {
            if i + 1 < expansion.len()
                && OperatorType::is_operator(&expansion[i], OperatorType::Dollar)
                && matches!(expansion[i + 1].ty, TokenType::Identifier)
            {
                let name = &expansion[i + 1].value;
                if let Some(tokens) = captures.get(name) {
                    result.extend(tokens.clone());
                }
                i += 2;
            } else {
                result.push(expansion[i].clone());
                i += 1;
            }
        }
        result
    }

    fn parse_expanded_tokens(tokens: Vec<Token>, name_token: &Token) -> Option<Expression> {
        if tokens.is_empty() {
            return None;
        }
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut parser = Parser::new(name_token.file.clone(), tokens, engine);
        let mut program = parser.parse();
        if program.statements.is_empty() {
            return None;
        }
        if let Statement::ExpressionStatement { expression } =
            &*program.statements.remove(0).borrow()
        {
            Some(expression.borrow().clone())
        } else {
            None
        }
    }
}
