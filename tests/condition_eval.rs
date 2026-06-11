use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::statement::Statement,
    condition_eval::{TargetTriple, flatten_program},
    lexer::{CharStream, Lexer},
    parser::Parser,
};

fn create_engine() -> Rc<RefCell<truss::diag::TrussDiagnosticEngine>> {
    Rc::new(RefCell::new(truss::diag::TrussDiagnosticEngine::new()))
}

fn parse(source: &str) -> Vec<Rc<RefCell<Statement>>> {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(source.to_string(), Rc::new("test.truss".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    let mut parser = Parser::new(Rc::new("test.truss".to_string()), tokens, create_engine());
    parser.parse().statements
}

#[test]
fn test_target_triple_parse() {
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    assert_eq!(triple.arch, "x86_64");
    assert_eq!(triple.os, "linux");

    let triple = TargetTriple::parse("aarch64-apple-darwin");
    assert_eq!(triple.arch, "aarch64");
    assert_eq!(triple.os, "darwin");

    let triple = TargetTriple::parse("x86_64-pc-windows-msvc");
    assert_eq!(triple.arch, "x86_64");
    assert_eq!(triple.os, "windows");

    let triple = TargetTriple::parse("wasm32-unknown-unknown");
    assert_eq!(triple.arch, "wasm32");
    assert_eq!(triple.os, "unknown");
}

#[test]
fn test_os_condition_selects_correct_branch() {
    let mut stmts = parse(
        "#if os(linux)
func a() {}
#elseif os(darwin)
func b() {}
#else
func c() {}
#endif",
    );
    assert_eq!(stmts.len(), 1);

    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "a");
    } else {
        panic!("Expected FunctionDecl a");
    }
}

#[test]
fn test_os_condition_selects_darwin_branch() {
    let mut stmts = parse(
        "#if os(linux)
func a() {}
#elseif os(darwin)
func b() {}
#else
func c() {}
#endif",
    );
    assert_eq!(stmts.len(), 1);

    let triple = TargetTriple::parse("aarch64-apple-darwin");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "b");
    } else {
        panic!("Expected FunctionDecl b");
    }
}

#[test]
fn test_os_condition_falls_back_to_else() {
    let mut stmts = parse(
        "#if os(darwin)
func a() {}
#elseif os(windows)
func b() {}
#else
func c() {}
#endif",
    );
    assert_eq!(stmts.len(), 1);

    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "c");
    } else {
        panic!("Expected FunctionDecl c");
    }
}

#[test]
fn test_os_condition_no_match_no_else_removes_block() {
    let mut stmts = parse(
        "#if os(darwin)
func a() {}
#endif",
    );
    assert_eq!(stmts.len(), 1);

    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 0);
}

#[test]
fn test_arch_condition_selects_correct_branch() {
    let mut stmts = parse(
        "#if arch(x86_64)
func a() {}
#elseif arch(aarch64)
func b() {}
#endif",
    );
    assert_eq!(stmts.len(), 1);

    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "a");
    } else {
        panic!("Expected FunctionDecl a");
    }
}

#[test]
fn test_arch_condition_selects_aarch64() {
    let mut stmts = parse(
        "#if arch(x86_64)
func a() {}
#elseif arch(aarch64)
func b() {}
#endif",
    );
    assert_eq!(stmts.len(), 1);

    let triple = TargetTriple::parse("aarch64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "b");
    } else {
        panic!("Expected FunctionDecl b");
    }
}

#[test]
fn test_combined_os_arch_condition() {
    let mut stmts = parse(
        "#if os(linux) && arch(x86_64)
func a() {}
#else
func b() {}
#endif",
    );
    assert_eq!(stmts.len(), 1);

    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "a");
    } else {
        panic!("Expected FunctionDecl a");
    }
}

#[test]
fn test_combined_os_arch_no_match_uses_else() {
    let mut stmts = parse(
        "#if os(linux) && arch(aarch64)
func a() {}
#else
func b() {}
#endif",
    );
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "b");
    } else {
        panic!("Expected FunctionDecl b");
    }
}

#[test]
fn test_not_condition() {
    let mut stmts = parse(
        "#if !os(windows)
func a() {}
#endif",
    );
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "a");
    } else {
        panic!("Expected FunctionDecl a");
    }
}

#[test]
fn test_or_condition() {
    let mut stmts = parse(
        "#if os(linux) || os(darwin)
func a() {}
#endif",
    );
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
}

#[test]
fn test_nested_conditional_blocks() {
    let mut stmts = parse(
        "#if os(linux)
#if arch(x86_64)
func a() {}
#endif
#endif",
    );
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "a");
    } else {
        panic!("Expected FunctionDecl a");
    }
}

#[test]
fn test_outer_inner_condition_eliminates_all() {
    let mut stmts = parse(
        "#if os(darwin)
#if arch(x86_64)
func a() {}
#endif
#endif",
    );
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 0);
}

#[test]
fn test_multiple_statements_in_winning_clause() {
    let mut stmts = parse(
        "#if os(linux)
func a() {}
func b() {}
#else
func c() {}
#endif",
    );
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 2);
    if let Statement::FunctionDecl { name, .. } = &*stmts[0].borrow() {
        assert_eq!(name.value, "a");
    }
    if let Statement::FunctionDecl { name, .. } = &*stmts[1].borrow() {
        assert_eq!(name.value, "b");
    }
}

#[test]
fn test_conditional_inside_function_body() {
    let mut stmts = parse(
        "func foo() {
#if os(linux)
    let x = 1
#else
    let y = 2
#endif
}",
    );
    let triple = TargetTriple::parse("x86_64-unknown-linux-gnu");
    flatten_program(&mut stmts, &triple);

    assert_eq!(stmts.len(), 1);
}
