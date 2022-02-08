use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use crate::builtin::string::GloomString;
use crate::bytecode::code::ByteCode;
use crate::exec::static_table::StaticTable;
use crate::exec::value::{GloomArgs, Value};
use crate::frontend::status::GloomStatus;
use crate::obj::func::{GloomFunc, GloomFuncObj};
use crate::obj::object::GloomObjRef;
use crate::vm::constant::ConstantPool;
use crate::vm::frame::{Frame, Operand};

pub struct GloomVM{
    static_table  : StaticTable,
    constant_pool : ConstantPool,
    status : GloomStatus,
}

impl GloomVM {
    pub fn new(static_table: StaticTable, constant_pool: ConstantPool, status: GloomStatus) -> Self {
        GloomVM { static_table, constant_pool, status }
    }
    pub fn run(mut self){
        let mut script_bodies = std::mem::replace(
            &mut self.status.script_bodies,
            Vec::with_capacity(0)
        );
        for script in script_bodies.iter_mut() {
            self.call_fn(&script.inner().func,GloomArgs::new(Vec::with_capacity(0)));
        }
    }
    pub fn call_fn(&self, func : &GloomFunc, args : GloomArgs) -> Operand{
        let mut frame = Frame::new(func.info.stack_size, func.info.local_size);
        frame.fill_args(&func.info.params,args);
        let bytecodes = func.body.bytecodes();
        self.interpret(bytecodes,&mut frame)
    }
    pub fn call(&self, func_obj : &GloomFuncObj, args : GloomArgs) -> Operand {
        let func = func_obj.func.inner();
        let mut frame = Frame::new(func.info.stack_size, func.info.local_size);
        frame.fill_args(&func.info.params,args);
        frame.fill_capture(&func.info.captures,&*func_obj.captures.borrow());
        let bytecodes = func.body.bytecodes();
        self.interpret(bytecodes,&mut frame)
    }

    #[inline]
    pub fn interpret(&self, bytecodes : &Vec<ByteCode>, frame : &mut Frame) -> Operand {
        let mut pc : usize = 0;
        let length = bytecodes.len();
        let mut result = Operand::Void;
        while pc < length {
            let code = *bytecodes.get(pc).unwrap();
            pc += 1;
            match code {
                ByteCode::Pop => match frame.pop() {
                    Operand::Some(Value::Ref(rf)) => self.drop_object(&rf),
                    _ => {}
                },
                ByteCode::LoadConstString(idx) => {
                    frame.push(Operand::Some(Value::Ref(
                        GloomObjRef::new(Rc::new(GloomString(RefCell::new(
                            String::clone(self.constant_pool.str.get(idx as usize).unwrap())
                        ))))
                    )));
                }
                ByteCode::LoadDirectInt(_) => {}
                ByteCode::LoadDirectNum(_) => {}
                ByteCode::LoadConstInt(idx) => {
                    frame.push(Operand::Some(Value::Int(
                        *self.constant_pool.int.get(idx as usize).unwrap()
                    )));
                }
                ByteCode::LoadConstNum(idx) => {
                    frame.push(Operand::Some(Value::Num(
                        *self.constant_pool.num.get(idx as usize).unwrap()
                    )));
                }
                ByteCode::LoadConstChar(ch) => {
                    frame.push(Operand::Some(Value::Char(ch)))
                }
                ByteCode::LoadConstBool(bl) => {
                    frame.push(Operand::Some(Value::Bool(bl)))
                }
                ByteCode::LoadClass(_) => {}
                ByteCode::LoadEnum(_) => {}
                ByteCode::LoadBuiltinType(_) => {}
                ByteCode::ReadLocal(_, _) => {}
                ByteCode::WriteLocalInt(_, _) => {}
                ByteCode::WriteLocalNum(_, _) => {}
                ByteCode::WriteLocalChar(_, _) => {}
                ByteCode::WriteLocalBool(_, _) => {}
                ByteCode::WriteLocalRef(_) => {}
                ByteCode::ReadStatic(_, _) => {}
                ByteCode::WriteStatic(_, _) => {}
                ByteCode::ReadField(_, _) => {}
                ByteCode::WriteField(_, _) => {}
                ByteCode::DropLocal(_) => {}
                ByteCode::BinaryOps(_) => {}
                ByteCode::NotOp => {}
                ByteCode::NegOp => {}
                ByteCode::LoadDirectFn(_) => {}
                ByteCode::CallTopFn { .. } => {}
                ByteCode::CallStaticFn { .. } => {}
                ByteCode::CallMethod { .. } => {}
                ByteCode::JumpIf(_) => {}
                ByteCode::Return => {}
                ByteCode::CopyTop => {}

                ByteCode::Jump(_) => {}
            }
        }
        result
    }

    #[inline]
    pub fn drop_object(&self, rf : &GloomObjRef){
        if rf.count() == 1 {
            rf.drop_by_vm(self);
        }
    }
    #[inline]
    pub fn drop_object_manually(&self, mut rf : ManuallyDrop<GloomObjRef>){
        if rf.count() == 1 {
            rf.drop_by_vm(self);
        }
        unsafe { ManuallyDrop::drop(&mut rf); }
    }
}