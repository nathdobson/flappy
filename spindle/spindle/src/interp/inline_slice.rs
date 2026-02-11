use core::alloc::{AllocError, Layout};
use core::fmt::{Debug, Display, Formatter};
use core::marker::{PhantomData, Unsize};
use core::ops::Deref;
use core::ops::DerefMut;
use core::ptr;
use core::ptr::Pointee;
use heapless::BuilderInPlace;
#[repr(C)]
pub struct InlineSlice<Z, U: ?Sized> {
    len: usize,
    inner: Z,
    phantom: PhantomData<U>,
}

pub struct InlineSliceInPlace<B, Z> {
    inner: B,
    layout: Layout,
    offset: usize,
    phantom: PhantomData<Z>,
}

impl<B: BuilderInPlace, Z> InlineSliceInPlace<B, Z> {
    pub fn new(inner: B) -> Result<Self, AllocError> {
        let (layout, offset) = Layout::new::<usize>()
            .extend(inner.layout())
            .ok()
            .ok_or(AllocError)?;
        Ok(Self {
            inner,
            layout,
            offset,
            phantom: PhantomData,
        })
    }
}

unsafe impl<B: BuilderInPlace, Z> BuilderInPlace for InlineSliceInPlace<B, Z>
where
    Z: Unsize<B::Output>,
    <B as BuilderInPlace>::Output: Pointee<Metadata = usize>,
{
    type Output = InlineSlice<Z, B::Output>;

    fn layout(&self) -> Layout {
        self.layout
    }

    unsafe fn build(self, ptr: *mut ()) -> *mut Self::Output {
        unsafe {
            let mut result = ptr as *mut Self::Output;
            let len = ptr::metadata(self.inner.build((&raw mut (*result).inner) as *mut ()));
            (*result).len = len;
            result
        }
    }
}

impl<Z, U: ?Sized + Pointee<Metadata = usize>> Deref for InlineSlice<Z, U> {
    type Target = U;
    fn deref(&self) -> &Self::Target {
        unsafe { &*ptr::from_raw_parts::<U>(&self.inner, self.len) }
    }
}

impl<Z, U: ?Sized + Pointee<Metadata = usize>> DerefMut for InlineSlice<Z, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *ptr::from_raw_parts_mut::<U>(&mut self.inner, self.len) }
    }
}

impl<Z, U: ?Sized + Debug + Pointee<Metadata = usize>> Debug for InlineSlice<Z, U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<Z, U: ?Sized + Display + Pointee<Metadata = usize>> Display for InlineSlice<Z, U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&**self, f)
    }
}
