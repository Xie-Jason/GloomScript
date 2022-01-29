use crate::exec::value::Value;
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
}

impl GloomObject {
    #[inline]
    pub fn new(class: RefCount<GloomClass>, size: u16) -> GloomObject {
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
        write!(f, "Object of {:?}", self.class)
    }
}
