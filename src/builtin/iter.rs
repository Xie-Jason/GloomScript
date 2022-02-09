use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use crate::vm::value::Value;
use crate::obj::object::GloomObjRef;

pub struct GloomIter {
    rf : GloomObjRef,
    curr : Cell<usize>
}

impl GloomIter {
    pub fn new(rf : GloomObjRef) -> GloomIter {
        GloomIter{
            rf,
            curr: Cell::new(0)
        }
    }
}

impl Debug for GloomIter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?} #{}",self.rf,self.curr.get())
    }
}

impl Iterator for GloomIter {
    type Item = Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut index = self.curr.get();
        let option = self.rf.at(&mut index);
        self.curr.set(index);
        option
    }
}
