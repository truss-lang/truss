use std::rc::Rc;

use truss::{
    ast::statement::Statement,
    lexer::{CharStream, Lexer},
    parser::Parser,
};

#[test]
fn test_parse_function_decl() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() -> UInt { 1 } func test2() {}".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { name, .. } = &*program.statements[0] {
        assert_eq!(name.value, "test");
    }
    if let Statement::FunctionDecl { name, .. } = &*program.statements[1] {
        assert_eq!(name.value, "test2");
    }
}
