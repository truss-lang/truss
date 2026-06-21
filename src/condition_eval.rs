use std::collections::HashMap;
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

    pub fn to_triple_string(&self) -> String {
        if self.arch == "unknown" || self.os.is_empty() {
            "x86_64-unknown-linux-gnu".to_string()
        } else {
            format!("{}-unknown-{}-gnu", self.arch, self.os)
        }
    }
}

pub type DefinedSymbols = HashMap<String, Option<String>>;

pub fn predefined_symbols(file: &str) -> DefinedSymbols {
    let mut symbols = DefinedSymbols::new();
    symbols.insert("__FILE__".to_string(), Some(file.to_string()));
    symbols.insert("__LINE__".to_string(), Some("0".to_string()));
    symbols.insert("__DATE__".to_string(), None);
    symbols.insert("__TIME__".to_string(), None);
    symbols.insert("__TRUSS__".to_string(), Some("1".to_string()));
    symbols
}

fn evaluate_condition(
    cond: &Condition,
    triple: &TargetTriple,
    symbols: &DefinedSymbols,
) -> bool {
    match cond {
        Condition::Bool(b) => *b,
        Condition::Os(token) => token.value == triple.os,
        Condition::Arch(token) => token.value == triple.arch,
        Condition::Defined(token) => symbols.contains_key(&token.value),
        Condition::Platform(_) => false,
        Condition::Not(inner) => !evaluate_condition(inner, triple, symbols),
        Condition::And(left, right) => {
            evaluate_condition(left, triple, symbols) && evaluate_condition(right, triple, symbols)
        }
        Condition::Or(left, right) => {
            evaluate_condition(left, triple, symbols) || evaluate_condition(right, triple, symbols)
        }
        Condition::Group(inner) => evaluate_condition(inner, triple, symbols),
    }
}

fn evaluate_clauses(
    clauses: &[ConditionalClause],
    triple: &TargetTriple,
    symbols: &DefinedSymbols,
) -> Vec<Rc<RefCell<Statement>>> {
    for clause in clauses {
        match clause.condition {
            Some(ref cond) => {
                if evaluate_condition(cond, triple, symbols) {
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

pub fn flatten_program(
    program: &mut Vec<Rc<RefCell<Statement>>>,
    triple: &TargetTriple,
    symbols: &mut DefinedSymbols,
) {
    if !symbols.contains_key("__FILE__") {
        let file = symbols
            .get("__FILE__")
            .cloned()
            .flatten()
            .unwrap_or_default();
        let predef = predefined_symbols(&file);
        for (k, v) in predef {
            symbols.entry(k).or_insert(v);
        }
    }
    flatten_conditional_blocks(program, triple, symbols);
}

fn flatten_conditional_blocks(
    statements: &mut Vec<Rc<RefCell<Statement>>>,
    triple: &TargetTriple,
    symbols: &mut DefinedSymbols,
) {
    let mut i = 0;
    while i < statements.len() {
        {
            let stmt_ref = statements[i].borrow();
            let (is_directive, directive_name, is_define) = match &*stmt_ref {
                Statement::DefineDirective { name, .. } => (true, Some(name.value.clone()), true),
                Statement::UndefDirective { name, .. } => (true, Some(name.value.clone()), false),
                _ => (false, None, false),
            };
            if is_directive {
                drop(stmt_ref);
                if let Some(name) = directive_name {
                    if is_define {
                        symbols.insert(name, None);
                    } else {
                        symbols.remove(&name);
                    }
                }
                statements.remove(i);
                continue;
            }
        }

        flatten_in_statement(&statements[i], triple, symbols);

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

            let winning_body = evaluate_clauses(&clauses, triple, symbols);
            statements.splice(i..=i, winning_body);
            continue;
        }
        i += 1;
    }
}

fn flatten_in_statement(
    stmt: &Rc<RefCell<Statement>>,
    triple: &TargetTriple,
    symbols: &mut DefinedSymbols,
) {
    let mut borrowed = stmt.borrow_mut();
    match &mut *borrowed {
        Statement::FunctionDecl { body, .. }
        | Statement::InitDecl { body, .. }
        | Statement::DeinitDecl { body, .. } => {
            if let FunctionBody::Statements(stmts) = &mut *body.borrow_mut() {
                flatten_conditional_blocks(stmts, triple, symbols);
            }
        }
        Statement::SubscriptDecl { accessors, .. } | Statement::VariableDecl { accessors, .. } => {
            for accessor in accessors {
                flatten_conditional_blocks(&mut accessor.body, triple, symbols);
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
            flatten_conditional_blocks(body, triple, symbols);
        }
        Statement::Guard { else_body, .. } => {
            flatten_conditional_blocks(else_body, triple, symbols);
        }
        Statement::Defer { body, .. } => {
            flatten_conditional_blocks(body, triple, symbols);
        }
        Statement::ExternDecl { statement, .. } => {
            let inner = statement.clone();
            drop(borrowed);
            flatten_in_statement(&inner, triple, symbols);
        }
        Statement::ExpressionStatement { expression } => {
            let inner = expression.clone();
            drop(borrowed);
            flatten_in_expression(&inner, triple, symbols);
        }
        _ => {}
    }
}

fn flatten_in_expression(
    expr: &Rc<RefCell<Expression>>,
    triple: &TargetTriple,
    symbols: &mut DefinedSymbols,
) {
    let mut borrowed = expr.borrow_mut();
    match &mut *borrowed {
        Expression::Match { cases, .. } => {
            for case in cases {
                flatten_conditional_blocks(&mut case.body, triple, symbols);
            }
        }
        _ => {}
    }
}
