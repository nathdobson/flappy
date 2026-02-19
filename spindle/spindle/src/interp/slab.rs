use crate::interp::linked_slab::LinkedSlab;
use core::alloc::AllocError;
use core::fmt::{Debug, Display, Formatter};
use core::marker::PhantomData;
use core::num::TryFromIntError;
use core::ops::Index;
use core::ops::IndexMut;
use core::slice;
use heapless::{Vec, VecView};
use core::ops::Add;

pub struct SlabStorage<T, const N: usize, I> {
    values: [Option<T>; N],
    freelist: Vec<I, N>,
}

pub struct Slab<'a, T, I> {
    values: &'a mut [Option<T>],
    freelist: &'a mut VecView<I>,
}

pub trait IndexType = Debug
    + Display
    + Copy
    + Clone
    + Debug
    + TryFrom<usize>
    + Into<usize>
    + Add<Self, Output = Self>
where <Self as TryFrom<usize>>::Error: Debug;

impl<T, const N: usize, I: IndexType> SlabStorage<T, N, I> {
    pub fn new() -> Self {
        let mut freelist = Vec::new();
        for i in (0..N).rev() {
            freelist.push(I::try_from(i).unwrap()).unwrap();
        }
        SlabStorage {
            values: [const { None }; N],
            freelist,
        }
    }
    pub fn start(&mut self) -> Slab<'_, T, I> {
        Slab {
            values: &mut self.values,
            freelist: &mut self.freelist,
        }
    }
}

impl<'a, T, I: IndexType> Slab<'a, T, I> {
    pub fn insert(&mut self, value: T) -> Result<I, AllocError> {
        let index = self.freelist.pop().ok_or(AllocError)?;
        self.values[index.into()] = Some(value);
        Ok(index)
    }
    pub fn remove(&mut self, index: I) -> T {
        let result = self.values[index.into()].take().unwrap();
        self.freelist.push(index).unwrap();
        result
    }
}

impl<'a, T, I: IndexType> Index<I> for Slab<'_, T, I> {
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        self.values[index.into()]
            .as_ref()
            .unwrap_or_else(|| panic!("No item at index {}", index))
    }
}

impl<'a, T, I: IndexType> IndexMut<I> for Slab<'_, T, I> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.values[index.into()]
            .as_mut()
            .unwrap_or_else(|| panic!("No item at index {}", index))
    }
}

pub struct Iter<'a, T, I: IndexType> {
    index: I,
    slab: &'a [Option<T>],
}

impl<'b, 'a, T, I: IndexType> IntoIterator for &'b Slab<'a, T, I> {
    type Item = (I, &'b T);
    type IntoIter = Iter<'b, T, I>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            index: I::try_from(0).unwrap(),
            slab: &self.values,
        }
    }
}

impl<'a, T, I: IndexType> Iterator for Iter<'a, T, I> {
    type Item = (I, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.index;
            self.index = index + I::try_from(1).unwrap();
            if let Some(value) = self.slab.get(index.into())? {
                return Some((index, value));
            }
        }
    }
}

impl<'a, T: Debug, I: IndexType> Debug for Slab<'a, T, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self).finish()
    }
}
