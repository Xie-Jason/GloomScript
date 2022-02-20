use std::any::Any;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};

use crate::builtin::iter::GloomListIter;
use crate::frontend::status::GloomStatus;
use crate::obj::func::GloomFunc;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;
use crate::vm::traceback::{GloomException,raise_exception};

pub struct GloomResult<T>(pub RefCell<std::result::Result<Box<T>,GloomException>>);

impl<T> Debug for GloomResult<T>
    where T:Debug
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{:?}\"", self.0.borrow())
    }
}

impl<T> Object for GloomResult<T> 
    where T:Debug
{
    fn obj_type(&self) -> ObjectType {
        ObjectType::Result
    }
    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn drop_by_vm(&self, _: &GloomVM, _: &GloomObjRef) {}

    fn iter(&self, rf: &GloomObjRef) -> GloomObjRef {
        GloomListIter::new(rf.clone())
    }

    fn at(&self, index: &mut usize) -> Option<Value> {
        raise_exception(GloomException::new_empty_exception())
    }

    fn next(&self) -> Value {
        raise_exception(GloomException::new_empty_exception())
    }

    fn method(&self, index: u16, status: &GloomStatus) -> RefCount<GloomFunc> {
        todo!()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        raise_exception(GloomException::new_empty_exception())
    }
}

impl<T> GloomResult<T> {
    #[inline]
    pub fn new(value: T) -> GloomObjRef {
        todo!();
        //GloomObjRef::new(Rc::new(GloomResult(RefCell::new(value))))
    }
}
