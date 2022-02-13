use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use crate::frontend::status::GloomStatus;
use crate::obj::func::GloomFunc;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;

#[derive(Debug)]
pub struct RangeIter {
    pub end: i64,
    pub step: i64,
    pub curr: Cell<i64>,
}

impl RangeIter {
    pub fn new(start: i64, end: i64, step: i64) -> GloomObjRef {
        GloomObjRef::new(Rc::new(RangeIter {
            end,
            step,
            curr: Cell::new(start),
        }))
    }
}

impl Object for RangeIter {
    fn obj_type(&self) -> ObjectType {
        ObjectType::RangeIter
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_vm(&self, _: &GloomVM, _: &GloomObjRef) {}

    fn iter(&self, _: &GloomObjRef) -> GloomObjRef {
        panic!()
    }

    fn at(&self, _: &mut usize) -> Option<Value> {
        panic!()
    }

    #[inline]
    fn next(&self) -> Value {
        let curr = self.curr.get();
        self.curr.set(self.curr.get() + self.step);
        if curr >= self.end {
            Value::None
        } else {
            Value::Int(curr)
        }
    }

    fn method(&self, _: u16, _: &GloomStatus) -> RefCount<GloomFunc> {
        panic!()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        panic!()
    }
}
