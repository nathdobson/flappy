use core::ops::{Deref, DerefMut};
use heapless::Vec;

pub struct Bytes<const N: usize>(pub Vec<u8, N>);

impl<const N: usize> Deref for Bytes<N> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for Bytes<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> Serialize for Bytes<N> {}
