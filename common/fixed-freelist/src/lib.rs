#![no_std]
#![feature(allocator_api)]
#![feature(unsafe_pinned)]
#![feature(unsize)]
extern crate alloc;

#[cfg(test)]
mod test;

use alloc::boxed::Box;
use core::alloc::{AllocError, Allocator, Layout};
use core::cell::RefCell;
use core::marker::Unsize;
use core::mem::MaybeUninit;
use core::pin::UnsafePinned;
use core::ptr::NonNull;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::RawMutex;
use heapless::{Vec, VecView};

pub struct FreelistStorage<M: RawMutex, T, const N: usize> {
    memory: [UnsafePinned<MaybeUninit<T>>; N],
    freelist: Mutex<M, RefCell<Vec<usize, N>>>,
}

#[derive(Copy, Clone)]
pub struct Freelist<'a, M: RawMutex> {
    freelist: &'a Mutex<M, RefCell<VecView<usize>>>,
    index: usize,
}

impl<M: RawMutex, T, const N: usize> FreelistStorage<M, T, N> {
    pub fn new() -> Self {
        let mut freelist = Vec::new();
        for i in 0..N {
            freelist.push(i).unwrap();
        }
        FreelistStorage {
            memory: [const { UnsafePinned::new(MaybeUninit::uninit()) }; N],
            freelist: Mutex::new(RefCell::new(freelist)),
        }
    }
    pub fn alloc_box<T2>(&self, value: T2) -> Result<Box<T2, Freelist<'_, M>>, AllocError> {
        self.alloc_box_with(|| value)
    }
    pub fn alloc_box_with<F: FnOnce() -> T2, T2>(
        &self,
        value: F,
    ) -> Result<Box<T2, Freelist<'_, M>>, AllocError> {
        unsafe {
            if Layout::new::<T>() != Layout::new::<T2>() {
                return Err(AllocError);
            }
            let index = self
                .freelist
                .lock(|freelist| freelist.borrow_mut().pop().ok_or(AllocError))?;
            let ptr = (*self.memory[index].get()).as_mut_ptr() as *mut T2;
            ptr.write(value());
            Ok(Box::from_raw_in(
                ptr,
                Freelist {
                    freelist: &self.freelist,
                    index,
                },
            ))
        }
    }
}

pub trait AllocBoxDefault<M: RawMutex, U: ?Sized> {
    fn alloc_box_default(&self) -> Result<Box<U, Freelist<'_, M>>, AllocError>;
}

impl<const N: usize, M: RawMutex, T, U: ?Sized> AllocBoxDefault<M, U> for FreelistStorage<M, T, N>
where
    T: Default + Unsize<U>,
{
    fn alloc_box_default(&self) -> Result<Box<U, Freelist<'_, M>>, AllocError> {
        Ok(self.alloc_box_with(T::default)?)
    }
}

unsafe impl<'a, M: RawMutex> Allocator for Freelist<'a, M> {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        self.freelist
            .lock(|freelist| freelist.borrow_mut().push(self.index).unwrap());
    }
}
