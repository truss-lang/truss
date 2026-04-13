use std::rc::Rc;

use truss::{
    ast::{expression::Expression, statement::Statement},
    id::CrateId,
    krate::Crate,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    types::Type,
};

#[test]
fn test_int32_type_resolver() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() -> Int32 { 1 }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let mut resolver = SymbolResolver::new(Crate::new("test".to_string(), CrateId { id: 0 }));
    resolver.resolve(&program, "test".to_string()).unwrap();
    if let Statement::FunctionDecl { return_type, .. } = &*program.statements[0].borrow()
        && let Expression::Type { ty, .. } = &*return_type.clone().unwrap().borrow()
    {
        assert_eq!(ty.clone().unwrap(), Type::Int32);
    }
}
