use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::builtin::classes::BuiltinClass;
use crate::exec::executor::Executor;
use crate::exec::value::Value;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;

pub struct BuiltinClassObj{
    pub class : RefCount<BuiltinClass>
}

impl Debug for BuiltinClassObj {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.class)
    }
}

impl Object for BuiltinClassObj {
    fn obj_type(&self) -> ObjectType {
        ObjectType::MetaBuiltinType
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn drop_by_exec(&self, _ : &Executor, _ : &GloomObjRef) {}

    fn at(&self, _ : &mut usize) -> Option<Value> {
        panic!()
    }
}

impl BuiltinClassObj {
    #[inline]
    pub fn new(class : RefCount<BuiltinClass>) -> GloomObjRef{
        GloomObjRef::new(Rc::new(
            BuiltinClassObj {
                class
            }
        ))
    }
}