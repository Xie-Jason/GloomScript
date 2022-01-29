use std::any::Any;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::obj::object::{GloomObjRef, Object, ObjectType};


pub struct GloomArray(RefCell<RawArray>);

pub enum RawArray {
    IntVec(Vec<i64>),
    NumVec(Vec<f64>),
    CharVec(Vec<char>),
    BoolVec(Vec<bool>),
    RefVec(Vec<GloomObjRef>)
}

impl GloomArray {
    pub fn new( array : RawArray) -> GloomObjRef{
        GloomObjRef::new(Rc::new(
            GloomArray(RefCell::new(array))
        ))
    }
}

impl Debug for GloomArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.0.borrow())
    }
}

impl Object for GloomArray {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Array
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for RawArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RawArray::IntVec(vec) => write!(f,"{:?}",vec),
            RawArray::NumVec(vec) => write!(f,"{:?}",vec),
            RawArray::CharVec(vec) => write!(f,"{:?}",vec),
            RawArray::BoolVec(vec) => write!(f,"{:?}",vec),
            RawArray::RefVec(vec) => write!(f, "{:?}", vec)
        }
    }
}