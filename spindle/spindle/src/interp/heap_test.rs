use crate::interp::heap::{HeapRef, HeapStorage};
use crate::interp::heap_types::{HeapString, HeapStringInPlace};
use crate::testutils::TEST_SPINDLE_OPTIONS;
use alloc::vec::Vec;
use core::alloc::{AllocError, Layout};
use core::fmt::{Display, Formatter};
use heapless::string::StringInPlace;
use heapless::vec::VecInPlace;
use heapless::{BuilderInPlace, VecView};
use rand::RngExt;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

#[test]
fn test_heap() {
    let mut heap_storage = HeapStorage::<20, 128>::new();
    let mut heap = heap_storage.start(1.0);
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
    heap.drop_ref(a).unwrap();
    heap.drop_ref(b).unwrap();
}

#[derive(Debug)]
struct Fake;

struct FakeInPlace(Layout);

impl FakeInPlace {
    pub fn new(layout: Layout) -> Self {
        FakeInPlace(layout)
    }
}

unsafe impl BuilderInPlace for FakeInPlace {
    type Output = Fake;

    fn layout(&self) -> Layout {
        self.0
    }

    unsafe fn build(self, ptr: *mut ()) -> *mut Self::Output {
        ptr as *mut Fake
    }
}

impl Display for Fake {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Fake")
    }
}

#[test]
fn test_compaction() {
    for compaction in [0.1, 0.5, 1.0, 2.0, 10.0] {
        let mut min_failure = usize::MAX;
        const MAX_ALLOCS: usize = 50;
        const MAX_BYTES: usize = 128;
        const MAX_OBJECT: usize = 8;
        for seed in 1..100000 {
            let mut heap_storage = HeapStorage::<MAX_ALLOCS, MAX_BYTES>::new();
            let mut bytes_count = 0;
            let mut alloc_count = 0;
            let mut refs: Vec<(Layout, HeapRef)> = vec![];
            let result = try {
                let mut heap = heap_storage.start(compaction);
                let mut rng = XorShiftRng::seed_from_u64(seed);

                for c in 0..1000 {
                    if rng.random_bool(0.5) {
                        let len: usize = rng.random_range(1..MAX_OBJECT);
                        let layout = Layout::from_size_align(len, 1).map_err(|_| AllocError)?;
                        refs.push((layout, heap.insert(FakeInPlace::new(layout))?));
                        bytes_count += layout.size();
                        alloc_count += 1;
                    } else {
                        if !refs.is_empty() {
                            let (layout, index) = refs.remove(rng.random_range(0..refs.len()));
                            heap.drop_ref(index)?;
                            bytes_count -= layout.size();
                            alloc_count -= 1;
                        }
                    }
                }
                while let Some((layout, index)) = refs.pop() {
                    heap.drop_ref(index)?;
                    bytes_count -= layout.size();
                    alloc_count -= 1;
                }
            };
            if result.is_err() {
                if alloc_count < MAX_ALLOCS - 1 {
                    min_failure = min_failure.min(bytes_count);
                }
            }
        }
        println!(
            "{:?} {:?} {:?}",
            compaction,
            ((min_failure + MAX_OBJECT) as f64) / (MAX_BYTES as f64),
            compaction / (compaction + 1.0)
        );
    }
}
