use std::any::{Any};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref};
use std::rc::{Rc, Weak};
use crate::vm::frame::Operand;
use crate::vm::value::Value;
use crate::vm::machine::GloomVM;

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

    #[inline(always)]
    pub fn drop_by_vm(&self, vm : &GloomVM){
        self.obj.drop_by_vm(vm,self);
    }

    #[inline(always)]
    pub fn at(&self, index :&mut usize) -> Option<Value>{
        self.obj.at(index)
    }

    #[inline(always)]
    pub fn iter(&self) -> GloomObjRef {
        self.obj.iter(self)
    }
    #[inline(always)]
    pub fn next(&self) -> Operand{
        match self.obj.next() {
            None => Operand::Void,
            Some(val) => Operand::Some(val)
        }
    }
}

impl Debug for GloomObjRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.obj.deref())
    }
}


pub trait Object : Debug {
    // any type should impl this two fn
    fn obj_type(&self) -> ObjectType;
    fn as_any(&self) -> &dyn Any;
    // any type have Gloom type should impl
    fn drop_by_vm(&self, vm : &GloomVM, rf: &GloomObjRef);
    // a type could return a iterator should impl
    fn iter(&self, rf : &GloomObjRef) -> GloomObjRef;
    // list collection should impl
    fn at(&self, index : &mut usize) -> Option<Value>;
    // iter type should impl
    fn next(&self) -> Option<Value>;
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
    ListIter,
    RangeIter,
}