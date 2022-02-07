use crate::exec::value::Value;
use crate::obj::slot::Slot;

pub struct Frame{
    stack : Vec<Operand>,
    local : Vec<Slot>
}

pub type Operand = Option<Value>;

impl Frame {
    pub fn new(stack_size : u16, local_size : u16) -> Frame {
        Frame{
            stack: Vec::with_capacity(stack_size as usize),
            local: Vec::with_capacity(local_size as usize)
        }
    }
}
