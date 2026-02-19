use crate::interp::slab::IndexType;
use crate::interp::slab::{Slab, SlabStorage};
use core::alloc::AllocError;
use core::fmt::{Debug, Formatter};
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
    fn push_back_link(&mut self, index: I) {
        if let Some(tail) = self.tail {
            self.slab[index].prev = Some(tail);
            self.slab[index].next = None;
            self.slab[tail].next = Some(index);
            self.tail = Some(index);
        } else {
            self.slab[index].next = None;
            self.slab[index].prev = None;
            self.head = Some(index);
            self.tail = Some(index);
        }
    }
    pub fn push_back(&mut self, value: T) -> Result<I, AllocError> {
        let index = self.slab.insert(LinkedSlabEntry {
            value,
            prev: None,
            next: None,
        })?;
        self.push_back_link(index);
        Ok(index)
    }
    pub fn move_to_back(&mut self, index: I) {
        self.remove_link(index);
        self.push_back_link(index);
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
    fn remove_link(&mut self, index: I) {
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
    }
    pub fn remove(&mut self, index: I) -> T {
        self.remove_link(index);
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

impl<'a, T: Debug, I: IndexType> Debug for LinkedSlab<'a, T, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LinkedSlab")
            .field_with("front", |f| write!(f, "{:?}", &self.head))
            .field_with("tail", |f| write!(f, "{:?}", &self.tail))
            .field("slab", &self.slab)
            .finish()
    }
}

pub struct Iter<'b, 'a, T, I: IndexType> {
    index: Option<I>,
    slab: &'b LinkedSlab<'a, T, I>,
}

impl<'b, 'a, T, I: IndexType> IntoIterator for &'b LinkedSlab<'a, T, I> {
    type Item = (I, &'b T);
    type IntoIter = Iter<'b, 'a, T, I>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            index: self.front_index(),
            slab: self,
        }
    }
}

impl<'b, 'a, T, I: IndexType> Iterator for Iter<'b, 'a, T, I> {
    type Item = (I, &'b T);
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index?;
        self.index = self.slab.slab[index].next;
        Some((index, &self.slab[index]))
    }
}

impl<I: Debug, T: Debug> Debug for LinkedSlabEntry<I, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LinkedSlabEntry")
            .field_with("prev", |f| write!(f, "{:?}", &self.prev))
            .field_with("next", |f| write!(f, "{:?}", &self.next))
            .field("value", &self.value)
            .finish()
    }
}
