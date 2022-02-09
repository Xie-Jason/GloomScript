use std::any::Any;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::vm::value::Value;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::vm::machine::GloomVM;

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
    #[inline]
    fn drop_by_vm(&self, vm: &GloomVM, _ : &GloomObjRef) {
        for value in self.vec.borrow().iter() {
            if let Value::Ref(rf) = value {
                vm.drop_object(rf);
            }
        }
    }
    #[inline]
    fn at(&self, index : &mut usize) -> Option<Value> {
        let option = self.vec.borrow().get(*index).map(|val| { val.clone() });
        *index += 1;
        option
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