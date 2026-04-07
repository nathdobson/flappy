#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
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

struct ArenaState {
    buffer: &'static mut [u8],
}

/// An arena allocator with a fixed capacity.
pub struct Arena {
    state: RefCell<ArenaState>,
}

/// An `ArenaErase<'ar>` is a zero-size no-op `Allocator` compatible with `&'ar Arena`. This allows
/// ArenaBox to use less memory while maintaining the lifetime invariants and ensuring destructors
/// are called.
pub struct ArenaErase<'ar> {
    phantom: PhantomData<&'ar mut [MaybeUninit<u8>]>,
}

/// A Box allocated on an `Arena` instead of a heap.
pub type ArenaBox<'ar, T> = Box<T, ArenaErase<'ar>>;

/// A Vec allocated on an `Arena` instead of a heap.
pub type ArenaVec<'ar, T> = Vec<T, &'ar Arena>;

impl Arena {
    /// Construct a new arena in the specified buffer. The `Arena` struct itself is stored on the
    /// arena, so this function may fail for especially small buffers.
    pub fn new(buffer: &'_ mut [u8]) -> Result<&'_ Arena, AllocError> {
        unsafe {
            let arena = Arena {
                state: RefCell::new(ArenaState {
                    buffer: &mut *(buffer as *mut [u8]),
                }),
            };
            let arena_ptr: &mut MaybeUninit<Arena> =
                Box::leak(arena.alloc_box::<MaybeUninit<Arena>>(MaybeUninit::uninit())?);
            let arena_ptr = &mut *(arena_ptr as *mut MaybeUninit<Arena>);
            let arena_ptr = arena_ptr.write(arena);
            Ok(arena_ptr)
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
    pub fn alloc_str(&'_ self, value: &str) -> Result<&'_ str, AllocError> {
        unsafe {
            let mut ptr = self.alloc_layout(
                Layout::from_size_align(value.len(), 1)
                    .ok()
                    .ok_or(AllocError)?,
            )?;
            ptr.as_mut().copy_from_slice(value.as_bytes());
            Ok(str::from_utf8_unchecked(ptr.as_mut()))
        }
    }
    /// Change the allocator for a Box to avoid the overhead of an extra pointer.
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
    /// Place a value on the Arena, and return a mutable reference. This can be a reference instead
    /// of an `ArenaBox` because destruction is a no-op for `Copy` types, and deallocation is a
    /// no-op for arenas.
    pub fn alloc_ref<'ar, T: Copy>(&'ar self, value: T) -> Result<&'ar mut T, AllocError> {
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

pub trait IntoRef {
    type Target;
    fn into_ref(self) -> Self::Target;
}

impl<'ar, T: 'ar> IntoRef for ArenaVec<'ar, T>
where
    T: Copy,
{
    type Target = &'ar [T];

    fn into_ref(self) -> Self::Target {
        Vec::leak(self)
    }
}

impl<'ar, T: 'ar> IntoRef for ArenaBox<'ar, T>
where
    T: Copy,
{
    type Target = &'ar T;

    fn into_ref(self) -> Self::Target {
        Box::leak(self)
    }
}
