use crate::interp::error::TypeError;
use crate::interp::linked_slab::{LinkedSlab, LinkedSlabStorage};
use core::alloc::{AllocError, Layout};
use core::any::{Any, TypeId};
use core::fmt::{Debug, Display, Formatter};
use core::ops::Range;
use core::pin::UnsafePinned;
use core::ptr::{DynMetadata, Pointee, metadata, null};
use core::{fmt, mem, ptr};
use heapless::deque::DequeView;
use heapless::{BuilderInPlace, Deque, Vec, VecView};
use log::info;

type HeapAddress = u32;

type HeapIndex = u16;

#[derive(Debug)]
pub struct HeapRef(HeapIndex);

struct HeapEntry {
    address: Range<HeapAddress>,
    refcount: u8,
    metadata: DynMetadata<dyn HeapObject>,
}

pub struct HeapStorage<const N: usize, const C: usize> {
    entries: LinkedSlabStorage<HeapEntry, N, HeapIndex>,
    heap: UnsafePinned<[u8; C]>,
}

pub struct Heap<'a> {
    entries: LinkedSlab<'a, HeapEntry, HeapIndex>,
    heap: &'a mut UnsafePinned<[u8]>,
}

pub trait HeapObject: Any + Debug + Display {}

impl<T> HeapObject for T where T: Any + Debug + Display {}

impl<const N: usize, const C: usize> HeapStorage<N, C> {
    pub fn new() -> Self {
        HeapStorage {
            entries: LinkedSlabStorage::new(),
            heap: UnsafePinned::new([0u8; C]),
        }
    }
    pub fn start(&mut self) -> Heap<'_> {
        Heap {
            entries: self.entries.start(),
            heap: &mut self.heap,
        }
    }
}

impl<'a> Heap<'a> {
    fn find_address(&self, layout: Layout) -> Result<Range<HeapAddress>, AllocError> {
        let free_start: HeapAddress = if let Some(back) = self.entries.back() {
            Layout::from_size_align(back.address.end as usize, 1)
                .ok()
                .ok_or(AllocError)?
                .align_to(layout.align())
                .ok()
                .ok_or(AllocError)?
                .size() as HeapAddress
        } else {
            0
        };
        let free_limit: HeapAddress = if let Some(front) = self.entries.front() {
            front.address.start
        } else {
            self.heap.get().len() as HeapAddress
        };
        let new_start = if free_start > free_limit {
            if free_start
                .checked_add(layout.size() as HeapAddress)
                .ok_or(AllocError)?
                < self.heap.get().len() as HeapAddress
            {
                free_start
            } else {
                0
            }
        } else {
            if free_start
                .checked_add(layout.size() as HeapAddress)
                .ok_or(AllocError)?
                < free_limit
            {
                free_start
            } else {
                return Err(AllocError);
            }
        };
        Ok(new_start..new_start + layout.size() as HeapAddress)
    }
    fn heap_raw_mut(&mut self, index: HeapAddress) -> *mut () {
        unsafe { (self.heap.get() as *mut u8).offset(index as isize) as *mut () }
    }
    fn heap_raw_const(&self, index: HeapAddress) -> *const () {
        unsafe { (self.heap.get() as *const u8).offset(index as isize) as *const () }
    }
    pub fn insert<B: BuilderInPlace + 'static>(&mut self, builder: B) -> Result<HeapRef, AllocError>
    where
        B::Output: HeapObject + Sized,
    {
        unsafe {
            let layout = builder.layout();
            let address = self.find_address(layout)?;
            let result = builder.build(self.heap_raw_mut(address.start));
            let result = result as *mut dyn HeapObject;
            let metadata = metadata(result);
            let heap_index = self.entries.push_back(HeapEntry {
                address,
                refcount: 1,
                metadata,
            })?;
            Ok(HeapRef(heap_index))
        }
    }
    pub fn get(&self, heap_ref: &HeapRef) -> &dyn HeapObject {
        unsafe {
            let address = self.entries[heap_ref.0].address.start;
            let metadata = self.entries[heap_ref.0].metadata;
            let ptr = self.heap_raw_const(address);
            &*ptr::from_raw_parts(ptr, metadata)
        }
    }
    pub fn get_mut(&mut self, heap_ref: &HeapRef) -> &mut dyn HeapObject {
        unsafe {
            let address = self.entries[heap_ref.0].address.start;
            let metadata = self.entries[heap_ref.0].metadata;
            let ptr = self.heap_raw_mut(address);
            &mut *ptr::from_raw_parts_mut(ptr, metadata)
        }
    }
    pub fn get_typed_mut<T: 'static>(&mut self, heap_ref: &HeapRef) -> Result<&mut T, TypeError> {
        Ok((self.get_mut(heap_ref) as &mut dyn Any)
            .downcast_mut::<T>()
            .ok_or(TypeError)?)
    }
    // pub fn try_get<T: 'static + ?Sized>(&self, heap_ref: &HeapRef) -> Result<&T, TypeError>
    // where
    //     <T as Pointee>::Metadata: SmallMetadata,
    // {
    //     unsafe {
    //         if self.entries[heap_ref.0].vtable.type_id() != TypeId::of::<T>() {
    //             return Err(TypeError);
    //         }
    //         let address = self.entries[heap_ref.0].address.start;
    //         let metadata = self.entries[heap_ref.0].metadata;
    //         let ptr = self.heap_raw_const(address);
    //         Ok(&*ptr::from_raw_parts::<T>(
    //             ptr,
    //             <T as Pointee>::Metadata::decode(metadata),
    //         ))
    //     }
    // }
    // pub fn try_get_mut<T: 'static + ?Sized>(
    //     &mut self,
    //     heap_ref: &HeapRef,
    // ) -> Result<&mut T, TypeError>
    // where
    //     <T as Pointee>::Metadata: SmallMetadata,
    // {
    //     unsafe {
    //         if self.entries[heap_ref.0].vtable.type_id() != TypeId::of::<T>() {
    //             return Err(TypeError);
    //         }
    //         let address = self.entries[heap_ref.0].address.start;
    //         let metadata = self.entries[heap_ref.0].metadata;
    //         let ptr = self.heap_raw_mut(address);
    //         Ok(&mut *ptr::from_raw_parts_mut::<T>(
    //             ptr,
    //             <T as Pointee>::Metadata::decode(metadata),
    //         ))
    //     }
    // }
    // pub fn get_dyn(&self, heap_ref: &HeapRef) -> FatRef<'_> {
    //     unsafe {
    //         let address = self.entries[heap_ref.0].address.start;
    //         let metadata = self.entries[heap_ref.0].metadata;
    //         let vtable = self.entries[heap_ref.0].vtable;
    //         let ptr = self.heap_raw_const(address);
    //         FatRef::new(
    //             RawFatPointer {
    //                 data: ptr,
    //                 metadata,
    //             },
    //             vtable,
    //         )
    //     }
    // }

    pub fn clone_ref(&mut self, heap_ref: &HeapRef) -> HeapRef {
        self.entries[heap_ref.0].refcount += 1;
        HeapRef(heap_ref.0)
    }
    pub fn drop_ref(&mut self, heap_ref: HeapRef) {
        self.entries[heap_ref.0].refcount -= 1;
        if self.entries[heap_ref.0].refcount == 0 {
            self.entries.remove(heap_ref.0);
        }
    }
}
