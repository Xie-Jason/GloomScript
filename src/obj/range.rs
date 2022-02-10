use std::any::Any;
use std::cell::Cell;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;

#[derive(Debug)]
pub struct RangeIter{
    pub end : i64,
    pub step : i64,
    pub curr : Cell<i64>,
}

impl Object for RangeIter{
    fn obj_type(&self) -> ObjectType {
        ObjectType::RangeIter
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_vm(&self, _ : &GloomVM, _ : &GloomObjRef) {}

    fn iter(&self, _ : &GloomObjRef) -> GloomObjRef {
        panic!()
    }

    fn at(&self, _ : &mut usize) -> Option<Value> {
        panic!()
    }

    fn next(&self) -> Option<Value> {
        self.curr.set(self.curr.get() + 1);
        let curr = self.curr.get();
        if curr >= self.end {
            Option::None
        }else {
            Option::Some(Value::Int(curr))
        }
    }
}