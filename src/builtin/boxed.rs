use std::any::Any;
use std::cell::Cell;
use core::fmt::{Debug, Formatter};
extern crate alloc;
use alloc::rc::Rc;

use crate::frontend::status::GloomStatus;
use crate::obj::func::GloomFunc;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;

pub struct GloomInt(pub Cell<i64>);

pub struct GloomNum(pub Cell<f64>);

pub struct GloomChar(pub Cell<char>);

pub struct GloomBool(pub Cell<bool>);

impl Object for GloomInt {
    #[inline(always)]
    fn obj_type(&self) -> ObjectType {
        ObjectType::Int
    }
    #[inline(always)]
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

    fn next(&self) -> Value {
        panic!()
    }

    fn method(&self, index: u16, status: &GloomStatus) -> RefCount<GloomFunc> {
        todo!()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        panic!()
    }
}

impl Debug for GloomInt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Int({})", self.0.get())
    }
}

impl GloomInt {
    #[inline(always)]
    pub fn new(i: i64) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomInt(Cell::new(i))))
    }
}

impl Object for GloomNum {
    #[inline(always)]
    fn obj_type(&self) -> ObjectType {
        ObjectType::Num
    }
    #[inline(always)]
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

    fn next(&self) -> Value {
        panic!()
    }

    fn method(&self, index: u16, status: &GloomStatus) -> RefCount<GloomFunc> {
        todo!()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        panic!()
    }
}

impl Debug for GloomNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Num({})", self.0.get())
    }
}

impl GloomNum {
    #[inline(always)]
    pub fn new(i: f64) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomNum(Cell::new(i))))
    }
}

impl Object for GloomChar {
    #[inline(always)]
    fn obj_type(&self) -> ObjectType {
        ObjectType::Char
    }
    #[inline(always)]
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

    fn next(&self) -> Value {
        panic!()
    }
    fn method(&self, index: u16, status: &GloomStatus) -> RefCount<GloomFunc> {
        todo!()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        panic!()
    }
}

impl Debug for GloomChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Char({})", self.0.get())
    }
}

impl GloomChar {
    #[inline(always)]
    pub fn new(i: char) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomChar(Cell::new(i))))
    }
}

impl Object for GloomBool {
    #[inline(always)]
    fn obj_type(&self) -> ObjectType {
        ObjectType::Bool
    }
    #[inline(always)]
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

    fn next(&self) -> Value {
        panic!()
    }

    fn method(&self, index: u16, status: &GloomStatus) -> RefCount<GloomFunc> {
        todo!()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        panic!()
    }
}

impl GloomBool {
    #[inline(always)]
    pub fn new(i: bool) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomBool(Cell::new(i))))
    }
}

impl Debug for GloomBool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bool({})", self.0.get())
    }
}
