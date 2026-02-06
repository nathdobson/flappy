#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(raw_slice_split)]
#![deny(unused_must_use)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]


mod test;

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::{AllocError, Allocator, Layout};
use core::cell::{RefCell, RefMut, UnsafeCell};
use core::marker::PhantomData;
use core::mem;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

pub struct ArenaStorage<'a> {
    buffer: &'a mut [u8],
    arena: Option<Arena>,
}

struct ArenaState {
    buffer: &'static mut [u8],
}

pub struct Arena {
    state: RefCell<ArenaState>,
}

pub struct ArenaErase<'ar> {
    phantom: PhantomData<&'ar mut [MaybeUninit<u8>]>,
}

pub type ArenaBox<'ar, T> = Box<T, ArenaErase<'ar>>;

pub type ArenaVec<'ar, T> = Vec<T, &'ar Arena>;

impl<'a> ArenaStorage<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        ArenaStorage {
            buffer,
            arena: None,
        }
    }
    pub fn start<'ar>(&'ar mut self) -> &'ar Arena {
        unsafe { self.arena.insert(Arena::new(self.buffer)) }
    }
}

impl Arena {
    unsafe fn new(buffer: *mut [u8]) -> Arena {
        unsafe {
            Arena {
                state: RefCell::new(ArenaState {
                    buffer: &mut *buffer,
                }),
            }
        }
    }
    pub fn alloc_layout(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.state.borrow_mut().alloc_layout(layout)
    }
    pub fn alloc_box<'ar, T>(&'ar self, value: T) -> Result<ArenaBox<'ar, T>, AllocError> {
        Ok(Self::erase_box(Box::try_new_in(value, self)?))
    }
    pub fn alloc_vec<'ar, const N: usize, T>(
        &'ar self,
        values: [T; N],
    ) -> Result<ArenaVec<'ar, T>, AllocError> {
        Ok((Box::try_new_in(values, self)? as Box<[T], &'ar Arena>).into())
    }
    pub fn erase_box<'ar, T>(b: Box<T, &'ar Self>) -> ArenaBox<'ar, T> {
        unsafe {
            ArenaBox::from_raw_in(
                Box::into_raw_with_allocator(b).0,
                ArenaErase {
                    phantom: PhantomData,
                },
            )
        }
    }
    pub fn alloc_ref<'ar, T: Copy>(&'ar self, value: T) -> Result<&'ar T, AllocError> {
        Ok(Box::leak(self.alloc_box(value)?))
    }
    pub fn alloc_bytes<'ar>(&'ar self, count: usize) -> Result<&'ar mut [u8], AllocError> {
        unsafe { Ok(self.state.borrow_mut().alloc_bytes(count)?.as_mut()) }
    }
    pub fn remaining(&self) -> usize {
        self.state.borrow().buffer.len()
    }
}

impl ArenaState {
    pub fn alloc_layout(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let pad = self.buffer.as_mut_ptr().align_offset(layout.align());
        self.buffer.split_off_mut(..pad).ok_or(AllocError)?;
        let result = self
            .buffer
            .split_off_mut(..layout.size())
            .ok_or(AllocError)?;
        Ok(NonNull::new(result).ok_or(AllocError)?)
    }
    pub fn alloc_bytes(&mut self, count: usize) -> Result<NonNull<[u8]>, AllocError> {
        Ok(
            NonNull::new(self.buffer.split_off_mut(..count).ok_or(AllocError)?)
                .ok_or(AllocError)?,
        )
    }
}
unsafe impl<'ar> Allocator for &'ar Arena {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc_layout(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}

unsafe impl<'ar> Allocator for ArenaErase<'ar> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}
