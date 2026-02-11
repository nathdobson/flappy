use crate::interp::heap::HeapStorage;
use heapless::VecView;
use heapless::vec::VecInPlace;

#[test]
fn test_heap() {
    let mut heap_storage = HeapStorage::<20, 128>::new();
    let mut heap = heap_storage.start();
    // let a = heap.insert(VecInPlace::<u32>::new(4)).unwrap();
    // assert_eq!(&*heap.try_get::<VecView<u32>>(&a).unwrap(), &[]);
    // heap.try_get_mut::<VecView<u32>>(&a).unwrap().push(42).unwrap();
    // assert_eq!(&*heap.try_get::<VecView<u32>>(&a).unwrap(), &[42]);
    // heap.drop_ref(a);
}
