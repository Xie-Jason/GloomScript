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
    pub fn read(&self, slot_idx: u16) -> Value {
        match self.table.slot(slot_idx) {
            Slot::Null => Value::None,
            Slot::Int(int) => Value::Int(int[0]),
            Slot::Num(num) => Value::Num(num[0]),
            Slot::Char(ch) => Value::Char(ch[0]),
            Slot::Bool(bl) => Value::Bool(bl[0]),
            Slot::Ref(rf) => Value::Ref(GloomObjRef::clone(rf)),
        }
    }
    #[inline(always)]
    pub fn is_init(&self, slot_idx: u16) -> bool {
        if let Slot::Null = self.table.slot(slot_idx) {
            false
        } else {
            true
        }
    }
    #[inline(always)]
    pub fn write_int(&self, slot_idx: u16, int: i64) {
        self.table.slot_mut(slot_idx).set_int(0, int)
    }
    #[inline(always)]
    pub fn write_num(&self, slot_idx: u16, num: f64) {
        self.table.slot_mut(slot_idx).set_num(0, num);
    }
    #[inline(always)]
    pub fn write_char(&self, slot_idx: u16, ch: char) {
        self.table.slot_mut(slot_idx).set_char(0, ch);
    }
    #[inline(always)]
    pub fn write_bool(&self, slot_idx: u16, bl: bool) {
        self.table.slot_mut(slot_idx).set_bool(0, bl);
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

pub struct ListIndexer {
    types: Vec<DataType>,
}

impl ListIndexer {
    pub fn put(&mut self, typ: DataType) -> u16 {
        let i = self.types.len() as u16;
        self.types.push(typ);
        i
    }

    pub fn get_type(&self, index: u16) -> &DataType {
        self.types.get(index as usize).unwrap()
    }

    pub fn new() -> Self {
        ListIndexer { types: vec![] }
    }

    pub fn size(&self) -> u16 {
        self.types.len() as u16
    }

    pub fn drop_vec(&self) -> Vec<u16> {
        let mut drop_vec = Vec::new();
        for (idx, typ) in self.types.iter().enumerate() {
            if let DataType::Ref(_) = typ {
                drop_vec.push(idx as u16);
            }
        }
        drop_vec
    }
}
