use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::statement::Statement,
    diag::TrussDiagnosticEngine,
    lexer::{CharStream, Lexer},
    macro_expander::MacroExpander,
    parser::Parser,
};

fn create_engine() -> Rc<RefCell<TrussDiagnosticEngine>> {
    Rc::new(RefCell::new(TrussDiagnosticEngine::new()))
}

fn parse_and_expand(
    source: &str,
) -> (
    Vec<Rc<RefCell<Statement>>>,
    Rc<RefCell<TrussDiagnosticEngine>>,
) {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(source.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let file = lexer.get_file();
    let mut parser = Parser::new(file, tokens, engine.clone());
    let mut program = parser.parse();
    let mut expander = MacroExpander::new(engine.clone());
    expander.expand(&mut program);
    (program.statements, engine)
}

#[test]
fn test_expand_simple_macro() {
    let (stmts, _) = parse_and_expand("macro id { ($x:expr) => { $x } }\nid!(42)");
    assert_eq!(stmts.len(), 2);
    assert!(matches!(&*stmts[0].borrow(), Statement::MacroDecl { .. }));
    assert!(matches!(
        &*stmts[1].borrow(),
        Statement::ExpressionStatement { .. }
    ));
}

#[test]
fn test_expand_macro_multiple_args() {
    let (stmts, _) =
        parse_and_expand("macro add { ($a:expr, $b:expr) => { $a + $b } }\nadd!(1, 2)");
    assert_eq!(stmts.len(), 2);
    assert!(matches!(&*stmts[0].borrow(), Statement::MacroDecl { .. }));
}

#[test]
fn test_expand_macro_in_expression_statement() {
    let (stmts, _) = parse_and_expand("macro id { ($x:expr) => { $x } }\nlet a = id!(42)");
    assert_eq!(stmts.len(), 2);
    assert!(matches!(&*stmts[0].borrow(), Statement::MacroDecl { .. }));
    if let Statement::VariableDecl { initializer, .. } = &*stmts[1].borrow() {
        assert!(initializer.is_some());
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_expand_macro_in_function() {
    let source = "macro id { ($x:expr) => { $x } }\nfunc test() -> Int32 { id!(42) }";
    let (stmts, _) = parse_and_expand(source);
    assert_eq!(stmts.len(), 2);
    assert!(matches!(&*stmts[0].borrow(), Statement::MacroDecl { .. }));
    if let Statement::FunctionDecl { body, .. } = &*stmts[1].borrow() {
        let body_ref = body.borrow();
        match &*body_ref {
            truss::ast::statement::FunctionBody::Expression(expr) => {
                assert!(!matches!(
                    &*expr.borrow(),
                    truss::ast::expression::Expression::MacroInvocation { .. }
                ));
            }
            truss::ast::statement::FunctionBody::Statements(inner) => {
                assert!(!inner.is_empty());
                if let Statement::ExpressionStatement { expression } = &*inner[0].borrow() {
                    assert!(!matches!(
                        &*expression.borrow(),
                        truss::ast::expression::Expression::MacroInvocation { .. }
                    ));
                }
            }
            truss::ast::statement::FunctionBody::None => {
                panic!("Expected non-empty function body");
            }
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_macro_symbold_declared() {
    let source = "macro id { ($x:expr) => { $x } }\nid!(42)";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(source.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let file = lexer.get_file();
    let mut parser = Parser::new(file, tokens, engine.clone());
    let mut program = parser.parse();

    let mut expander = MacroExpander::new(engine.clone());
    expander.expand(&mut program);

    let krate = Rc::new(RefCell::new(truss::krate::Package::new("test".to_string())));
    let mut resolver = truss::symbol_resolver::SymbolResolver::new(krate.clone(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let scope = krate
        .borrow()
        .modules
        .get("test")
        .unwrap()
        .borrow()
        .scope
        .clone()
        .unwrap();
    let symbol = scope.borrow().get_symbol("id");
    assert!(symbol.is_some());
    if let Some(sym) = symbol {
        assert!(matches!(
            &*sym.borrow(),
            truss::symbol::Symbol::Macro { .. }
        ));
    }
}
