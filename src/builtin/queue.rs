use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::exec::executor::Executor;
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