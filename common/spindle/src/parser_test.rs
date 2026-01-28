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
    let mut arena = ArenaStorage::<1024>::new();
    let arena = arena.start();
    let program = Parser::new(Lexer::new(code), arena)
        .parse_program()
        .unwrap();
    assert_matches!(program, Program { stmts: _ });
}
