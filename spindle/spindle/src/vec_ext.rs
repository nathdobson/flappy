use alloc::alloc::AllocError;
use alloc::alloc::Allocator;
use alloc::vec::Vec;
pub trait VecExt {
    type Item;
    fn try_push(&mut self, item: Self::Item) -> Result<(), AllocError>;
    fn try_extend_from_slice(&mut self, slice: &[Self::Item]) -> Result<(), AllocError>
    where
        Self::Item: Clone;
}

impl<T, A: Allocator> VecExt for Vec<T, A> {
    type Item = T;
    fn try_push(&mut self, item: Self::Item) -> Result<(), AllocError> {
        self.try_reserve(1).map_err(|_| AllocError)?;
        self.push_within_capacity(item).map_err(|_| AllocError)?;
        Ok(())
    }

    fn try_extend_from_slice(&mut self, slice: &[Self::Item]) -> Result<(), AllocError>
    where
        Self::Item: Clone,
    {
        for x in slice {
            self.try_push(x.clone())?;
        }
        Ok(())
    }
}
