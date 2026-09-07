#![no_std]
#![feature(ptr_metadata)]
#![deny(unused_must_use)]

use core::alloc::Layout;
use core::marker::PhantomData;
use core::ptr;
use core::ptr::null;
use heapless::VecView;
use heapless::string::StringView;

pub unsafe trait UnsizedBuilder {
    type Output: ?Sized;
    fn layout(&self) -> Layout;
    unsafe fn build(self, ptr: *mut ()) -> *mut Self::Output;
}

pub struct StringBuilder(usize);
impl StringBuilder {
    pub fn new(cap: usize) -> Self {
        StringBuilder(cap)
    }
}

unsafe impl UnsizedBuilder for StringBuilder {
    type Output = StringView;

    fn layout(&self) -> Layout {
        unsafe { Layout::for_value_raw(ptr::from_raw_parts::<StringView>(null::<()>(), self.0)) }
    }

    unsafe fn build(self, p: *mut ()) -> *mut Self::Output {
        let result: *mut Self::Output = ptr::from_raw_parts_mut(p as *mut u8, self.0);
        unsafe {
            (*result).set_len(0);
        }
        result
    }
}

pub struct VecBuilder<T>(usize, PhantomData<T>);

impl<T> VecBuilder<T> {
    pub fn new(cap: usize) -> Self {
        VecBuilder(cap, PhantomData)
    }
}

unsafe impl<T> UnsizedBuilder for VecBuilder<T> {
    type Output = VecView<T>;

    fn layout(&self) -> Layout {
        unsafe { Layout::for_value_raw(ptr::from_raw_parts::<VecView<T>>(null::<()>(), self.0)) }
    }

    unsafe fn build(self, p: *mut ()) -> *mut Self::Output {
        let result: *mut Self::Output = ptr::from_raw_parts_mut(p as *mut u8, self.0);
        unsafe {
            (*result).set_len(0);
        }
        result
    }
}
