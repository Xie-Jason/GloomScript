use std::alloc;
use std::alloc::Layout;
use std::mem::{ManuallyDrop, MaybeUninit};

use crate::obj::object::GloomObjRef;
use crate::vm::slot::Slot;

pub struct Table {
    ptr: *mut MaybeUninit<Slot>,
}

impl Table {
    pub fn new(len: u16) -> Table {
        let layout = Layout::array::<MaybeUninit<Slot>>(len as usize).unwrap();
        unsafe {
            let ptr: *mut MaybeUninit<Slot> = alloc::alloc(layout) as *mut MaybeUninit<Slot>;
            for i in 0..len {
                ptr.add(i as usize).write(MaybeUninit::new(Slot::Null))
            }
            Table { ptr }
        }
    }
    pub fn dealloc(&mut self, len: u16) {
        let layout = Layout::array::<MaybeUninit<Slot>>(len as usize).unwrap();
        unsafe { alloc::dealloc(self.ptr as *mut u8, layout) }
    }

    #[inline(always)]
    pub fn take_slot_ref(&self, index: u16) -> ManuallyDrop<GloomObjRef> {
        unsafe {
            self.ptr
                .add(index as usize)
                .as_mut()
                .expect("null pointer exception")
                .assume_init_mut()
                .take()
                .into_ref()
        }
    }

    #[inline(always)]
    pub fn slot_mut(&self, index: u16) -> &mut Slot {
        unsafe {
            self.ptr
                .add(index as usize)
                .as_mut()
                .expect("null pointer exception")
                .assume_init_mut()
        }
    }

    #[inline(always)]
    pub fn slot(&self, index: u16) -> &Slot {
        unsafe {
            self.ptr
                .add(index as usize)
                .as_ref()
                .expect("null pointer exception")
                .assume_init_ref()
        }
    }

    #[inline]
    pub fn as_slice<'a>(&self, len: u16) -> &'a mut [MaybeUninit<Slot>] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, len as usize) }
    }
}
