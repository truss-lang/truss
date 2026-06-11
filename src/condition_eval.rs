use std::{cell::RefCell, rc::Rc};

use crate::ast::{
    expression::Expression,
    statement::{Condition, ConditionalClause, FunctionBody, Statement},
};

#[derive(Debug, Clone, PartialEq)]
pub struct TargetTriple {
    pub arch: String,
    pub os: String,
}

impl TargetTriple {
    pub fn parse(triple: &str) -> Self {
        let parts: Vec<&str> = triple.split('-').collect();
        let arch = parts.first().map(|s| s.to_string()).unwrap_or_default();
        let os = if parts.len() >= 4 {
            parts[2].to_string()
        } else if parts.len() >= 3 {
            parts[2].to_string()
        } else {
            String::new()
        };
        TargetTriple { arch, os }
    }

    pub fn host() -> Self {
        match option_env!("HOST") {
            Some(triple) => TargetTriple::parse(triple),
            None => TargetTriple::parse("unknown-unknown-unknown"),
        }
    }
}

fn evaluate_condition(cond: &Condition, triple: &TargetTriple) -> bool {
    match cond {
        Condition::Bool(b) => *b,
        Condition::Os(token) => token.value == triple.os,
        Condition::Arch(token) => token.value == triple.arch,
        Condition::Defined(_) | Condition::Platform(_) => false,
        Condition::Not(inner) => !evaluate_condition(inner, triple),
        Condition::And(left, right) => {
            evaluate_condition(left, triple) && evaluate_condition(right, triple)
        }
        Condition::Or(left, right) => {
            evaluate_condition(left, triple) || evaluate_condition(right, triple)
        }
        Condition::Group(inner) => evaluate_condition(inner, triple),
    }
}

fn evaluate_clauses(
    clauses: &[ConditionalClause],
    triple: &TargetTriple,
) -> Vec<Rc<RefCell<Statement>>> {
    for clause in clauses {
        match clause.condition {
            Some(ref cond) => {
                if evaluate_condition(cond, triple) {
                    return clause.body.clone();
                }
            }
            None => {
                return clause.body.clone();
            }
        }
    }
    Vec::new()
}

pub fn flatten_program(program: &mut Vec<Rc<RefCell<Statement>>>, triple: &TargetTriple) {
    flatten_conditional_blocks(program, triple);
}

fn flatten_conditional_blocks(
    statements: &mut Vec<Rc<RefCell<Statement>>>,
    triple: &TargetTriple,
) {
    let mut i = 0;
    while i < statements.len() {
        flatten_in_statement(&statements[i], triple);

        let is_conditional = {
            let stmt = statements[i].borrow();
            matches!(&*stmt, Statement::ConditionalBlock { .. })
        };

        if is_conditional {
            let clauses = {
                let stmt = statements[i].borrow();
                match &*stmt {
                    Statement::ConditionalBlock { clauses } => clauses.clone(),
                    _ => unreachable!(),
                }
            };

            let winning_body = evaluate_clauses(&clauses, triple);
            statements.splice(i..=i, winning_body);
            continue;
        }
        i += 1;
    }
}

fn flatten_in_statement(stmt: &Rc<RefCell<Statement>>, triple: &TargetTriple) {
    let mut borrowed = stmt.borrow_mut();
    match &mut *borrowed {
        Statement::FunctionDecl { body, .. }
        | Statement::InitDecl { body, .. }
        | Statement::DeinitDecl { body, .. } => {
            if let FunctionBody::Statements(stmts) = &mut *body.borrow_mut() {
                flatten_conditional_blocks(stmts, triple);
            }
        }
        Statement::SubscriptDecl { accessors, .. } | Statement::VariableDecl { accessors, .. } => {
            for accessor in accessors {
                flatten_conditional_blocks(&mut accessor.body, triple);
            }
        }
        Statement::StructDecl { body, .. }
        | Statement::ClassDecl { body, .. }
        | Statement::EnumDecl { body, .. }
        | Statement::ExtensionDecl { body, .. }
        | Statement::ModuleDecl { body, .. }
        | Statement::Loop { body, .. }
        | Statement::While { body, .. }
        | Statement::RepeatWhile { body, .. }
        | Statement::For { body, .. }
        | Statement::ExternBlock { items: body, .. } => {
            flatten_conditional_blocks(body, triple);
        }
        Statement::Guard { else_body, .. } => {
            flatten_conditional_blocks(else_body, triple);
        }
        Statement::Defer { body, .. } => {
            flatten_conditional_blocks(body, triple);
        }
        Statement::ExternDecl { statement, .. } => {
            let inner = statement.clone();
            drop(borrowed);
            flatten_in_statement(&inner, triple);
        }
        Statement::ExpressionStatement { expression } => {
            let inner = expression.clone();
            drop(borrowed);
            flatten_in_expression(&inner, triple);
        }
        _ => {}
    }
}

fn flatten_in_expression(expr: &Rc<RefCell<Expression>>, triple: &TargetTriple) {
    let mut borrowed = expr.borrow_mut();
    match &mut *borrowed {
        Expression::Match { cases, .. } => {
            for case in cases {
                flatten_conditional_blocks(&mut case.body, triple);
            }
        }
        _ => {}
    }
}
