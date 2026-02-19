use crate::interp::linked_slab::LinkedSlabStorage;
use std::assert_matches;

#[test]
fn test_move_to_back() {
    let mut slab = LinkedSlabStorage::<&str, 128, usize>::new();
    let mut slab = slab.start();
    let a = slab.push_back("a").unwrap();
    let b = slab.push_back("b").unwrap();
    let c = slab.push_back("c").unwrap();
    assert_eq!(slab.front_index().unwrap(), a);
    assert_eq!(slab.back_index().unwrap(), c);
    slab.move_to_back(a);
    assert_eq!(slab.front_index().unwrap(), b);
    assert_eq!(slab.back_index().unwrap(), a);
    println!("{:#?}", slab);
}

#[test]
fn test_move_to_back2() {
    let mut slab = LinkedSlabStorage::<&str, 128, usize>::new();
    let mut slab = slab.start();
    let a = slab.push_back("a").unwrap();
    assert_eq!(slab.front_index().unwrap(), a);
    assert_eq!(slab.back_index().unwrap(), a);
    slab.move_to_back(a);
    assert_eq!(slab.front_index().unwrap(), a);
    assert_eq!(slab.back_index().unwrap(), a);
}

#[test]
fn test_remove1() {
    let mut slab = LinkedSlabStorage::<&str, 128, usize>::new();
    let mut slab = slab.start();
    let a = slab.push_back("a").unwrap();
    let b = slab.push_back("b").unwrap();
    let c = slab.push_back("c").unwrap();
    slab.remove(a);
    assert_matches!(slab.into_iter().collect::<Vec<_>>(), [(1, "b"), (2, "c")]);
}

#[test]
fn test_remove2() {
    let mut slab = LinkedSlabStorage::<&str, 128, usize>::new();
    let mut slab = slab.start();
    let a = slab.push_back("a").unwrap();
    let b = slab.push_back("b").unwrap();
    let c = slab.push_back("c").unwrap();
    slab.remove(b);
    assert_matches!(slab.into_iter().collect::<Vec<_>>(), [(0, "a"), (2, "c")]);
}

#[test]
fn test_remove3() {
    let mut slab = LinkedSlabStorage::<&str, 128, usize>::new();
    let mut slab = slab.start();
    let a = slab.push_back("a").unwrap();
    let b = slab.push_back("b").unwrap();
    let c = slab.push_back("c").unwrap();
    slab.remove(c);
    assert_matches!(slab.into_iter().collect::<Vec<_>>(), [(0, "a"), (1, "b")]);
}

#[test]
fn test_pop_front() {
    let mut slab = LinkedSlabStorage::<&str, 128, usize>::new();
    let mut slab = slab.start();
    let a = slab.push_back("a").unwrap();
    let b = slab.push_back("b").unwrap();
    let c = slab.push_back("c").unwrap();
    assert_eq!(slab.pop_front(), Some("a"));
    assert_eq!(slab.pop_front(), Some("b"));
    assert_eq!(slab.pop_front(), Some("c"));
    assert_eq!(slab.pop_front(), None);
}
