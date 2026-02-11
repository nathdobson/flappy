use crate::interp::fat_ptr::HasSmallMetadata;
use crate::interp::fat_ptr::{RawFatPointer, SmallMetadata};
use core::any::TypeId;
use core::fmt;
use core::fmt::{Debug, Display, Formatter};
use core::marker::PhantomData;
use core::ptr::Pointee;
pub struct VTable {
    type_id: TypeId,
    debug_fmt: unsafe fn(RawFatPointer, f: &mut Formatter) -> fmt::Result,
    display_fmt: unsafe fn(RawFatPointer, f: &mut Formatter) -> fmt::Result,
}

pub struct FatRef<'a> {
    ptr: RawFatPointer,
    vtable: &'static VTable,
    phantom: PhantomData<&'a ()>,
}

impl<'a> FatRef<'a> {
    pub unsafe fn new(ptr: RawFatPointer, vtable: &'static VTable) -> Self {
        FatRef {
            ptr,
            vtable,
            phantom: PhantomData,
        }
    }
}

impl<'a> Debug for FatRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe { (self.vtable.debug_fmt)(self.ptr, f) }
    }
}

impl<'a> Display for FatRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe { (self.vtable.display_fmt)(self.ptr, f) }
    }
}

impl VTable {
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}

pub trait HasVTable: HasSmallMetadata {
    fn vtable() -> &'static VTable;
}

impl<T: ?Sized + 'static + Debug + Display + HasSmallMetadata> HasVTable for T {
    fn vtable() -> &'static VTable {
        &const {
            VTable {
                type_id: TypeId::of::<T>(),
                debug_fmt: |ptr, formatter| unsafe { Debug::fmt(&*ptr.into_ptr::<T>(), formatter) },
                display_fmt: |ptr, formatter| unsafe {
                    Display::fmt(&*ptr.into_ptr::<T>(), formatter)
                },
            }
        }
    }
}
