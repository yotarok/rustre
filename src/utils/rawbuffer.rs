use std::mem;
use std::heap;
use std::heap::{Alloc};
use std::ptr::null_mut;
use std::slice::{from_raw_parts,from_raw_parts_mut};

pub struct RawBuffer {
    pub buffer: *mut u8,
    layout: heap::Layout,
}

impl RawBuffer {
    pub fn new(sz: usize, al: usize) -> RawBuffer {
        unsafe {
            let mut allocator = heap::Heap::default();
            let layout = heap::Layout::from_size_align(sz, al).expect("Invalid layout specified");

            let ptr = allocator.alloc(layout.clone()).expect("Allocation failed");
            RawBuffer {
                buffer: ptr,
                layout: layout
            }
        }
    }

    #[inline(always)]
    pub fn as_slice<T>(&self) -> &[T] {
        unsafe {
            from_raw_parts(mem::transmute(self.buffer), self.layout.size())
        }
    }

    #[inline(always)]
    pub fn as_slice_mut<T>(&mut self) -> &mut [T] {
        unsafe {
            from_raw_parts_mut(mem::transmute(self.buffer), self.layout.size())
        }
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.layout.size()
    }

    #[inline(always)]
    pub fn as_ptr<T>(&self) -> *const T {
        unsafe {
            mem::transmute(self.buffer)
        }
    }

    #[inline(always)]
    pub fn as_mut_ptr<T>(&mut self) -> *mut T {
        unsafe {
            mem::transmute(self.buffer)
        }
    }

}

impl Drop for RawBuffer {
    fn drop(&mut self) {
        if self.buffer != null_mut() {
            unsafe {
                let mut allocator = heap::Heap::default();
                allocator.dealloc(self.buffer, self.layout.clone());
            }
        }
    }
}
