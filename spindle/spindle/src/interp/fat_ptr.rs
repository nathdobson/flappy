use core::ptr::{DynMetadata, Pointee, metadata, null};
use core::{mem, ptr};

pub trait SmallMetadata {
    fn encode(self) -> *const ();
    unsafe fn decode(ptr: *const ()) -> Self;
}

impl SmallMetadata for () {
    fn encode(self) -> *const () {
        null()
    }
    unsafe fn decode(ptr: *const ()) -> Self {
        ()
    }
}

impl SmallMetadata for usize {
    fn encode(self) -> *const () {
        unsafe { mem::transmute(self) }
    }
    unsafe fn decode(ptr: *const ()) -> Self {
        unsafe { mem::transmute(ptr) }
    }
}
impl<D> SmallMetadata for DynMetadata<D> {
    fn encode(self) -> *const () {
        unsafe { mem::transmute(self) }
    }
    unsafe fn decode(ptr: *const ()) -> Self {
        unsafe { mem::transmute(ptr) }
    }
}

pub trait HasSmallMetadata = Pointee where <Self as Pointee>::Metadata: SmallMetadata;

#[derive(Copy, Clone)]
pub struct RawFatPointer {
    pub data: *const (),
    pub metadata: *const (),
}

impl RawFatPointer {
    pub unsafe fn from_ptr<T: ?Sized>(ptr: *mut T) -> Self
    where
        T: HasSmallMetadata,
    {
        RawFatPointer {
            data: ptr as *mut (),
            metadata: metadata(ptr).encode(),
        }
    }
    pub unsafe fn into_ptr<T: ?Sized>(self) -> *const T
    where
        T: HasSmallMetadata,
    {
        unsafe { ptr::from_raw_parts(self.data, T::Metadata::decode(self.metadata)) }
    }
}
