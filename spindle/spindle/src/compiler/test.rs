use crate::Program;
use crate::Spindle;
use crate::compiler::ast::Expr;
use crate::compiler::ast::InfixExpr;
use crate::compiler::ast::Stmt;
use crate::compiler::token::NumberToken;
use crate::compiler::token::Symbol;
use crate::compiler::token::SymbolToken;
use crate::testutils::TestSpindle;
use std::assert_matches;

macro_rules! plus {
    ($p1: pat, $p2: pat) => {
        Expr::InfixExpr(InfixExpr {
            left: $p1,
            symbol: SymbolToken {
                symbol: Symbol::Plus,
                ..
            },
            right: $p2,
        })
    };
}

macro_rules! times {
    ($p1: pat, $p2: pat) => {
        Expr::InfixExpr(InfixExpr {
            left: $p1,
            symbol: SymbolToken {
                symbol: Symbol::Times,
                ..
            },
            right: $p2,
        })
    };
}

macro_rules! less {
    ($p1: pat, $p2: pat) => {
        Expr::InfixExpr(InfixExpr {
            left: $p1,
            symbol: SymbolToken {
                symbol: Symbol::Less,
                ..
            },
            right: $p2,
        })
    };
}

macro_rules! and {
    ($p1: pat, $p2: pat) => {
        Expr::InfixExpr(InfixExpr {
            left: $p1,
            symbol: SymbolToken {
                symbol: Symbol::AndAnd,
                ..
            },
            right: $p2,
        })
    };
}

macro_rules! number {
    ($p1: pat) => {
        Expr::Number(NumberToken { number: $p1, .. })
    };
}

macro_rules! assert_expr {
    ($program: ident, $p: pat) => {
        assert_matches!(
            $program,
            Program {
                stmts: [Stmt::ExprStmt($p)]
            }
        );
    };
}

#[tokio::test]
async fn test_add() {
    let mut spindle = TestSpindle::new();
    let program = spindle.start().parse(r#"1+2;"#).await.unwrap();
    assert_expr!(program, plus!(number!("1"), number!("2")));
}

#[tokio::test]
async fn test_add_mul() {
    let mut spindle = TestSpindle::new();
    let program = spindle.start().parse(r#"1+2*3;"#).await.unwrap();
    assert_expr!(
        program,
        plus!(number!("1"), times!(number!("2"), number!("3")))
    );
}

#[tokio::test]
async fn test_mul_add() {
    let mut spindle = TestSpindle::new();
    let program = spindle.start().parse(r#"1*2+3;"#).await.unwrap();
    assert_expr!(
        program,
        plus!(times!(number!("1"), number!("2")), number!("3"))
    );
}

#[tokio::test]
async fn test_mul_add_less() {
    let mut spindle = TestSpindle::new();
    let program = spindle.start().parse(r#"1*2+3<4;"#).await.unwrap();
    assert_expr!(
        program,
        less!(
            plus!(times!(number!("1"), number!("2")), number!("3")),
            number!("4")
        )
    );
}
#[tokio::test]
async fn test_less_and_less() {
    let mut spindle = TestSpindle::new();
    let program = spindle.start().parse(r#"1<2 && 3<4;"#).await.unwrap();
    assert_expr!(
        program,
        and!(
            less!(number!("1"), number!("2")),
            less!(number!("3"), number!("4"))
        )
    );
}
