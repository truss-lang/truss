use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::Expression,
        statement::{FunctionBody, Statement},
    },
    id::CrateId,
    krate::Crate,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    type_resolver::{self, TypeResolver},
    types::TypeKind,
};

#[test]
fn test_variable_resolver() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Int32 { let a = 1 return a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().kind, Some(TypeKind::Int32));
    } else {
        panic!();
    }
}
