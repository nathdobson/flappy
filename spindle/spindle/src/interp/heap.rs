use heapless::deque::DequeView;
use heapless::{Deque, Vec, VecView};

type HeapAddress = u32;

type HeapRef = u16;
struct HeapEntry {
    start: HeapAddress,
    limit: HeapAddress,
    refcount: u8,
}

struct HeapStorage<const N: usize, const C: usize> {
    entries: [Option<HeapEntry>; N],
    freelist: Vec<HeapRef, N>,
    heap_queue: Deque<HeapRef, N>,
    heap: [u8; C],
}

struct Heap<'a> {
    entries: &'a mut [Option<HeapEntry>],
    freelist: &'a mut VecView<HeapRef>,
    heap_queue: &'a mut DequeView<HeapRef>,
    heap: &'a mut [u8],
}

impl<const N: usize, const C: usize> HeapStorage<N, C> {
    pub fn new() -> Self {
        HeapStorage {
            entries: [const { None }; N],
            freelist: (0u16..N as HeapRef).collect(),
            heap_queue: Deque::new(),
            heap: [0; C],
        }
    }
    pub fn start(&mut self) -> Heap<'_> {
        Heap {
            entries: &mut self.entries,
            freelist: &mut self.freelist,
            heap_queue: &mut self.heap_queue,
            heap: &mut self.heap,
        }
    }
}
