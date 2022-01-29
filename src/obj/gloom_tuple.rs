use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::exec::value::Value;
use crate::obj::object::{GloomObjRef, Object, ObjectType};

pub struct GloomTuple {
    vec : Vec<Value>
}

impl Object for GloomTuple {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Tuple
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for GloomTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Tuple of {:?}",self.vec)
    }
}

impl GloomTuple {
    pub fn new(vec : Vec<Value>) -> GloomObjRef{
        GloomObjRef::new(Rc::new(
            GloomTuple{ vec }
        ))
    }
}