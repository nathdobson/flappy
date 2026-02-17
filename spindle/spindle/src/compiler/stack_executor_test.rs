use crate::compiler::stack::{StackStorage, new_stack};
use crate::compiler::stack_executor::stack_executor;
use core::sync::atomic::Ordering;
use tokio::task::yield_now;
#[tokio::test]
async fn stack_executor_test() {
    let mut alloc = new_stack::<65536>();
    let mut alloc: &mut StackStorage = &mut alloc;
    let mut alloc = alloc.start();
    {
        let mut did_run = false;
        assert_eq!(
            42,
            stack_executor(alloc.reborrow(), async |spawn| {
                did_run = true;
                42
            })
            .await
            .unwrap(),
        );
        assert!(did_run);
    }
    {
        let mut did_run = 0;
        assert_eq!(
            42,
            stack_executor(alloc.reborrow(), async |spawn| {
                did_run += 1;
                let result = spawn
                    .recurse(async |spawn| {
                        did_run += 1;
                        42
                    })
                    .await
                    .unwrap();
                did_run += 1;
                result
            })
            .await
            .unwrap()
        );
        assert_eq!(did_run, 3);
    }
    {
        let mut did_run = 0;
        assert_eq!(
            42,
            stack_executor(alloc.reborrow(), async |spawn| {
                did_run += 1;
                let result = spawn
                    .recurse(async |spawn| {
                        did_run += 1;
                        let result = spawn
                            .recurse(async |spawn| {
                                did_run += 1;
                                42
                            })
                            .await
                            .unwrap();
                        did_run += 1;
                        result
                    })
                    .await
                    .unwrap();
                did_run += 1;
                result
            })
            .await
            .unwrap()
        );
        assert_eq!(did_run, 5);
    }
    {
        let mut did_run = 0;
        assert_eq!(
            42,
            stack_executor(alloc.reborrow(), async |mut spawn| {
                did_run += 1;
                spawn
                    .reborrow()
                    .recurse(async |spawn| {
                        did_run += 1;
                    })
                    .await
                    .unwrap();
                did_run += 1;
                spawn
                    .reborrow()
                    .recurse(async |spawn| {
                        did_run += 1;
                    })
                    .await
                    .unwrap();
                did_run += 1;
                42
            })
            .await
            .unwrap(),
        );
        assert_eq!(did_run, 5);
    }

    {
        let mut did_run = 0;
        stack_executor(alloc.reborrow(), async |spawn| {
            yield_now().await;
            did_run += 1;
            yield_now().await;
            spawn
                .recurse(async |spawn| {
                    yield_now().await;
                    did_run += 1;
                    yield_now().await;
                    spawn
                        .recurse(async |spawn| {
                            did_run += 1;
                        })
                        .await
                        .unwrap();
                    yield_now().await;
                    did_run += 1;
                    yield_now().await;
                })
                .await
                .unwrap();
            yield_now().await;
            did_run += 1;
            yield_now().await;
        })
        .await
        .unwrap();
        assert_eq!(did_run, 5);
    }
}
