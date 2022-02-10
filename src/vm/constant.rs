use crate::obj::func::GloomFunc;
use crate::obj::object::GloomObjRef;
use crate::obj::refcount::RefCount;

pub struct ConstantPool{
    pub int : Vec<i64>,
    pub num : Vec<f64>,
    pub str : Vec<GloomObjRef>,
    pub nameless_fn : Vec<RefCount<GloomFunc>>
}

const CONST_POOL_INIT_CAP : usize = 8;

impl ConstantPool {
    pub fn new() -> ConstantPool{
        ConstantPool{
            int: Vec::with_capacity(CONST_POOL_INIT_CAP),
            num: Vec::with_capacity(CONST_POOL_INIT_CAP),
            str: Vec::with_capacity(CONST_POOL_INIT_CAP),
            nameless_fn : Vec::with_capacity(CONST_POOL_INIT_CAP),
        }
    }
}