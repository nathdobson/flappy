use crate::FreelistStorage;
use crate::alloc::string::ToString;
use alloc::string::String;
use core::mem;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

#[test]
fn test() {
    let freelist = FreelistStorage::<NoopRawMutex, String, 2>::new();
    let a = freelist.alloc_box("a".to_string()).unwrap();
    let b = freelist.alloc_box("b".to_string()).unwrap();
    assert_eq!(*"a", **a);
    assert_eq!(*"b", **b);
    assert!(freelist.alloc_box("c".to_string()).is_err());
    mem::drop(a);
    let d = freelist.alloc_box("d".to_string()).unwrap();
    assert_eq!(*"b", **b);
    assert_eq!(*"d", **d);
    assert!(freelist.alloc_box("c".to_string()).is_err());
}
