use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;
use arena::ArenaStorage;
use std::assert_matches;

#[test]
fn test_parser() {
    use itertools::Itertools;

    let code = r#"
        let foo = 2 + 2;
        print(foo);
    "#;
    const CAP: usize = 100000;
    let mut arena = Box::new(ArenaStorage::<CAP>::new());
    let arena = arena.start();
    let program = Parser::new(Lexer::new(code), arena)
        .parse_program()
        .unwrap();
    println!("{:?}", CAP - arena.remaining());
    assert_matches!(program, Program { stmts: _ });
}
