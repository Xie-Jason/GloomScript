use std::mem::{ManuallyDrop, MaybeUninit};
use crate::obj::object::{GloomObjRef};
use crate::obj::slot::Slot;

pub struct Scope<'a>{
    slots: &'a mut [MaybeUninit<Slot>]
}

impl<'a> Scope<'a> {
    pub fn from_slice(slice : &'a mut [MaybeUninit<Slot>]) -> Scope<'a>{
        for slot in slice.iter_mut() {
            slot.write(Slot::Null);
        }
        Scope{
            slots: slice
        }
    }
    #[inline(always)]
    pub fn read_int(&self, slot_idx : u16,sub_idx : u8) -> i64{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().get_int(sub_idx)
        }
    }
    #[inline(always)]
    pub fn read_num(&self, slot_idx : u16,sub_idx : u8) -> f64{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().get_num(sub_idx)
        }
    }
    #[inline(always)]
    pub fn read_char(&self, slot_idx : u16,sub_idx : u8) -> char{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().get_char(sub_idx)
        }
    }
    #[inline(always)]
    pub fn read_bool(&self, slot_idx : u16,sub_idx : u8) -> bool{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().get_bool(sub_idx)
        }
    }
    #[inline(always)]
    pub fn read_ref(&self, slot_idx : u16) -> &GloomObjRef{
        unsafe {
            self.slots[slot_idx as usize].assume_init_ref().get_ref()
        }
    }

    #[inline(always)]
    pub fn write_int(&mut self,slot_idx : u16, sub_idx : u8, int_val : i64){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().set_int(sub_idx,int_val);
        }
    }
    #[inline(always)]
    pub fn write_num(&mut self,slot_idx : u16, sub_idx : u8, num_val : f64){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().set_num(sub_idx,num_val);
        }
    }
    #[inline(always)]
    pub fn write_char(&mut self,slot_idx : u16, sub_idx : u8, char_val : char){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().set_char(sub_idx,char_val);
        }
    }
    #[inline(always)]
    pub fn write_bool(&mut self,slot_idx : u16, sub_idx : u8, bool_val : bool){
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().set_bool(sub_idx ,bool_val);
        }
    }

    #[inline(always)]
    pub fn write_ref_firstly(&mut self, slot_idx : u16, rf : GloomObjRef){
        self.slots[slot_idx as usize].write(Slot::Ref(
            ManuallyDrop::new(rf)
        ));
    }
    #[inline(always)]
    pub fn replace_ref(&mut self,slot_idx : u16, rf : GloomObjRef) -> ManuallyDrop<GloomObjRef>{
        unsafe {
            self.slots[slot_idx as usize]
                .assume_init_mut()
                .replace(Slot::Ref(ManuallyDrop::new(rf)))
                .into_ref()
        }
    }
    #[inline(always)]
    pub fn take_ref(&mut self, slot_idx : u16) -> ManuallyDrop<GloomObjRef> {
        unsafe {
            self.slots[slot_idx as usize].assume_init_mut().take().into_ref()
        }
    }
}
