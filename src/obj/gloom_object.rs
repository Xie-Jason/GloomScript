use crate::vm::value::{GloomArgs, Value};
use crate::obj::gloom_class::GloomClass;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::slot::Slot;
use crate::obj::table::Table;
use crate::obj::types::BasicType;
use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use crate::vm::machine::GloomVM;

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

    fn drop_by_vm(&self, vm: &GloomVM, rf : &GloomObjRef) {
        let class = self.class.inner();
        if class.fn_drop_idx < u16::MAX {
            vm.call_fn(
                &*class.funcs.get(class.fn_drop_idx as usize).unwrap().inner(),
                GloomArgs::new(vec![Value::Ref(rf.clone())])
            );
        }
        for idx in class.ref_index_iter() {
            vm.drop_object(self.table.slot(*idx).get_ref());
        }
    }

    fn iter(&self, _ : &GloomObjRef) -> GloomObjRef {
        todo!()
    }

    fn at(&self, _ : &mut usize) -> Option<Value> {
        todo!()
    }

    fn next(&self) -> Option<Value> {
        panic!()
    }
}

impl GloomObject {
    #[inline]
    pub fn new(class: RefCount<GloomClass>) -> GloomObject {
        let size = class.inner().field_indexer.size();
        GloomObject {
            table: Table::new(size),
            class,
        }
    }
    #[inline(always)]
    pub fn read_field(&self, slot_idx: u16, sub_idx: u8, field_type: BasicType) -> Value {
        match field_type {
            BasicType::Int => Value::Int(self.table.slot(slot_idx).get_int(sub_idx)),
            BasicType::Num => Value::Num(self.table.slot(slot_idx).get_num(sub_idx)),
            BasicType::Char => Value::Char(self.table.slot(slot_idx).get_char(sub_idx)),
            BasicType::Bool => Value::Bool(self.table.slot(slot_idx).get_bool(sub_idx)),
            BasicType::Ref => Value::Ref(self.table.slot(slot_idx).get_ref().clone()),
        }
    }
    #[inline(always)]
    pub fn write_field(
        &self,
        slot_idx: u16,
        sub_idx: u8,
        field: Value,
        field_type: BasicType,
    ) -> Option<ManuallyDrop<GloomObjRef>> {
        match field_type {
            BasicType::Int => {
                self.table
                    .slot_mut(slot_idx)
                    .set_int(sub_idx, field.assert_int_include_num());
                Option::None
            }
            BasicType::Num => {
                self.table
                    .slot_mut(slot_idx)
                    .set_num(sub_idx, field.assert_num_include_int());
                Option::None
            }
            BasicType::Char => {
                self.table
                    .slot_mut(slot_idx)
                    .set_char(sub_idx, field.assert_char());
                Option::None
            }
            BasicType::Bool => {
                self.table
                    .slot_mut(slot_idx)
                    .set_bool(sub_idx, field.assert_bool());
                Option::None
            }
            BasicType::Ref => {
                let slot = self
                    .table
                    .slot_mut(slot_idx)
                    .replace(Slot::Ref(ManuallyDrop::new(field.assert_into_ref())));
                match slot {
                    Slot::Null => Option::None,
                    Slot::Ref(rf) => Option::Some(rf),
                    _ => panic!(),
                }
            }
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
        let mut string : String = class.name.deref().clone();
        string.push_str(" { ");
        for (name, (slot_idx, sub_idx, _ , is_fn)) in class.map.iter() {
            if ! *is_fn {
                let field_type = class.field_indexer.get_type(*slot_idx).as_basic();
                string.push_str(name.as_str());
                string.push_str(" : ");
                string.push_str(format!("{:?}",self.read_field(*slot_idx,*sub_idx,field_type)).as_str());
                string.push_str(" , ");
            }
        }
        string.remove(string.len()-2);
        string.push_str("}");
        write!(f,"{}",string.as_str())
    }
}
