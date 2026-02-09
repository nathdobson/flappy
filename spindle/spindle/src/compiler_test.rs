use crate::ast::CallExpr;
use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::testutils::with_test_compile;
use crate::vm::{
    VmCallExpr, VmExpr, VmExprStmt, VmForStmt, VmFunction, VmFunctionName, VmLetStmt, VmOperator,
    VmOperatorExpr, VmProgram, VmStmt,
};
use arena::ArenaStorage;
use std::assert_matches;

#[tokio::test]
async fn test_parser() {
    use itertools::Itertools;

    let code = r#"
        let foo = 2 + 2;
        print(foo);
        print(foo);
    "#;
    with_test_compile(code, async |program| {
        assert_matches!(
            program,
            VmProgram {
                functions: [VmFunction {
                    stmt: VmStmt::LetStmt(VmLetStmt {
                        expr: VmExpr::Operator(VmOperatorExpr {
                            operator: VmOperator::Plus,
                            left: VmExpr::Number(2),
                            right: VmExpr::Number(2)
                        }),
                        next: VmStmt::ExprStmt(VmExprStmt {
                            expr: VmExpr::Call(VmCallExpr {
                                function: VmFunctionName::Print,
                                args: [VmExpr::Var(0)],
                            }),
                            next: VmStmt::ExprStmt(VmExprStmt {
                                expr: VmExpr::Call(VmCallExpr {
                                    function: VmFunctionName::Print,
                                    args: [VmExpr::Var(0)],
                                }),
                                next: VmStmt::Noop,
                            }),
                        })
                    })
                }]
            }
        );
    })
    .await;
}

#[tokio::test]
async fn test_for_loop() {
    use itertools::Itertools;

    let code = r#"
        for x in 0..10{
            print(x);
        }
    "#;
    with_test_compile(code, async |program| {
        assert_matches!(
            program,
            VmProgram {
                functions: [VmFunction {
                    stmt: VmStmt::ForStmt(VmForStmt {
                        init: VmExpr::Number(0),
                        limit: VmExpr::Number(10),
                        inner: VmStmt::ExprStmt(VmExprStmt {
                            expr: VmExpr::Call(VmCallExpr {
                                function: VmFunctionName::Print,
                                args: [VmExpr::Var(0)],
                            }),
                            next: VmStmt::Noop,
                        }),
                    })
                }]
            }
        );
    })
    .await;
}
