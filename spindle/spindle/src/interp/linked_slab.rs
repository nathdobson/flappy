use crate::interp::slab::IndexType;
use crate::interp::slab::{Slab, SlabStorage};
use core::alloc::AllocError;
use core::ops::Index;
use core::ops::IndexMut;
pub struct LinkedSlabStorage<T, const N: usize, I> {
    slab: SlabStorage<LinkedSlabEntry<T, I>, N, I>,
}

struct LinkedSlabEntry<T, I> {
    value: T,
    prev: Option<I>,
    next: Option<I>,
}

pub struct LinkedSlab<'a, T, I> {
    slab: Slab<'a, LinkedSlabEntry<T, I>, I>,
    head: Option<I>,
    tail: Option<I>,
}

impl<T, const N: usize, I: IndexType> LinkedSlabStorage<T, N, I> {
    pub fn new() -> Self {
        LinkedSlabStorage {
            slab: SlabStorage::new(),
        }
    }
    pub fn start(&mut self) -> LinkedSlab<'_, T, I> {
        LinkedSlab {
            slab: self.slab.start(),
            head: None,
            tail: None,
        }
    }
}

impl<'a, T, I: IndexType> LinkedSlab<'a, T, I> {
    pub fn push_back(&mut self, value: T) -> Result<I, AllocError> {
        if let Some(tail) = self.tail {
            let result = self.slab.insert(LinkedSlabEntry {
                value,
                prev: Some(tail),
                next: None,
            })?;
            self.tail = Some(result);
            Ok(result)
        } else {
            let result = self.slab.insert(LinkedSlabEntry {
                value,
                prev: None,
                next: None,
            })?;
            self.head = Some(result);
            self.tail = Some(result);
            Ok(result)
        }
    }
    pub fn move_to_back(&mut self, index: I) {
        if let Some(next) = self.slab[index].next {
            if let Some(prev) = self.slab[index].prev {
                self.slab[prev].next = Some(next);
                self.slab[next].prev = Some(prev);
                self.slab[index].prev = self.tail;
                self.slab[index].next = None;
                self.tail = Some(index);
            } else {
                self.slab[next].prev = None;
                self.slab[index].prev = self.tail;
                self.slab[index].next = None;
                self.head = Some(next);
                self.tail = Some(index);
            }
        } else {
            //already at back
        }
    }
    pub fn pop_front(&mut self) -> Option<T> {
        let head = self.head?;
        let result = self.slab.remove(head);
        self.head = result.next;
        if let Some(head) = self.head {
            self.slab[head].prev = None;
        }
        Some(result.value)
    }
    pub fn remove(&mut self, index: I) -> T {
        if let Some(next) = self.slab[index].next {
            if let Some(prev) = self.slab[index].prev {
                self.slab[prev].next = Some(next);
                self.slab[next].prev = Some(prev);
            } else {
                self.head = Some(next);
                self.slab[next].prev = None;
            }
        } else {
            if let Some(prev) = self.slab[index].prev {
                self.tail = Some(prev);
                self.slab[prev].next = None;
            } else {
                self.head = None;
                self.tail = None;
            }
        }
        self.slab.remove(index).value
    }
    pub fn front_index(&self) -> Option<I> {
        self.head
    }
    pub fn back_index(&self) -> Option<I> {
        self.tail
    }
    pub fn front(&self) -> Option<&T> {
        Some(&self.slab[self.head?].value)
    }
    pub fn back(&self) -> Option<&T> {
        Some(&self.slab[self.tail?].value)
    }
    pub fn front_mut(&mut self) -> Option<&mut T> {
        Some(&mut self.slab[self.head?].value)
    }
    pub fn back_mut(&mut self) -> Option<&mut T> {
        Some(&mut self.slab[self.tail?].value)
    }
}

impl<'a, T, I: IndexType> Index<I> for LinkedSlab<'_, T, I> {
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        &self.slab[index].value
    }
}

impl<'a, T, I: IndexType> IndexMut<I> for LinkedSlab<'_, T, I> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.slab[index].value
    }
}
