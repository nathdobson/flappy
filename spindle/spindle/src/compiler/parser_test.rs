use crate::compiler::ast::{CallExpr, Expr, ExprList, ForStmt, Program, Stmt};
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use crate::compiler::token::{IdentToken, NumberToken, Token};
use arena::ArenaStorage;
use itertools::Itertools;
use std::assert_matches;

fn expect_program(code: &str, expect: impl for<'a> FnOnce(&'a Program<'a>)) {
    let capacity = 100000usize;
    let mut arena_slice = vec![0; capacity];
    let mut arena = ArenaStorage::new(&mut arena_slice);
    let arena = arena.start();
    let program = Parser::new(Lexer::new(code, arena), arena)
        .parse_program()
        .unwrap();
    println!("{:?}", capacity - arena.remaining());
    expect(&program);
}

#[test]
fn test_parser1() {
    expect_program(
        r#"
        let foo = 2 + 2;
        print(foo);
    "#,
        |p| assert_matches!(p, Program { stmts: _ }),
    );
}

#[test]
fn test_parser2() {
    expect_program(
        r#"
        for x in 0..10 {
            print(x);
        }
    "#,
        |p| {
            assert_matches!(
                p,
                Program {
                    stmts: [Stmt::For(ForStmt {
                        for_token: _,
                        ident: IdentToken { ident: "x", loc: _ },
                        init_expr: Expr::Number(NumberToken {
                            number: "0",
                            loc: _
                        }),
                        limit_expr: Expr::Number(NumberToken {
                            number: "10",
                            loc: _
                        }),
                        open_brace: _,
                        inner: [Stmt::ExprStmt(Expr::Call(CallExpr {
                            callee: Expr::Var(IdentToken {
                                ident: "print",
                                loc: _
                            }),
                            lparen: _,
                            args: ExprList {
                                exprs: [Expr::Var(IdentToken { ident: "x", loc: _ })],
                                commas: []
                            },
                            rparen: _
                        }))],
                        close_brace: _,
                    })]
                }
            )
        },
    );
}
