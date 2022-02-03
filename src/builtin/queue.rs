use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::exec::executor::Executor;
use crate::exec::value::Value;
use crate::obj::object::{GloomObjRef, Object, ObjectType};

pub struct GloomQueue(RefCell<RawQueue>);

pub enum RawQueue {
    IntQue(VecDeque<i64>),
    NumQue(VecDeque<f64>),
    CharQue(VecDeque<char>),
    BoolQue(VecDeque<bool>),
    RefQue(VecDeque<GloomObjRef>)
}

impl GloomQueue {
    #[inline]
    pub fn new(queue : RawQueue) -> GloomObjRef {
        GloomObjRef::new(Rc::new(
            GloomQueue(RefCell::new(queue))
        ))
    }
    #[inline]
    pub fn get(&self, index : usize) -> Option<Value>{
        match &*self.0.borrow() {
            RawQueue::IntQue(vec) => vec.get(index).map(|val| { Value::Int(*val) }),
            RawQueue::NumQue(vec) => vec.get(index).map(|val| { Value::Num(*val) }),
            RawQueue::CharQue(vec) => vec.get(index).map(|val| { Value::Char(*val) }),
            RawQueue::BoolQue(vec) => vec.get(index).map(|val| { Value::Bool(*val) }),
            RawQueue::RefQue(vec) => vec.get(index).map(|val| { Value::Ref(val.clone()) }),
        }
    }
}

impl Debug for GloomQueue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.0)
    }
}

impl Object for GloomQueue {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Queue
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_exec(&self, exec: &Executor, _ : &GloomObjRef) {
        if let RawQueue::RefQue(vec) = &*self.0.borrow(){
            for rf in vec.iter() {
                exec.drop_object(rf);
            }
        }
    }

    fn at(&self, index: &mut usize) -> Option<Value> {
        let option = self.get(*index);
        *index += 1;
        option
    }
}

impl Debug for RawQueue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RawQueue::IntQue(que) => write!(f,"{:?}",que),
            RawQueue::NumQue(que) => write!(f,"{:?}",que),
            RawQueue::CharQue(que) => write!(f,"{:?}",que),
            RawQueue::BoolQue(que) => write!(f,"{:?}",que),
            RawQueue::RefQue(que) => write!(f,"{:?}",que)
        }
    }
}