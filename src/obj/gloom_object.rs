use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use crate::exec::value::Value;
use crate::obj::gloom_class::GloomClass;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::table::Table;
use crate::obj::types::BasicType;

pub struct GloomObject{
    pub table : Table,
    pub class : RefCount<GloomClass>
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
    pub fn new(class : RefCount<GloomClass>, size : u16 ) -> GloomObject{
        GloomObject {
            table: Table::new(size),
            class
        }
    }
    #[inline(always)]
    pub fn read_field(&self, slot_idx : u16,sub_idx : u8, field_type : BasicType) -> Value{
        unsafe {
            match field_type {
                BasicType::Int => Value::Int(self.table.slot(slot_idx).int[sub_idx as usize]),
                BasicType::Num => Value::Num(self.table.slot(slot_idx).num[sub_idx as usize]),
                BasicType::Char => Value::Char(self.table.slot(slot_idx).ch[sub_idx as usize]),
                BasicType::Bool => Value::Bool(self.table.slot(slot_idx).bl[sub_idx as usize]),
                BasicType::Ref => Value::Ref(self.table.slot(slot_idx).rf.deref().clone()),
            }
        }
    }
    #[inline(always)]
    pub fn write_field(&self, slot_idx : u16, sub_idx : u8, field : Value, field_type : BasicType ) -> Option<ManuallyDrop<GloomObjRef>> {
        let sub_idx = sub_idx as usize;
        unsafe {
            match field_type {
                BasicType::Int => {
                    self.table.slot_mut(slot_idx).int[sub_idx] = field.assert_int();
                    Option::None
                }
                BasicType::Num => {
                    self.table.slot_mut(slot_idx).num[sub_idx] = field.assert_num();
                    Option::None
                }
                BasicType::Char => {
                    self.table.slot_mut(slot_idx).ch[sub_idx] = field.assert_char();
                    Option::None
                }
                BasicType::Bool => {
                    self.table.slot_mut(slot_idx).bl[sub_idx] = field.assert_bool();
                    Option::None
                }
                BasicType::Ref => {
                    Option::Some(std::mem::replace(
                        &mut self.table.slot_mut(slot_idx).rf,
                        ManuallyDrop::new(field.assert_into_ref())
                    ))
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
        write!(f,"Object of {:?}",self.class)
    }
}