use alloc::alloc::Allocator;
use alloc::boxed::Box;
use core::alloc::{AllocError, Layout};
use core::fmt::{Debug, Formatter};
use core::intrinsics::abort;
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::Deref;
use core::ops::DerefMut;
use core::pin::pin;
use core::pin::{Pin, UnsafePinned};
use core::ptr::NonNull;
use core::task::{Context, Poll};
use core::{mem, ptr};

pub struct StackStorage<B: ?Sized = [u8]> {
    borrowed: bool,
    data: B,
}

pub struct Stack<'a> {
    parent: &'a mut bool,
    data: &'a mut [u8],
    borrowed_left: bool,
    borrowed_right: bool,
}

pub struct StackSlot<'a> {
    parent: Option<&'a mut bool>,
    layout: Layout,
    data: &'a mut [u8],
}

pub struct StackBox<'a, T: ?Sized> {
    parent: &'a mut bool,
    value: &'a mut ManuallyDrop<T>,
}

pub fn new_stack<const N: usize>() -> StackStorage<[u8; N]> {
    StackStorage {
        borrowed: false,
        data: [0; N],
    }
}

impl StackStorage {
    pub fn start(&mut self) -> Stack<'_> {
        self.borrowed = true;
        Stack {
            parent: &mut self.borrowed,
            data: &mut self.data,
            borrowed_left: false,
            borrowed_right: false,
        }
    }
}

impl<'a> Stack<'a> {
    pub fn push(&mut self, layout: Layout) -> Result<(Stack<'_>, StackSlot<'_>), AllocError> {
        let mut data = &mut *self.data;
        let pad = data.as_mut_ptr().align_offset(layout.align());
        data.split_off_mut(..pad).ok_or(AllocError)?;
        let result = data.split_off_mut(..layout.size()).ok_or(AllocError)?;
        self.borrowed_left = true;
        self.borrowed_right = true;
        Ok((
            Stack {
                parent: &mut self.borrowed_right,
                data,
                borrowed_left: false,
                borrowed_right: false,
            },
            StackSlot {
                parent: Some(&mut self.borrowed_left),
                layout,
                data: result,
            },
        ))
    }
    pub async fn recurse<O, F: for<'b> AsyncFnOnce(Stack<'b>) -> O>(
        &mut self,
        f: F,
    ) -> Result<O, AllocError> {
        let (stack, slot) = self.push(Layout::new::<
            <F as AsyncFnOnce<(Stack<'static>,)>>::CallOnceFuture,
        >())?;
        Ok(slot.init(f(stack))?.into_pin().await)
    }
}

impl<'a> StackSlot<'a> {
    pub fn init<T>(mut self, value: T) -> Result<StackBox<'a, T>, AllocError> {
        unsafe {
            if self.layout != Layout::new::<T>() {
                return Err(AllocError);
            }
            let ptr = self.data.as_mut_ptr() as *mut T;
            ptr::write(ptr, value);
            let parent = self.parent.take().unwrap();
            Ok(StackBox {
                parent,
                value: &mut *(ptr as *mut ManuallyDrop<T>),
            })
        }
    }
}

impl<'a, T> StackBox<'a, T> {
    pub fn into_pin(self) -> Pin<Self> {
        unsafe { Pin::new_unchecked(self) }
    }
}

impl<B: ?Sized> Drop for StackStorage<B> {
    fn drop(&mut self) {
        if self.borrowed {
            panic_abort();
        }
    }
}
impl<'a> Drop for Stack<'a> {
    fn drop(&mut self) {
        if self.borrowed_left {
            panic_abort();
        }
        if self.borrowed_right {
            panic_abort();
        }
        *self.parent = false;
    }
}
impl<'a> Drop for StackSlot<'a> {
    fn drop(&mut self) {
        if let Some(parent) = &mut self.parent {
            **parent = false;
        }
    }
}
impl<'a, T: ?Sized> Drop for StackBox<'a, T> {
    fn drop(&mut self) {
        unsafe {
            *self.parent = false;
            ManuallyDrop::drop(self.value);
        }
    }
}

fn panic_abort() {
    struct Bomb();
    impl Drop for Bomb {
        fn drop(&mut self) {
            abort();
        }
    }
    let _x = Bomb();
    panic!("Didn't call destructor.");
}

impl<'a, T: Debug> Debug for StackBox<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized> Deref for StackBox<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T: ?Sized> DerefMut for StackBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
