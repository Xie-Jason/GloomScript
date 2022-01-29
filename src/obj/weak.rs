use std::fmt::{Debug, Formatter};
use std::rc::{Weak};
use crate::obj::object::{Object};
use crate::obj::types::DataType;

pub struct GloomWeak {
    wk : Weak<dyn Object>,
    generic : DataType
}

impl Debug for GloomWeak {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Object of Weak<{:?}>",self.generic)
    }
}


// 仅在单线程环境下使用 just use in single-thread env
unsafe impl Send for GloomWeak {}
unsafe impl Sync for GloomWeak {}