use crate::{Arena, ArenaStorage};
use core::mem::MaybeUninit;

#[test]
fn test() {
    let mut arena_storage = ArenaStorage::<1024>::new();
    let arena = arena_storage.start();
    let foo = arena.alloc_box(123u8).unwrap();
    assert_eq!(*foo, 123u8);
}
