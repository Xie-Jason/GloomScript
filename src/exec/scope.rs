use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use crate::obj::object::{GloomObjRef};
use crate::obj::slot::Slot;

pub struct Scope<'a>{
    slots: &'a mut [MaybeUninit<Slot>]
}

impl<'a> Scope<'a> {
    pub fn from_slice(slice : &'a mut [MaybeUninit<Slot>]) -> Scope<'a>{
        Scope{
            slots: slice
        }
    }
    #[inline(always)]
    pub fn read_int(&self, slot_idx : u16,sub_idx : u8) -> i64{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().int[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_num(&self, slot_idx : u16,sub_idx : u8) -> f64{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().num[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_char(&self, slot_idx : u16,sub_idx : u8) -> char{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().ch[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_bool(&self, slot_idx : u16,sub_idx : u8) -> bool{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().bl[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_ref(&self, slot_idx : u16) -> &GloomObjRef{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().rf.deref()
        }
    }
    #[inline(always)]
    pub fn read_ref_mut(&mut self, slot_idx : u16) ->  &mut GloomObjRef{
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().rf.deref_mut()
        }
    }

    #[inline(always)]
    pub fn write_int(&mut self,slot_idx : u16, sub_idx : u8, int_val : i64){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().int[sub_idx as usize] = int_val;
        }
    }
    #[inline(always)]
    pub fn write_num(&mut self,slot_idx : u16, sub_idx : u8, num_val : f64){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().num[sub_idx as usize] = num_val;
        }
    }
    #[inline(always)]
    pub fn write_char(&mut self,slot_idx : u16, sub_idx : u8, char_val : char){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().ch[sub_idx as usize] = char_val;
        }
    }
    #[inline(always)]
    pub fn write_bool(&mut self,slot_idx : u16, sub_idx : u8, bool_val : bool){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().bl[sub_idx as usize] = bool_val;
        }
    }

    #[inline(always)]
    pub fn write_ref_firstly(&mut self, slot_idx : u16, rf : GloomObjRef){
        self.slots[slot_idx as usize].write(Slot{
            rf : ManuallyDrop::new(rf)
        });
    }
    #[inline(always)]
    pub fn replace_ref(&mut self,slot_idx : u16, rf : GloomObjRef) -> ManuallyDrop<GloomObjRef>{
        unsafe {
            std::mem::replace::<ManuallyDrop<GloomObjRef>>(
                &mut self.slots[slot_idx as usize].assume_init_mut().rf,
                ManuallyDrop::new(rf)
            )
        }
    }
    #[inline(always)]
    pub fn take_ref(&mut self, slot_idx : u16) -> ManuallyDrop<GloomObjRef> {
        unsafe {
            let slot = std::mem::replace::<Slot>(
                self.slots[slot_idx as usize].assume_init_mut(),
                Slot { int: [0, 0] }
            );
            slot.rf
        }
    }
}
