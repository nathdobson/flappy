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
    let capacity = 100000usize;
    let mut arena_slice = vec![0; capacity];
    let mut arena = ArenaStorage::new(&mut arena_slice);
    let arena = arena.start();
    let program = Parser::new(Lexer::new(code), arena)
        .parse_program()
        .unwrap();
    println!("{:?}", capacity - arena.remaining());
    assert_matches!(program, Program { stmts: _ });
}
