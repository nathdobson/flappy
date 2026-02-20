use arena::Arena;
use crate::compiler::ast::{CallExpr, Expr, ExprList, ForStmt, Program, Stmt};
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use crate::compiler::stack::{StackStorage, new_stack};
use crate::compiler::stack_executor::stack_executor;
use crate::compiler::token::{IdentToken, NumberToken, Token};
use itertools::Itertools;
use std::assert_matches;

async fn expect_program<'src>(code: &'src str, expect: impl for<'a> FnOnce(&'a Program<'src, 'a>)) {
    let capacity = 100000usize;
    let mut arena_slice = vec![0; capacity];
    let mut arena = Arena::new(&mut arena_slice).unwrap();
    let mut stack = new_stack::<65536>();
    let stack: &mut StackStorage = &mut stack;
    let stack = stack.start();
    let program = stack_executor(stack, async |spawn| {
        Parser::new(Lexer::new(code, arena), arena)
            .parse_program(spawn)
            .await
            .unwrap()
    })
    .await
    .unwrap();
    println!("{:?}", capacity - arena.remaining());
    expect(&program);
}

#[tokio::test]
async fn test_parser1() {
    expect_program(
        r#"
        let foo = 2 + 2;
        print(foo);
    "#,
        |p| assert_matches!(p, Program { stmts: _ }),
    )
    .await;
}

#[tokio::test]
async fn test_parser2() {
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
    )
    .await;
}
