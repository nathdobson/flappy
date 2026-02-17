use crate::compiler::stack::{Stack, StackStorage, new_stack};
use core::alloc::Layout;

#[test]
fn test() {
    let mut stack = new_stack::<1024>();
    let stack: &mut StackStorage = &mut stack;
    let mut s = stack.start();
    let (mut s, a1) = s.push(Layout::new::<String>()).unwrap();
    let a1 = a1.init("hello".to_string()).unwrap();
    let (mut s, a2) = s.push(Layout::new::<String>()).unwrap();
    let a2 = a2.init("world".to_string()).unwrap();
    assert_eq!(*a1, "hello");
    assert_eq!(*a2, "world");
}
async fn recurse(mut stack: Stack<'_>) -> usize {
    1 + stack
        .recurse(async |stack| recurse(stack).await)
        .await
        .unwrap_or(0)
}

#[tokio::test]
async fn test_rec() {
    let mut stack = new_stack::<1024>();
    let stack: &mut StackStorage = &mut stack;
    let mut stack = stack.start();
    assert!(recurse(stack).await > 5);
}
