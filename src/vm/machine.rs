use std::mem::ManuallyDrop;
use crate::bytecode::code::ByteCode;
use crate::vm::static_table::StaticTable;
use crate::vm::value::{GloomArgs, Value};
use crate::frontend::status::GloomStatus;
use crate::obj::func::{FuncBody, GloomFunc, GloomFuncObj};
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
        match &func.body {
            FuncBody::Builtin(func) => {
                func(self,args)
            }
            FuncBody::ByteCodes(bytecodes) => {
                let mut frame = Frame::new(func.info.stack_size, func.info.local_size);
                frame.fill_args(&func.info.params,args);
                self.interpret(bytecodes,&mut frame)
            }
            unknown => panic!("unknown func body {:?} of {:?}",unknown,func)
        }
    }
    pub fn call(&self, func_obj : &GloomFuncObj, args : GloomArgs) -> Operand {
        let func = func_obj.func.inner();
        match &func.body {
            FuncBody::Builtin(func) => {
                func(self,args)
            }
            FuncBody::ByteCodes(bytecodes) => {
                let mut frame = Frame::new(func.info.stack_size, func.info.local_size);
                frame.fill_args(&func.info.params,args);
                frame.fill_capture(&func.info.captures,&*func_obj.captures.borrow());
                self.interpret(bytecodes,&mut frame)
            }
            unknown => panic!("unknown func body {:?} of {:?}",unknown,func)
        }
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
                        self.constant_pool.str.get(idx as usize).unwrap().clone()
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
                ByteCode::LoadDirectChar(ch) => {
                    frame.push(Operand::Some(Value::Char(ch)))
                }
                ByteCode::LoadDirectBool(bl) => {
                    frame.push(Operand::Some(Value::Bool(bl)))
                }

                ByteCode::CopyTop => {}
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
                ByteCode::WriteStaticInt(_, _) => {}
                ByteCode::WriteStaticNum(_, _) => {}
                ByteCode::WriteStaticChar(_, _) => {}
                ByteCode::WriteStaticBool(_, _) => {}
                ByteCode::WriteStaticRef(_) => {}
                ByteCode::ReadField(_, _) => {}
                ByteCode::WriteFieldInt(_, _) => {}
                ByteCode::WriteFieldNum(_, _) => {}
                ByteCode::WriteFieldChar(_, _) => {}
                ByteCode::WriteFieldBool(_, _) => {}
                ByteCode::WriteFieldRef(_) => {}
                ByteCode::DropLocal(_) => {}
                ByteCode::NotOp => {}
                ByteCode::NegOp => {}
                ByteCode::Plus => {}
                ByteCode::Sub => {}
                ByteCode::Mul => {}
                ByteCode::Div => {}
                ByteCode::GreaterThan => {}
                ByteCode::LessThan => {}
                ByteCode::GreaterThanEquals => {}
                ByteCode::LessThanEquals => {}
                ByteCode::Equals => {}
                ByteCode::NotEquals => {}
                ByteCode::LogicAnd => {}
                ByteCode::LogicOr => {}
                ByteCode::LoadDirectDefFn(_) => {}
                ByteCode::CallTopFn { .. } => {}
                ByteCode::CallStaticFn { .. } => {}
                ByteCode::CallMethod { .. } => {}
                ByteCode::JumpIf(_) => {}
                ByteCode::Jump(_) => {}
                ByteCode::Return => {}
                ByteCode::CollectTuple(_) => {}
                ByteCode::CollectArray(_, _) => {}
                ByteCode::CollectQueue(_, _) => {}
                ByteCode::Construct(_) => {}
                ByteCode::JumpIfNot(_) => {}
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