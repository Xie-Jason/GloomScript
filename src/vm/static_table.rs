use std::mem::ManuallyDrop;

use crate::obj::object::GloomObjRef;
use crate::obj::table::Table;
use crate::obj::types::DataType;
use crate::vm::slot::Slot;
use crate::vm::value::Value;

pub struct StaticTable {
    pub len: u16,
    pub table: Table,
    pub drop_vec: Vec<u16>,
}

impl Drop for StaticTable {
    fn drop(&mut self) {
        self.table.dealloc(self.len)
    }
}

impl StaticTable {
    pub fn new(len: u16, drop_vec: Vec<u16>) -> StaticTable {
        StaticTable {
            len,
            table: Table::new(len),
            drop_vec,
        }
    }
    #[inline(always)]
    pub fn read(&self, slot_idx: u16, sub_idx: u8) -> Value {
        let sub_idx = sub_idx as usize;
        match self.table.slot(slot_idx) {
            Slot::Null => Value::None,
            Slot::Int(int) => Value::Int(int[sub_idx]),
            Slot::Num(num) => Value::Num(num[sub_idx]),
            Slot::Char(ch) => Value::Char(ch[sub_idx]),
            Slot::Bool(bl) => Value::Bool(bl[sub_idx]),
            Slot::Ref(rf) => Value::Ref(GloomObjRef::clone(rf)),
        }
    }
    #[inline(always)]
    pub fn write_int(&self, slot_idx: u16, sub_idx: u8, int: i64) {
        self.table.slot_mut(slot_idx).set_int(sub_idx, int)
    }
    #[inline(always)]
    pub fn write_num(&self, slot_idx: u16, sub_idx: u8, num: f64) {
        self.table.slot_mut(slot_idx).set_num(sub_idx, num);
    }
    #[inline(always)]
    pub fn write_char(&self, slot_idx: u16, sub_idx: u8, ch: char) {
        self.table.slot_mut(slot_idx).set_char(sub_idx, ch);
    }
    #[inline(always)]
    pub fn write_bool(&self, slot_idx: u16, sub_idx: u8, bl: bool) {
        self.table.slot_mut(slot_idx).set_bool(sub_idx, bl);
    }
    #[inline(always)]
    pub fn write_ref(&self, slot_idx: u16, rf: GloomObjRef) -> Option<ManuallyDrop<GloomObjRef>> {
        match self
            .table
            .slot_mut(slot_idx)
            .replace(Slot::Ref(ManuallyDrop::new(rf)))
        {
            Slot::Null => Option::None,
            Slot::Ref(rf) => Option::Some(rf),
            _ => panic!(),
        }
    }
    /*#[inline(always)]
     pub fn read_int(&self, slot_idx : u16, sub_idx : u8) -> i64{
         self.table.slot(slot_idx).get_int(sub_idx)
     }
     #[inline(always)]
     pub fn read_num(&self, slot_idx : u16, sub_idx : u8) -> f64{
         self.table.slot(slot_idx).get_num(sub_idx)
     }
     #[inline(always)]
     pub fn read_char(&self, slot_idx : u16, sub_idx : u8) -> char{
         self.table.slot(slot_idx).get_char(sub_idx)
     }
     #[inline(always)]
     pub fn read_bool(&self, slot_idx : u16, sub_idx : u8) -> bool{
         self.table.slot(slot_idx).get_bool(sub_idx)
     }
     #[inline(always)]
     pub fn read_ref(&self, slot_idx : u16) -> &GloomObjRef {
         self.table.slot(slot_idx).get_ref()
     }
    */
}

pub struct ListIndexer{
    types : Vec<DataType>
}
