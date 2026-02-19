use crate::AnnotatedParserError;
use crate::SpindleError;
use crate::compiler::parser::ParserError;
use crate::compiler::stack::{StackStorage, new_stack};
use crate::interp::Interp;
use crate::interp::heap::HeapStorage;
use crate::interp::value::Value;
use crate::native::{NativeFn, PrintFn};
use crate::testutils::interp;
use heapless::String;
use log::info;
use std::assert_matches;

#[tokio::test]
async fn test_interp() {
    let result = interp(
        r#"
        let foo = 2 + 2;
        print(foo);
        print(foo);
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["4", _]);
}

#[tokio::test]
async fn test_for_loop() {
    let result = interp(
        r#"
        for x in 10..13{
            print(x);
        }
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["10", "11", "12",]);
}

#[tokio::test]
async fn test_if_stmt() {
    let result = interp(
        r#"
        if false {
            print(10);
        }
        if true {
            print(20);
        }
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["20"]);
}

#[tokio::test]
async fn test_if_else_stmt() {
    let result = interp(
        r#"
        if false {
            print(10);
        }else{
            print(11);
        }
        if true {
            print(20);
        }else{
            print(21);
        }
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["11", "20",]);
}

#[tokio::test]
async fn test_if_else_if_stmt() {
    let result = interp(
        r#"
        if false {
            print(10);
        }else if false {
            print(11);
        } else{
            print(12);
        }
        if false {
            print(20);
        }else if true {
            print(21);
        } else{
            print(22);
        }
        if true {
            print(30);
        }else if true {
            print(31);
        } else{
            print(32);
        }
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["12", "21", "30",]);
}

#[tokio::test]
async fn test_string_literal() {
    let result = interp(
        r#"
        print("hi ", 2, " all");
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["hi 2 all"]);
}

#[tokio::test]
async fn test_two_argument() {
    let result = interp(
        r#"
        let a = 1;
        let b = 2;
        print(a, b);
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["12"]);
}

#[tokio::test]
async fn test_loop() {
    let result = interp(
        r#"
        let a = 1;
        loop {
            if a > 3 {
               break;
            }
            print(a);
            a = a + 1;
        }
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["1", "2", "3"]);
}

#[tokio::test]
async fn test_while() {
    let result = interp(
        r#"
        let a = 1;
        while a < 3 {
            print(a);
            a = a + 1;
        }
       "#,
    )
    .await
    .unwrap();
    assert_matches!(result, ["1", "2",]);
}

#[tokio::test]
async fn test_recursive_parsing() {
    let mut code = "1".to_string();
    for x in 0..1000 {
        code.push_str("+1");
    }
    code.push_str(";");
    let result = interp(&code).await;
    assert_matches!(
        result,
        Err(SpindleError::ParserError(AnnotatedParserError {
            cause: ParserError::AllocError,
            ..
        }))
    );
}

#[tokio::test]
async fn test_continue() {
    let result = interp(
        r#"
        let a = 1;
        loop {
            if a < 3 {
                a = a + 1;
                continue;
            }
            print(a);
            break;
        }
       "#,
    )
        .await
        .unwrap();
    assert_matches!(result, ["3"]);
}

#[tokio::test]
async fn test_not() {
    let result = interp(
        r#"
        print(!false);
        print(!true);
        print(!null);
       "#,
    )
        .await
        .unwrap();
    assert_matches!(result, ["true","false","true"]);
}

#[tokio::test]
async fn test_neg() {
    let result = interp(
        r#"
        print(10 + - 5);
       "#,
    )
        .await
        .unwrap();
    assert_matches!(result, ["5"]);
}
