use crate::interp::heap::HeapStorage;
use crate::interp::heap_types::{HeapString, HeapStringInPlace};
use heapless::VecView;
use heapless::string::StringInPlace;
use heapless::vec::VecInPlace;

#[test]
fn test_heap() {
    let mut heap_storage = HeapStorage::<20, 128>::new();
    let mut heap = heap_storage.start();
    let a = heap
        .insert(HeapStringInPlace::new(StringInPlace::new(1)).unwrap())
        .unwrap();
    heap.get_typed_mut::<HeapString>(&a)
        .unwrap()
        .push_str("A")
        .unwrap();
    let b = heap
        .insert(HeapStringInPlace::new(StringInPlace::new(1)).unwrap())
        .unwrap();
    heap.get_typed_mut::<HeapString>(&b)
        .unwrap()
        .push_str("B")
        .unwrap();
    heap.drop_ref(a);
    heap.drop_ref(b);
}
