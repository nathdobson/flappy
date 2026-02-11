use crate::interp::Interp;
use crate::interp::heap::HeapStorage;
use crate::interp::value::Value;
use crate::native::{NativeFn, PrintFn};
use crate::stack::{StackStorage, new_stack};
use crate::testutils::{with_test_compile, TEST_NATIVES};
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
        let mut interp = Interp::new(
            program,
            &mut value_stack,
            heap_storage.start(),
            TEST_NATIVES,
        );
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
        print("hi");
       "#,
    )
    .await;
    assert_matches!(
        testing_logger::take()[..],
        [CapturedLog {
            body: "hi",
            level: _,
            target: _
        },]
    );
}
