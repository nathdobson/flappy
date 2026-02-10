use core::alloc::AllocError;
use core::fmt::Debug;
use core::ops::Index;
use core::ops::IndexMut;
use heapless::{Vec, VecView};

pub struct SlabStorage<T, const N: usize, I> {
    values: [Option<T>; N],
    freelist: Vec<I, N>,
}

pub struct Slab<'a, T, I> {
    values: &'a mut [Option<T>],
    freelist: &'a mut VecView<I>,
}

pub trait IndexType = Copy + Clone + Debug + TryFrom<usize> + Into<usize>
where <Self as TryFrom<usize>>::Error: core::fmt::Debug;

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
        self.values[index.into()].as_ref().unwrap()
    }
}

impl<'a, T, I: IndexType> IndexMut<I> for Slab<'_, T, I> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.values[index.into()].as_mut().unwrap()
    }
}
