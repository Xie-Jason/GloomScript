use std::any::Any;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use crate::builtin::iter::GloomListIter;
use crate::frontend::status::GloomStatus;
use crate::obj::func::GloomFunc;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;

pub struct GloomTuple {
    vec: RefCell<Vec<Value>>,
}

impl Object for GloomTuple {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Tuple
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[inline]
    fn drop_by_vm(&self, vm: &GloomVM, _: &GloomObjRef) {
        for value in self.vec.borrow().iter() {
            if let Value::Ref(rf) = value {
                vm.drop_object(rf);
            }
        }
    }

    fn iter(&self, rf: &GloomObjRef) -> GloomObjRef {
        GloomListIter::new(rf.clone())
    }

    #[inline]
    fn at(&self, index: &mut usize) -> Option<Value> {
        let option = self.vec.borrow().get(*index).map(|val| val.clone());
        *index += 1;
        option
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

impl Debug for GloomTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut string = String::with_capacity(16);
        string.push_str("(");
        for val in self.vec.borrow().iter() {
            string.push_str(format!("{:?}, ",val).as_str())
        }
        if string.len() > 1 {
            string.remove(string.len()-1);
            string.remove(string.len()-1);
        }
        string.push_str(")");
        write!(f, "{}", string)
    }
}

impl GloomTuple {
    pub fn new(vec: Vec<Value>) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomTuple {
            vec: RefCell::new(vec),
        }))
    }
}
