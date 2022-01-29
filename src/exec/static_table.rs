use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use crate::obj::object::GloomObjRef;
use crate::obj::table::Table;

pub struct StaticTable{
    pub len : u16,
    pub table : Table,
    pub drop_vec : Vec<u16>
}

impl Drop for StaticTable {
    fn drop(&mut self) {
        self.table.dealloc(self.len)
    }
}

impl StaticTable {
    pub fn new(len : u16, drop_vec : Vec<u16>) -> StaticTable{
        StaticTable{
            len,
            table: Table::new(len),
            drop_vec
        }
    }
    #[inline(always)]
    pub fn read_int(&self, slot_idx : u16, sub_idx : u8) -> i64{
        unsafe {
            self.table.slot(slot_idx).int[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_num(&self, slot_idx : u16, sub_idx : u8) -> f64{
        unsafe {
            self.table.slot(slot_idx).num[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_char(&self, slot_idx : u16, sub_idx : u8) -> char{
        unsafe {
            self.table.slot(slot_idx).ch[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_bool(&self, slot_idx : u16, sub_idx : u8) -> bool{
        unsafe {
            self.table.slot(slot_idx).bl[sub_idx as usize]
        }
    }
    #[inline(always)]
    pub fn read_ref(&self, slot_idx : u16) -> &GloomObjRef {
        unsafe {
            self.table.slot(slot_idx).rf.deref()
        }
    }
    #[inline(always)]
    pub fn read_ref_mut(&self, slot_idx : u16) -> &mut GloomObjRef {
        unsafe {
            self.table.slot_mut(slot_idx).rf.deref_mut()
        }
    }
    #[inline(always)]
    pub fn write_int(&self, slot_idx : u16, sub_idx : u8, int : i64){
        unsafe {
            self.table.slot_mut(slot_idx).int[sub_idx as usize] = int;
        }
    }
    #[inline(always)]
    pub fn write_num(&self, slot_idx : u16, sub_idx : u8, num : f64){
        unsafe {
            self.table.slot_mut(slot_idx).num[sub_idx as usize] = num;
        }
    }
    #[inline(always)]
    pub fn write_char(&self, slot_idx : u16, sub_idx : u8, ch : char){
        unsafe {
            self.table.slot_mut(slot_idx).ch[sub_idx as usize] = ch;
        }
    }
    #[inline(always)]
    pub fn write_bool(&self, slot_idx : u16, sub_idx : u8, bl : bool){
        unsafe {
            self.table.slot_mut(slot_idx).bl[sub_idx as usize] = bl;
        }
    }
    #[inline(always)]
    pub fn write_ref_firstly(&self, slot_idx : u16, rf : GloomObjRef){
        self.table.slot_mut(slot_idx).rf = ManuallyDrop::new(rf);
    }
    #[inline(always)]
    pub fn replace_ref(&self, slot_idx : u16, rf : GloomObjRef) -> ManuallyDrop<GloomObjRef> {
        unsafe {
            std::mem::replace::<ManuallyDrop<GloomObjRef>>(
            &mut self.table.slot_mut(slot_idx).rf,
            ManuallyDrop::new(rf)
            )
        }
    }
}