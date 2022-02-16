use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::rc::Rc;

use crate::frontend::status::GloomStatus;
use crate::obj::func::GloomFunc;
use crate::obj::class::GloomClass;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::table::Table;
use crate::vm::machine::GloomVM;
use crate::vm::slot::Slot;
use crate::vm::value::{GloomArgs, Value};

pub struct GloomObject {
    pub table: Table,
    pub class: RefCount<GloomClass>,
}

impl Object for GloomObject {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Class
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_vm(&self, vm: &GloomVM, rf: &GloomObjRef) {
        let class = self.class.inner();
        if class.fn_drop_idx < u16::MAX {
            vm.call_fn(
                &*class.funcs.get(class.fn_drop_idx as usize).unwrap().inner(),
                GloomArgs::new(vec![Value::Ref(rf.clone())]),
            );
        }
        for idx in class.ref_index_iter() {
            vm.drop_object(self.table.slot(*idx).get_ref());
        }
    }

    fn iter(&self, _: &GloomObjRef) -> GloomObjRef {
        todo!()
    }

    fn at(&self, _: &mut usize) -> Option<Value> {
        todo!()
    }

    fn next(&self) -> Value {
        panic!()
    }

    fn method(&self, index: u16, status: &GloomStatus) -> RefCount<GloomFunc> {
        self.class
            .inner()
            .funcs
            .get(index as usize)
            .unwrap()
            .clone()
    }

    fn field(&self, i1: u16, i2: u8) -> Value {
        self.read_field(i1, i2)
    }
}

impl GloomObject {
    #[inline]
    pub fn new(class: RefCount<GloomClass>) -> GloomObjRef {
        let size = class.inner().field_indexer.size();
        GloomObjRef::new(Rc::new(GloomObject {
            table: Table::new(size),
            class,
        }))
    }

    #[inline]
    pub fn read_field(&self, slot_idx: u16, sub_idx: u8) -> Value {
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

    #[inline]
    pub fn write_field_int(&self, slot_idx: u16, sub_idx: u8, val: i64) {
        self.table.slot_mut(slot_idx).set_int(sub_idx, val);
    }
    #[inline]
    pub fn write_field_num(&self, slot_idx: u16, sub_idx: u8, val: f64) {
        self.table.slot_mut(slot_idx).set_num(sub_idx, val);
    }
    #[inline]
    pub fn write_field_char(&self, slot_idx: u16, sub_idx: u8, val: char) {
        self.table.slot_mut(slot_idx).set_char(sub_idx, val);
    }
    #[inline]
    pub fn write_field_bool(&self, slot_idx: u16, sub_idx: u8, val: bool) {
        self.table.slot_mut(slot_idx).set_bool(sub_idx, val);
    }
    #[inline]
    pub fn write_field_ref(
        &self,
        slot_idx: u16,
        val: GloomObjRef,
    ) -> Option<ManuallyDrop<GloomObjRef>> {
        match self
            .table
            .slot_mut(slot_idx)
            .replace(Slot::Ref(ManuallyDrop::new(val)))
        {
            Slot::Ref(rf) => Option::Some(rf),
            Slot::Null => Option::None,
            slot => panic!("{:?}", slot),
        }
    }
}

impl Drop for GloomObject {
    fn drop(&mut self) {
        self.table.dealloc(self.class.inner().len())
    }
}

impl Debug for GloomObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let class = self.class.inner();
        let mut string: String = class.name.deref().clone();
        string.push_str(" { ");
        for (name, (slot_idx, sub_idx, _, is_fn)) in class.map.iter() {
            if !*is_fn {
                string.push_str(name.as_str());
                string.push_str(" : ");
                string.push_str(format!("{:?}", self.read_field(*slot_idx, *sub_idx)).as_str());
                string.push_str(" , ");
            }
        }
        string.remove(string.len() - 2);
        string.push_str("}");
        write!(f, "{}", string.as_str())
    }
}
