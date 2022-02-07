use std::rc::Rc;

pub struct ConstantPool{
    int : Vec<i64>,
    num : Vec<f64>,
    str : Vec<Rc<String>>,
}

impl ConstantPool {
    pub fn new() -> ConstantPool{
        ConstantPool{
            int: vec![],
            num: vec![],
            str: vec![]
        }
    }
}