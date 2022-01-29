use std::alloc::{Layout};
use std::alloc;
use std::mem::{ManuallyDrop, MaybeUninit, size_of};
use crate::obj::object::GloomObjRef;
use crate::obj::slot::Slot;

pub struct Table {
    ptr : *mut MaybeUninit<Slot>,
}

impl Table {
    pub fn new(size : u16) -> Table {
        let layout = match Layout::from_size_align(size as usize * size_of::<MaybeUninit<Slot>>(), size_of::<Slot>()){
            Ok(layout) => layout,
            Err(err) => {
                panic!("{}",err)
            }
        };
        unsafe {
            let ptr : *mut MaybeUninit<Slot> = alloc::alloc(layout) as *mut MaybeUninit<Slot>;
            for i in 0..size {
                ptr.add(i as usize).write(MaybeUninit::new(Slot::Null))
            }
            Table {
                ptr
            }
        }
    }
    pub fn dealloc(&mut self, len: u16){
        let layout = match Layout::from_size_align(len as usize * size_of::<MaybeUninit<Slot>>(), size_of::<Slot>()){
            Ok(layout) => layout,
            Err(err) => {
                panic!("{}",err)
            }
        };
        unsafe {
            alloc::dealloc(self.ptr as *mut u8,layout)
        }
    }

    #[inline(always)]
    pub fn take_slot_ref(&self, index : u16) -> ManuallyDrop<GloomObjRef> {
        unsafe {
            self.ptr.add(index as usize)
                .as_mut().expect("null pointer exception")
                .assume_init_mut().take().into_ref()
        }
    }

    #[inline(always)]
    pub fn slot_mut(&self, index : u16) -> &mut Slot{
        unsafe {
            self.ptr.add(index as usize)
                .as_mut().expect("null pointer exception")
                .assume_init_mut()
        }
    }

    #[inline(always)]
    pub fn slot(&self, index : u16) -> &Slot{
        unsafe {
            self.ptr.add(index as usize)
                .as_ref().expect("null pointer exception")
                .assume_init_ref()
        }
    }

    #[inline]
    pub fn as_slice<'a>(&self, len : u16) -> &'a mut [MaybeUninit<Slot>] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr, len as usize)
        }
    }


}