#![feature(unique_rc_arc)]

use std::mem::MaybeUninit;
use std::rc::{Rc, UniqueRc, Weak};

pub struct EmptyRc<T>(UniqueRc<MaybeUninit<T>>);

impl<T> EmptyRc<T> {
    pub fn new() -> Self {
        EmptyRc(UniqueRc::new(MaybeUninit::uninit()))
    }
    pub fn downgrade(&self) -> Weak<T> {
        unsafe { Weak::from_raw(Weak::into_raw(UniqueRc::downgrade(&self.0)) as *mut T) }
    }
    pub fn into_rc(mut self, value: T) -> Rc<T> {
        unsafe {
            self.0.write(value);
            Rc::from_raw(Rc::into_raw(UniqueRc::into_rc(self.0)) as *mut T)
        }
    }
}

#[test]
fn test() {
    let foo = EmptyRc::<String>::new();
    let foo_weak = foo.downgrade();
    assert!(foo_weak.upgrade().is_none());
    let foo = foo.into_rc("hi".to_string());
    assert_eq!(*foo_weak.upgrade().unwrap(), "hi");
    mem::drop(foo);
    assert!(foo_weak.upgrade().is_none());
}
