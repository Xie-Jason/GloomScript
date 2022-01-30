use std::any::Any;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::exec::executor::Executor;
use crate::exec::value::Value;
use crate::obj::object::{GloomObjRef, Object, ObjectType};

pub struct GloomTuple {
    vec : RefCell<Vec<Value>>
}

impl Object for GloomTuple {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Tuple
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn drop_by_exec(&self, exec: &Executor, _ : &GloomObjRef) {
        for value in self.vec.borrow().iter() {
            if let Value::Ref(rf) = value {
                exec.drop_object(rf);
            }
        }
    }
}

impl Debug for GloomTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Tuple of {:?}",self.vec.borrow())
    }
}

impl GloomTuple {
    pub fn new(vec : Vec<Value>) -> GloomObjRef{
        GloomObjRef::new(Rc::new(
            GloomTuple{ vec : RefCell::new(vec) }
        ))
    }
}