use alloc::alloc::Allocator;
use alloc::boxed::Box;
use core::alloc::{AllocError, Layout};
use core::fmt::{Debug, Formatter};
use core::intrinsics::abort;
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::Deref;
use core::ops::DerefMut;
use core::pin::{Pin, UnsafePinned};
use core::ptr::NonNull;
use core::{mem, ptr};

pub struct PinStackStorage<B: ?Sized = [u8]> {
    borrowed: bool,
    data: B,
}

pub struct PinStack<'a> {
    parent: &'a mut bool,
    data: &'a mut [u8],
    borrowed_left: bool,
    borrowed_right: bool,
}

pub struct PinStackSlot<'a> {
    parent: Option<&'a mut bool>,
    layout: Layout,
    data: &'a mut [u8],
}

pub struct PinStackBox<'a, T> {
    parent: &'a mut bool,
    value: &'a mut ManuallyDrop<T>,
}

pub fn new_pin_stack<const N: usize>() -> PinStackStorage<[u8; N]> {
    PinStackStorage {
        borrowed: false,
        data: [0; N],
    }
}

impl PinStackStorage {
    pub fn start(&mut self) -> PinStack<'_> {
        self.borrowed = true;
        PinStack {
            parent: &mut self.borrowed,
            data: &mut self.data,
            borrowed_left: false,
            borrowed_right: false,
        }
    }
}

impl<'a> PinStack<'a> {
    pub fn push(&mut self, layout: Layout) -> Result<(PinStack<'_>, PinStackSlot<'_>), AllocError> {
        self.borrowed_left = true;
        self.borrowed_right = true;
        let mut data = &mut *self.data;
        let pad = data.as_mut_ptr().align_offset(layout.align());
        data.split_off_mut(..pad).ok_or(AllocError)?;
        let result = data.split_off_mut(..layout.size()).ok_or(AllocError)?;
        Ok((
            PinStack {
                parent: &mut self.borrowed_right,
                data,
                borrowed_left: false,
                borrowed_right: false,
            },
            PinStackSlot {
                parent: Some(&mut self.borrowed_left),
                layout,
                data: result,
            },
        ))
    }
}

impl<'a> PinStackSlot<'a> {
    pub fn init<T>(mut self, value: T) -> Result<PinStackBox<'a, T>, AllocError> {
        unsafe {
            if self.layout != Layout::new::<T>() {
                return Err(AllocError);
            }
            let ptr = self.data.as_mut_ptr() as *mut T;
            ptr::write(ptr, value);
            let parent = self.parent.take().unwrap();
            Ok(PinStackBox {
                parent,
                value: &mut *(ptr as *mut ManuallyDrop<T>),
            })
        }
    }
}

impl<'a, T> PinStackBox<'a, T> {
    pub fn into_pin(self) -> Pin<Self> {
        unsafe { Pin::new_unchecked(self) }
    }
}

impl<B: ?Sized> Drop for PinStackStorage<B> {
    fn drop(&mut self) {
        if self.borrowed {
            panic_abort();
        }
    }
}
impl<'a> Drop for PinStack<'a> {
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
impl<'a> Drop for PinStackSlot<'a> {
    fn drop(&mut self) {
        if let Some(parent) = &mut self.parent {
            **parent = false;
        }
    }
}
impl<'a, T> Drop for PinStackBox<'a, T> {
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
    panic!("failed to call destructor");
}

impl<'a, T: Debug> Debug for PinStackBox<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<'a, T> Deref for PinStackBox<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T> DerefMut for PinStackBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

#[test]
fn test() {
    let mut stack = new_pin_stack::<1024>();
    let stack: &mut PinStackStorage = &mut stack;
    let mut s = stack.start();
    let (mut s, a1) = s.push(Layout::new::<String>()).unwrap();
    let a1 = a1.init("hello".to_string()).unwrap();
    let (mut s, a2) = s.push(Layout::new::<String>()).unwrap();
    let a2 = a2.init("world".to_string()).unwrap();
    assert_eq!(*a1, "hello");
    assert_eq!(*a2, "world");
}
