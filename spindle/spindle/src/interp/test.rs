use crate::interp::Interp;
use crate::interp::value::Value;
use crate::stack::{StackStorage, new_stack};
use crate::testutils::with_test_compile;
use log::info;
use std::assert_matches;
use testing_logger::CapturedLog;

async fn interp(code: &str) {
    with_test_compile(code, async |program| {
        let mut value_stack = heapless::Vec::<Value, 128>::new();
        let mut stack = new_stack::<65536>();
        let mut stack: &mut StackStorage = &mut stack;
        let stack = stack.start();
        let mut interp = Interp::new(program, &mut value_stack);
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
