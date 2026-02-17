use crate::compiler::stack::{StackStorage, new_stack};
use crate::interp::Interp;
use crate::interp::heap::HeapStorage;
use crate::interp::value::Value;
use crate::native::{NativeFn, PrintFn};
use crate::testutils::with_test_compile;
use heapless::String;
use log::info;
use std::assert_matches;
use testing_logger::CapturedLog;

async fn interp(code: &str) {
    with_test_compile(code, async |program| {
        let mut value_stack = heapless::Vec::<Value, 128>::new();
        let mut stack = new_stack::<65536>();
        let mut stack: &mut StackStorage = &mut stack;
        let stack = stack.start();
        let mut heap_storage = HeapStorage::<1024, 65536>::new();
        let mut interp = Interp::new(program, &mut value_stack, heap_storage.start(), &[&PrintFn]);
        interp.interp(stack).await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn test_interp() {
    testing_logger::setup();
    interp(
        r#"
        let foo = 2 + 2;
        print(foo);
        print(foo);
       "#,
    )
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [
            CapturedLog {
                body: "4",
                level: _,
                target: _
            },
            _
        ]
    );
}

#[tokio::test]
async fn test_for_loop() {
    testing_logger::setup();
    interp(
        r#"
        for x in 10..13{
            print(x);
        }
       "#,
    )
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [
            CapturedLog {
                body: "10",
                level: _,
                target: _
            },
            CapturedLog {
                body: "11",
                level: _,
                target: _
            },
            CapturedLog {
                body: "12",
                level: _,
                target: _
            },
        ]
    );
}

#[tokio::test]
async fn test_if_stmt() {
    testing_logger::setup();
    interp(
        r#"
        if false {
            print(10);
        }
        if true {
            print(20);
        }
       "#,
    )
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [CapturedLog {
            body: "20",
            level: _,
            target: _
        },]
    );
}

#[tokio::test]
async fn test_if_else_stmt() {
    testing_logger::setup();
    interp(
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
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [
            CapturedLog {
                body: "11",
                level: _,
                target: _
            },
            CapturedLog {
                body: "20",
                level: _,
                target: _
            },
        ]
    );
}

#[tokio::test]
async fn test_if_else_if_stmt() {
    testing_logger::setup();
    interp(
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
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [
            CapturedLog {
                body: "12",
                level: _,
                target: _
            },
            CapturedLog {
                body: "21",
                level: _,
                target: _
            },
            CapturedLog {
                body: "30",
                level: _,
                target: _
            },
        ]
    );
}

#[tokio::test]
async fn test_string_literal() {
    testing_logger::setup();
    interp(
        r#"
        print("hi ", 2, " all");
       "#,
    )
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [CapturedLog {
            body: "hi 2 all",
            level: _,
            target: _
        },]
    );
}

#[tokio::test]
async fn test_two_argument() {
    testing_logger::setup();
    interp(
        r#"
        let a = 1;
        let b = 2;
        print(a, b);
       "#,
    )
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [CapturedLog {
            body: "12",
            level: _,
            target: _
        },]
    );
}

#[tokio::test]
async fn test_loop() {
    testing_logger::setup();
    interp(
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
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [
            CapturedLog {
                body: "1",
                level: _,
                target: _
            },
            CapturedLog {
                body: "2",
                level: _,
                target: _
            },
            CapturedLog {
                body: "3",
                level: _,
                target: _
            },
        ]
    );
}

#[tokio::test]
async fn test_while() {
    testing_logger::setup();
    interp(
        r#"
        let a = 1;
        while a < 3 {
            print(a);
            a = a + 1;
        }
       "#,
    )
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [
            CapturedLog {
                body: "1",
                level: _,
                target: _
            },
            CapturedLog {
                body: "2",
                level: _,
                target: _
            },
        ]
    );
}

// #[tokio::test]
// async fn test_recursive_parsing() {
//     testing_logger::setup();
//     let mut code = "1".to_string();
//     for x in 0..1000 {
//         code.push_str("+1");
//     }
//     code.push_str(";");
//     interp(&code).await;
//     assert_matches!(testing_logger::take()[..], []);
// }
