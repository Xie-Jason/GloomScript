use std::rc::Rc;
use crate::obj::object::GloomObjRef;

pub struct ConstantPool{
    pub int : Vec<i64>,
    pub num : Vec<f64>,
    pub str : Vec<GloomObjRef>,
}

const CONST_POOL_INIT_CAP : usize = 8;

impl ConstantPool {
    pub fn new() -> ConstantPool{
        ConstantPool{
            int: Vec::with_capacity(CONST_POOL_INIT_CAP),
            num: Vec::with_capacity(CONST_POOL_INIT_CAP),
            str: Vec::with_capacity(CONST_POOL_INIT_CAP)
        }
    }
}