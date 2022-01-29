use std::any::{Any};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref};
use std::rc::{Rc, Weak};
use crate::exec::executor::Executor;

#[derive(Clone)]
pub struct GloomObjRef{
    obj : Rc<dyn Object>
}

impl GloomObjRef {
    #[inline(always)]
    pub fn downcast<T : Object + 'static>(&self) -> &T{
        self.obj.deref()
            .as_any()
            .downcast_ref::<T>().unwrap()
    }
    pub fn weak(&self) -> Weak<dyn Object> {
        Rc::downgrade(&self.obj)
    }
    #[inline(always)]
    pub fn new(obj : Rc<dyn Object>) -> GloomObjRef{
        GloomObjRef{
            obj
        }
    }
    #[inline(always)]
    pub fn count(&self) -> usize{
        Rc::strong_count(&self.obj)
    }

    #[inline(always)]
    pub fn obj_type(&self) -> ObjectType{
        self.obj.obj_type()
    }

    #[inline(always)]
    pub fn addr_eqs(&self, other : &GloomObjRef) -> bool{
        Rc::ptr_eq(&self.obj,&other.obj)
    }

    #[inline]
    pub fn drop_by_exec(&self, exec : &Executor){
        self.obj.drop_by_exec(exec)
    }
}

impl Debug for GloomObjRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.obj.deref())
    }
}

pub trait Object : Debug {
    fn obj_type(&self) -> ObjectType;
    fn as_any(&self) -> &dyn Any;
    fn drop_by_exec(&self, exec: &Executor);
}

pub enum ObjectType {
    Int,
    Num,
    Char,
    Bool,
    Func,
    Class,
    MetaClass,
    Enum,
    MetaEnum,
    Interface,
    MetaBuiltinType,
    Type,
    String,
    Array,
    Queue,
    Tuple,
}

// 仅在单线程环境下使用 just use in single-thread env
unsafe impl Send for GloomObjRef {}
unsafe impl Sync for GloomObjRef {}