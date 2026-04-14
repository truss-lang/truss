use std::rc::Rc;

use truss::{
    ast::{expression::Expression, statement::Statement},
    id::CrateId,
    krate::Crate,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
};

#[test]
fn test_variable_resolver() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() { let a = 1 a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let mut resolver = SymbolResolver::new(Crate::new("test".to_string(), CrateId { id: 0 }));
    resolver.resolve(&program, "test".to_string()).unwrap();
    assert!(
        if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
            && let Expression::Block { statements, .. } = &*body.borrow()
            && let Statement::ExpressionStatement { expression } = &*statements[1].borrow()
            && let Expression::Variable { symbol, .. } = &*expression.borrow()
        {
            symbol.is_some()
        } else {
            false
        }
    );
}
