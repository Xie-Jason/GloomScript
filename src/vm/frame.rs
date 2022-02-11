use std::fmt::{Debug, Formatter};
use std::mem::ManuallyDrop;
use crate::vm::value::{GloomArgs, Value};
use crate::obj::func::{Capture, Param};
use crate::obj::object::GloomObjRef;
use crate::vm::slot::Slot;
use crate::obj::types::{BasicType, DataType};
use crate::vm::machine::GloomVM;

pub struct Frame{
    stack : Vec<Value>,
    local : Box<[Slot]>
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"stack{:?} | local{:?}",self.stack,self.local)
    }
}

impl Frame {
    #[inline]
    pub fn new(stack_size : u16, local_size : u16) -> Frame {
        let mut vec: Vec<Slot> = Vec::with_capacity(local_size as usize);
        unsafe {
            // init the local scope
            let ptr = vec.as_mut_ptr();
            for idx in 0..vec.capacity() {
                // vec.push(Slot::Null); this will change the length
                ptr.add(idx).write(Slot::Null);
            }
            vec.set_len(vec.capacity())
        }

        Frame{
            stack : Vec::with_capacity(stack_size as usize),
            local : vec.into_boxed_slice()
        }
    }
    #[inline]
    pub fn fill_args(&mut self, params : &Vec<Param>, args : GloomArgs){
        for (param, arg) in params.iter().zip(args.vec.into_iter()) {
            let (slot_idx,sub_idx) = param.index;
            match &param.data_type {
                DataType::Int => self.write_int(arg.assert_int(),slot_idx,sub_idx),
                DataType::Num => self.write_num(arg.assert_num(),slot_idx,sub_idx),
                DataType::Char => self.write_char(arg.assert_char(),slot_idx,sub_idx),
                DataType::Bool => self.write_bool(arg.assert_bool(),slot_idx,sub_idx),
                DataType::Ref(_) => {
                    let option = self.write_ref(arg.assert_into_ref(), slot_idx);
                    debug_assert!(option.is_none())
                }
            }
        }
    }
    #[inline]
    pub fn fill_capture(&mut self, captures : &Vec<Capture> , captured : &Vec<Value>){
        for (capture, val) in captures.iter().zip(captured.iter()) {
            let (slot_idx,sub_idx) = (capture.to_slot_idx,capture.to_sub_idx);
            match capture.basic_type {
                BasicType::Int => self.write_int(val.assert_int(),slot_idx,sub_idx),
                BasicType::Num => self.write_num(val.assert_num(),slot_idx,sub_idx),
                BasicType::Char => self.write_char(val.assert_char(),slot_idx,sub_idx),
                BasicType::Bool => self.write_bool(val.assert_bool(),slot_idx,sub_idx),
                BasicType::Ref => {
                    let option = self.write_ref(val.clone().assert_into_ref(), slot_idx);
                    debug_assert!(option.is_none())
                }
            }
        }
    }
    #[inline]
    pub fn stack_not_empty(&self) -> bool{
        ! self.stack.is_empty()
    }
    #[inline(always)]
    pub fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }
    #[inline(always)]
    pub fn push(&mut self, val : Value){
        self.stack.push(val);
    }

    #[inline(always)]
    pub fn top(&self) -> &Value{
        self.stack.last().unwrap()
    }

    #[inline(always)]
    pub fn top_mut(&mut self) -> &mut Value{
        self.stack.last_mut().unwrap()
    }

    #[inline]
    pub fn drop_local(&mut self, vm : &GloomVM, slot_idx : u16){
        let slot = self.local.get_mut(slot_idx as usize).unwrap().replace(Slot::Null);
        match slot {
            Slot::Null => {}
            Slot::Ref(rf) => {
                vm.drop_object_manually(rf);
            }
            slot => panic!("{:?}",slot)
        }
    }

    pub fn print_stack(&self){
        println!("{:?}",self.stack);
    }

    /*#[inline]
    pub fn read_int(&self, slot_idx : u16, sub_idx : u8) -> i64{
        self.local[slot_idx as usize].get_int(sub_idx)
    }
    #[inline]
    pub fn read_num(&self, slot_idx : u16, sub_idx : u8) -> f64{
        self.local[slot_idx as usize].get_num(sub_idx)
    }
    #[inline]
    pub fn read_char(&self, slot_idx : u16, sub_idx : u8) -> char{
        self.local[slot_idx as usize].get_char(sub_idx)
    }
    #[inline]
    pub fn read_bool(&self, slot_idx : u16, sub_idx : u8) -> bool{
        self.local[slot_idx as usize].get_bool(sub_idx)
    }
    #[inline]
    pub fn read_ref(&self, slot_idx : u16) -> GloomObjRef{
        self.local[slot_idx as usize].get_ref().clone()
    }*/
    #[inline]
    pub fn read(&self, slot_idx : u16, sub_idx : u8) -> Value{
        match &self.local[slot_idx as usize] {
            Slot::Null => Value::None,
            Slot::Int(val) => Value::Int(val[sub_idx as usize]),
            Slot::Num(val) => Value::Num(val[sub_idx as usize]),
            Slot::Char(val) => Value::Char(val[sub_idx as usize]),
            Slot::Bool(val) => Value::Bool(val[sub_idx as usize]),
            Slot::Ref(val) => Value::Ref(GloomObjRef::clone(val))
        }
    }
    #[inline]
    pub fn write_int(&mut self, val : i64, slot_idx : u16, sub_idx : u8){
        self.local[slot_idx as usize].set_int(sub_idx,val);
    }
    #[inline]
    pub fn write_num(&mut self, val : f64, slot_idx : u16, sub_idx : u8){
        self.local[slot_idx as usize].set_num(sub_idx,val);
    }
    #[inline]
    pub fn write_char(&mut self, val : char, slot_idx : u16, sub_idx : u8){
        self.local[slot_idx as usize].set_char(sub_idx,val);
    }
    #[inline]
    pub fn write_bool(&mut self, val : bool, slot_idx : u16, sub_idx : u8){
        self.local[slot_idx as usize].set_bool(sub_idx,val);
    }
    #[inline]
    pub fn write_ref(&mut self, val : GloomObjRef, slot_idx : u16) -> Option<ManuallyDrop<GloomObjRef>>{
        match self.local[slot_idx as usize].replace(Slot::Ref(ManuallyDrop::new(val))) {
            Slot::Null => Option::None,
            Slot::Ref(rf) => Option::Some(rf),
            _ => panic!()
        }
    }
}
