use std::fmt::Debug;
use std::mem::ManuallyDrop;
use crate::builtin::obj::BuiltinClassObj;
use crate::bytecode::code::ByteCode;
use crate::vm::static_table::StaticTable;
use crate::vm::value::{GloomArgs, Value};
use crate::frontend::status::GloomStatus;
use crate::obj::func::{FuncBody, GloomFunc, GloomFuncObj};
use crate::obj::gloom_class::GloomClassObj;
use crate::obj::gloom_object::GloomObject;
use crate::obj::object::GloomObjRef;
use crate::vm::constant::ConstantPool;
use crate::vm::frame::{Frame};

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
    pub fn call_fn(&self, func : &GloomFunc, args : GloomArgs) -> Value{
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
    pub fn call(&self, func_obj : &GloomFuncObj, args : GloomArgs) -> Value {
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
    pub fn interpret(&self, bytecodes : &Vec<ByteCode>, frame : &mut Frame) -> Value {
        let mut pc : usize = 0;
        let length = bytecodes.len();
        let mut result = Value::None;
        while pc < length {
            let code = *bytecodes.get(pc).unwrap();
            pc += 1;
            match code {
                ByteCode::Pop => match frame.pop() {
                    Value::Ref(rf) => self.drop_object(&rf),
                    _ => {}
                },
                ByteCode::LoadConstString(idx) => {
                    frame.push(Value::Ref(
                        self.constant_pool.str.get(idx as usize).unwrap().clone()
                    ));
                }
                ByteCode::LoadDirectInt(i) => {
                    frame.push(Value::Int(i as i64));
                }
                ByteCode::LoadDirectNum(n) => {
                    frame.push(Value::Num(n as f64));
                }
                ByteCode::LoadConstInt(idx) => {
                    frame.push(Value::Int(
                        *self.constant_pool.int.get(idx as usize).unwrap()
                    ));
                }
                ByteCode::LoadConstNum(idx) => {
                    frame.push(Value::Num(
                        *self.constant_pool.num.get(idx as usize).unwrap()
                    ));
                }
                ByteCode::LoadDirectChar(ch) => {
                    frame.push(Value::Char(ch));
                }
                ByteCode::LoadDirectBool(bl) => {
                    frame.push(Value::Bool(bl));
                }
                ByteCode::CopyTop => {
                    frame.push(frame.top().clone());
                }
                ByteCode::LoadClass(idx) => {
                    frame.push(Value::Ref(
                        GloomClassObj::new(
                            self.status.classes.get(idx as usize).unwrap().clone()
                        )
                    ));
                }
                ByteCode::LoadEnum(_) => {
                    panic!()
                }
                ByteCode::LoadBuiltinType(idx) => {
                    frame.push(Value::Ref(
                        BuiltinClassObj::new(
                            self.status.builtin_classes.get(idx as usize).unwrap().clone()
                        )
                    ))
                }
                ByteCode::ReadLocal(slot_idx,sub_idx) => {
                    frame.push(frame.read(slot_idx,sub_idx));
                }
                ByteCode::WriteLocalInt(slot_idx, sub_idx) => {
                    frame.write_int(frame.pop().assert_int(),slot_idx,sub_idx);
                }
                ByteCode::WriteLocalNum(slot_idx, sub_idx) => {
                    frame.write_num(frame.pop().assert_num(),slot_idx,sub_idx);
                }
                ByteCode::WriteLocalChar(slot_idx, sub_idx) => {
                    frame.write_char(frame.pop().assert_char(),slot_idx,sub_idx);
                }
                ByteCode::WriteLocalBool(slot_idx, sub_idx) => {
                    frame.write_bool(frame.pop().assert_bool(),slot_idx,sub_idx);
                }
                ByteCode::WriteLocalRef(slot_idx) => {
                    let option
                        = frame.write_ref(frame.pop().assert_into_ref(), slot_idx);
                    self.drop_option_manually(option);
                }
                ByteCode::ReadStatic(slot_idx, sub_idx) => {
                    frame.push(self.static_table.read(slot_idx,sub_idx));
                }
                ByteCode::WriteStaticInt(slot_idx, sub_idx) => {
                    self.static_table.write_int(slot_idx,sub_idx,frame.pop().assert_int());
                }
                ByteCode::WriteStaticNum(slot_idx, sub_idx) => {
                    self.static_table.write_num(slot_idx,sub_idx,frame.pop().assert_num());
                }
                ByteCode::WriteStaticChar(slot_idx, sub_idx) => {
                    self.static_table.write_char(slot_idx,sub_idx,frame.pop().assert_char());
                }
                ByteCode::WriteStaticBool(slot_idx, sub_idx) => {
                    self.static_table.write_bool(slot_idx,sub_idx,frame.pop().assert_bool());
                }
                ByteCode::WriteStaticRef(slot_idx) => {
                    let rf
                        = self.static_table.replace_ref(slot_idx, frame.pop().assert_into_ref());
                    self.drop_object_manually(rf);
                }
                ByteCode::ReadField(slot_idx, sub_idx) => {
                    frame.push(
                        frame.top().as_ref()
                            .downcast::<GloomObject>()
                            .read_field(slot_idx,sub_idx)
                    );
                }
                ByteCode::WriteFieldInt(slot_idx, sub_idx) => {
                    let i = frame.pop().assert_int();
                    frame.top().as_ref()
                        .downcast::<GloomObject>()
                        .write_field_int(slot_idx,sub_idx,i);
                }
                ByteCode::WriteFieldNum(slot_idx, sub_idx) => {
                    let n = frame.pop().assert_num();
                    frame.top().as_ref()
                        .downcast::<GloomObject>()
                        .write_field_num(slot_idx,sub_idx,n);
                }
                ByteCode::WriteFieldChar(slot_idx, sub_idx) => {
                    let c = frame.pop().assert_char();
                    frame.top().as_ref()
                        .downcast::<GloomObject>()
                        .write_field_char(slot_idx,sub_idx,c);
                }
                ByteCode::WriteFieldBool(slot_idx, sub_idx) => {
                    let b = frame.pop().assert_bool();
                    frame.top().as_ref()
                        .downcast::<GloomObject>()
                        .write_field_bool(slot_idx,sub_idx,b);
                }
                ByteCode::WriteFieldRef(slot_idx) => {
                    let rf = frame.pop().assert_into_ref();
                    let option
                        = frame.top().as_ref()
                        .downcast::<GloomObject>()
                        .write_field_ref(slot_idx, rf);
                    self.drop_option_manually(option);
                }
                ByteCode::DropLocal(slot_idx) => {
                    frame.drop_local(self,slot_idx);
                }
                ByteCode::NotOp => {
                    frame.top_mut().not();
                }
                ByteCode::NegOp => {
                    frame.top_mut().neg();
                }
                ByteCode::Plus => {
                    let val = frame.pop();
                    frame.top_mut().plus(val);
                }
                ByteCode::Sub => {
                    let val = frame.pop();
                    frame.top_mut().sub(val);
                }
                ByteCode::Mul => {
                    let val = frame.pop();
                    frame.top_mut().multiply(val);
                }
                ByteCode::Div => {
                    let val = frame.pop();
                    frame.top_mut().divide(val);
                }
                ByteCode::PlusOne => {
                    frame.top_mut().plus_one();
                }
                ByteCode::SubOne => {
                    frame.top_mut().sub_one();
                }
                ByteCode::GreaterThan => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(left.greater_than(right)));
                }
                ByteCode::LessThan => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(left.less_than(right)));
                }
                ByteCode::GreaterThanEquals => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(left.greater_equal(right)));
                }
                ByteCode::LessThanEquals => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(left.less_equal(right)));
                }
                ByteCode::Equals => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(left.equals(right)));
                }
                ByteCode::NotEquals => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(! left.equals(right)));
                }
                ByteCode::LogicAnd => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(right.assert_bool() && left.assert_bool()));
                }
                ByteCode::LogicOr => {
                    let right = frame.pop();
                    let left = frame.pop();
                    frame.push(Value::Bool(right.assert_bool() || left.assert_bool()));
                }
                ByteCode::LoadDirectDefFn(idx) => {
                    frame.push(Value::Ref(
                        GloomFuncObj::new_func(
                            self.status.funcs.get(idx as usize).unwrap().clone()
                        )
                    ));
                }
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
                ByteCode::LoadNamelessFn(_) => {}
                ByteCode::RangeIter => {}
                ByteCode::InvokeIter => {}
                ByteCode::InvokeNext => {}
                ByteCode::JumpIfNone(_) => {}
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
    #[inline]
    pub fn drop_option_manually(&self, mut option : Option<ManuallyDrop<GloomObjRef>>){
        if let Some(mut rf) = option {
            if rf.count() == 1 {
                rf.drop_by_vm(self);
            }
            unsafe { ManuallyDrop::drop(&mut rf); }
        }
    }
}