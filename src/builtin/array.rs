extern crate alloc;
use std::any::Any;
use std::cell::RefCell;
use core::fmt::{Debug, Formatter};
use alloc::rc::Rc;
use hashbrown::HashMap;
use crate::builtin::classes::BuiltinClass;

use crate::builtin::iter::GloomListIter;
use crate::frontend::status::GloomStatus;
use crate::obj::func::{GloomFunc, Param, ReturnType};
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;

pub struct GloomArray(pub RefCell<RawArray>);

pub enum RawArray {
    IntVec(Vec<i64>),
    NumVec(Vec<f64>),
    CharVec(Vec<char>),
    BoolVec(Vec<bool>),
    RefVec(Vec<GloomObjRef>),
}

impl GloomArray {
    pub fn new(array: RawArray) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomArray(RefCell::new(array))))
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<Value> {
        match &*self.0.borrow() {
            RawArray::IntVec(vec) => vec.get(index).map(|i| Value::Int(*i)),
            RawArray::NumVec(vec) => vec.get(index).map(|f| Value::Num(*f)),
            RawArray::CharVec(vec) => vec.get(index).map(|c| Value::Char(*c)),
            RawArray::BoolVec(vec) => vec.get(index).map(|b| Value::Bool(*b)),
            RawArray::RefVec(vec) => vec.get(index).map(|rf| Value::Ref(rf.clone())),
        }
    }
}

impl Debug for GloomArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0.borrow())
    }
}

impl Object for GloomArray {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Array
    }
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_vm(&self, vm: &GloomVM, _: &GloomObjRef) {
        if let RawArray::RefVec(vec) = &*self.0.borrow() {
            for rf in vec.iter() {
                vm.drop_object(rf);
            }
        }
    }

    fn iter(&self, rf: &GloomObjRef) -> GloomObjRef {
        GloomListIter::new(rf.clone())
    }

    #[inline]
    fn at(&self, index: &mut usize) -> Option<Value> {
        let option = self.get(*index);
        *index += 1;
        option
    }

    fn next(&self) -> Value {
        todo!()
    }

    fn method(&self, _: u16, _: &GloomStatus) -> RefCount<GloomFunc> {
        todo!()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        panic!()
    }
}

impl Debug for RawArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RawArray::IntVec(vec) => write!(f, "{:?}", vec),
            RawArray::NumVec(vec) => write!(f, "{:?}", vec),
            RawArray::CharVec(vec) => write!(f, "{:?}", vec),
            RawArray::BoolVec(vec) => write!(f, "{:?}", vec),
            RawArray::RefVec(vec) => write!(f, "{:?}", vec),
        }
    }
}

impl BuiltinClass {
    pub fn gloom_array_class() -> BuiltinClass{
        let map = HashMap::new();
        let funcs = Vec::new();

        BuiltinClass {
            name: "Array".to_string(),
            map,
            funcs,
            get_ref_type_fn: Box::new(|option| {
                match option {
                    None => Result::Ok(RefType::Array(Box::new(DataType::Ref(RefType::Any)))),
                    Some(mut vec) => {
                        if vec.len() == 1 {
                            Result::Ok(RefType::Array(Box::new(vec.pop().unwrap())))
                        }else {
                            Result::Err(format!("type Array<T> need only one generic type, found {:?}",vec))
                        }
                    }
                }
            }),
        }
    }
}