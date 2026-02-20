use crate::Arena;
use core::mem::MaybeUninit;

#[test]
fn test() {
    let mut arena = [0u8; 1024];
    let arena = Arena::new(&mut arena).unwrap();
    let foo = arena.alloc_box(123u8).unwrap();
    assert_eq!(*foo, 123u8);
}
