use core::alloc::{AllocError, Layout};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ptr;
use core::ptr::{Pointee, metadata};
use heapless::BuilderInPlace;
use heapless::string::StringInPlace;
#[repr(C)]
pub struct InlineMetadata<T: ?Sized + Pointee> {
    metadata: T::Metadata,
    offset: usize,
    value: PhantomData<T>,
}

struct InlineMetadataInPlace<B> {
    inner: B,
    layout: Layout,
    offset: usize,
}

impl<B: BuilderInPlace> InlineMetadataInPlace<B> {
    pub fn new(inner: B) -> Result<Self, AllocError> {
        Layout::new::<<B::Output as Pointee>::Metadata>()
            .extend(inner.0.layout()?)
            .ok()
            .ok_or(AllocError)?;
        Ok(InlineMetadataInPlace { inner })
    }
}

unsafe impl<B: BuilderInPlace> BuilderInPlace for InlineMetadataInPlace<B> {
    type Output = InlineMetadata<B::Output>;

    fn layout(&self) -> Layout {
        self.layout
    }

    unsafe fn build(self, ptr: *mut ()) -> *mut Self::Output {
        unsafe {
            let offset = Layout::new::<<B::Output as Pointee>::Metadata>()
                .extend(self.0.layout().unwrap())
                .unwrap()
                .1;
            let inner = self
                .0
                .build((ptr as *mut u8).offset(offset as isize) as *mut ());
            let outer = ptr as *mut Self::Output;
            (*outer).metadata = metadata(inner);

            outer
        }
    }
}

impl<T> Deref for InlineMetadata<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        todo!();
    }
}

#[test]
fn test() {
    unsafe {
        let mut value = [0u8; 1024];
        let result = InlineMetadataInPlace::new(StringInPlace::new(128))
            .build(&mut value as *mut [u8; 1024] as *mut ());
    }
}
