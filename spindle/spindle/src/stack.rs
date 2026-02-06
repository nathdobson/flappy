use alloc::alloc::Allocator;
use alloc::boxed::Box;
use core::alloc::{AllocError, Layout};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

pub struct StackFrame<'a> {
    buffer: &'a mut [u8],
}

pub struct StackErase<'a>(PhantomData<&'a ()>);
pub type StackBox<'a, T> = Box<T, StackErase<'a>>;

impl<'a> StackFrame<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        StackFrame { buffer }
    }
    pub fn alloc_box<T>(&mut self, value: T) -> Result<StackBox<'a, T>, AllocError> {
        Ok(Box::write(self.alloc_uninit::<T>()?, value))
    }
    pub fn alloc_uninit<T>(&mut self) -> Result<StackBox<'a, MaybeUninit<T>>, AllocError> {
        unsafe {
            Ok(Box::from_raw_in(
                self.alloc_layout(Layout::new::<T>())?.as_ptr() as *mut MaybeUninit<T>,
                StackErase(PhantomData),
            ))
        }
    }
    fn alloc_layout(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let pad = self.buffer.as_mut_ptr().align_offset(layout.align());
        self.buffer.split_off_mut(..pad).ok_or(AllocError)?;
        let result = self
            .buffer
            .split_off_mut(..layout.size())
            .ok_or(AllocError)?;
        Ok(NonNull::new(result).ok_or(AllocError)?)
    }
    pub fn reborrow<'b>(&'b mut self) -> StackFrame<'b> {
        StackFrame { buffer: self.buffer }
    }
}

unsafe impl<'a> Allocator for StackErase<'a> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}
