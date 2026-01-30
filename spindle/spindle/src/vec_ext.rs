use alloc::alloc::AllocError;
use alloc::vec::Vec;
use alloc::alloc::Allocator;
pub trait VecExt {
    type Item;
    fn try_push(&mut self, item: Self::Item) -> Result<(), AllocError>;
}

impl<T, A: Allocator> VecExt for Vec<T, A> {
    type Item = T;
    fn try_push(&mut self, item: Self::Item) -> Result<(), AllocError> {
        self.try_reserve(1).map_err(|_| AllocError)?;
        self.push_within_capacity(item).map_err(|_| AllocError)?;
        Ok(())
    }
}
