use std::collections::VecDeque;
use std::mem::{transmute, ManuallyDrop};
use std::ops::Deref;

use crate::builtin::array::{GloomArray, RawArray};
use crate::builtin::obj::BuiltinClassObj;
use crate::builtin::queue::{GloomQueue, RawQueue};
use crate::bytecode::code::ByteCode;
use crate::frontend::status::GloomStatus;
use crate::obj::func::{FuncBody, GloomFunc, GloomFuncObj};
use crate::obj::class::GloomClassObj;
use crate::obj::gloom_enum::GloomEnum;
use crate::obj::gloom_object::GloomObject;
use crate::obj::tuple::GloomTuple;
use crate::obj::object::{GloomObjRef, ObjectType};
use crate::obj::range::RangeIter;
use crate::obj::refcount::RefCount;
use crate::obj::types::BasicType;
use crate::vm::constant::ConstantPool;
use crate::vm::frame::Frame;
use crate::vm::static_table::StaticTable;
use crate::vm::value::{GloomArgs, Value};

pub struct GloomVM {
    static_table: StaticTable,
    constant_pool: ConstantPool,
    status: GloomStatus,
}

impl GloomVM {
    pub fn new(
        static_table: StaticTable,
        constant_pool: ConstantPool,
        status: GloomStatus,
    ) -> Self {
        GloomVM {
            static_table,
            constant_pool,
            status,
        }
    }
    pub fn run(mut self) {
        let mut script_bodies =
            std::mem::replace(&mut self.status.script_bodies, Vec::with_capacity(0));
        for script in script_bodies.iter_mut() {
            self.call_fn(&script.inner().func, GloomArgs::new(Vec::with_capacity(0)));
        }
    }
    pub fn call_fn(&self, func: &GloomFunc, args: GloomArgs) -> Value {
        match &func.body {
            FuncBody::Builtin(func) => func(self, args),
            FuncBody::ByteCodes(bytecodes) => {
                let mut frame = Frame::new(func.info.stack_size, func.info.local_size);
                frame.fill_args(&func.info.params, args);
                /*for (idx,code) in bytecodes.iter().enumerate() {
                    println!("#{:3} {:?}",idx,code);
                }*/
                let value = self.interpret(bytecodes, &mut frame);
                for idx in func.info.drop_slots.iter() {
                    frame.drop_local(self, *idx);
                }
                value
            }
            FuncBody::Jit(ptr) => {
                let func = unsafe { transmute::<_, extern "C" fn(GloomArgs) -> Value>(ptr) };
                func(args)
            }
            unknown => panic!("unknown func body {:?} of {:?}", unknown, func),
        }
    }
    pub fn call(&self, func_obj: &GloomFuncObj, args: GloomArgs) -> Value {
        let func = func_obj.func.inner();
        match &func.body {
            FuncBody::Builtin(func) => func(self, args),
            FuncBody::ByteCodes(bytecodes) => {
                let mut frame = Frame::new(func.info.stack_size, func.info.local_size);
                frame.fill_args(&func.info.params, args);
                frame.fill_capture(&func.info.captures, &*func_obj.captures.borrow());
                let value = self.interpret(bytecodes, &mut frame);
                for idx in func.info.drop_slots.iter() {
                    frame.drop_local(self, *idx);
                }
                value
            }
            FuncBody::Jit(ptr) => {
                let func = unsafe { transmute::<_, extern "C" fn(GloomArgs) -> Value>(ptr) };
                func(args)
            }
            unknown => panic!("unknown func body {:?} of {:?}", unknown, func),
        }
    }

    #[inline]
    pub fn interpret(&self, bytecodes: &Vec<ByteCode>, frame: &mut Frame) -> Value {
        let mut pc: usize = 0;
        let length = bytecodes.len();
        let mut result = Value::None;
        while pc < length {
            let code = *bytecodes.get(pc).unwrap();
            // frame.print_stack();
            pc += 1;
            match code {
                ByteCode::Pop => match frame.pop() {
                    Value::Ref(rf) => self.drop_object(&rf),
                    _ => {}
                },
                ByteCode::LoadConstString(idx) => {
                    frame.push(Value::Ref(
                        self.constant_pool.str.get(idx as usize).unwrap().clone(),
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
                        *self.constant_pool.int.get(idx as usize).unwrap(),
                    ));
                }
                ByteCode::LoadConstNum(idx) => {
                    frame.push(Value::Num(
                        *self.constant_pool.num.get(idx as usize).unwrap(),
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
                    frame.push(Value::Ref(GloomClassObj::new(
                        self.status.classes.get(idx as usize).unwrap().clone(),
                    )));
                }
                ByteCode::LoadEnum(_) => {
                    panic!()
                }
                ByteCode::LoadBuiltinType(idx) => frame.push(Value::Ref(BuiltinClassObj::new(
                    self.status
                        .builtin_classes
                        .get(idx as usize)
                        .unwrap()
                        .clone(),
                ))),
                ByteCode::ReadLocal(slot_idx, sub_idx) => {
                    frame.push(frame.read(slot_idx, sub_idx));
                }
                ByteCode::WriteLocalInt(slot_idx, sub_idx) => {
                    let i = frame.pop().assert_int();
                    frame.write_int(i, slot_idx, sub_idx);
                }
                ByteCode::WriteLocalNum(slot_idx, sub_idx) => {
                    let n = frame.pop().assert_num();
                    frame.write_num(n, slot_idx, sub_idx);
                }
                ByteCode::WriteLocalChar(slot_idx, sub_idx) => {
                    let c = frame.pop().assert_char();
                    frame.write_char(c, slot_idx, sub_idx);
                }
                ByteCode::WriteLocalBool(slot_idx, sub_idx) => {
                    let b = frame.pop().assert_bool();
                    frame.write_bool(b, slot_idx, sub_idx);
                }
                ByteCode::WriteLocalRef(slot_idx) => {
                    let rf = frame.pop().assert_into_ref();
                    let option = frame.write_ref(rf, slot_idx);
                    self.drop_option_manually(option);
                }
                ByteCode::ReadStatic(slot_idx, sub_idx) => {
                    frame.push(self.static_table.read(slot_idx, sub_idx));
                }
                ByteCode::WriteStaticInt(slot_idx, sub_idx) => {
                    self.static_table
                        .write_int(slot_idx, sub_idx, frame.pop().assert_int());
                }
                ByteCode::WriteStaticNum(slot_idx, sub_idx) => {
                    self.static_table
                        .write_num(slot_idx, sub_idx, frame.pop().assert_num());
                }
                ByteCode::WriteStaticChar(slot_idx, sub_idx) => {
                    self.static_table
                        .write_char(slot_idx, sub_idx, frame.pop().assert_char());
                }
                ByteCode::WriteStaticBool(slot_idx, sub_idx) => {
                    self.static_table
                        .write_bool(slot_idx, sub_idx, frame.pop().assert_bool());
                }
                ByteCode::WriteStaticRef(slot_idx) => {
                    let rf = self
                        .static_table
                        .write_ref(slot_idx, frame.pop().assert_into_ref());
                    self.drop_option_manually(rf);
                }
                ByteCode::ReadField(slot_idx, sub_idx) => {
                    frame.push(frame.top().as_ref().read_field(slot_idx, sub_idx));
                }
                ByteCode::ReadFieldAndPop(slot_idx, sub_idx) => {
                    let field = frame.pop().assert_into_ref().read_field(slot_idx, sub_idx);
                    frame.push(field);
                }
                ByteCode::WriteFieldInt(slot_idx, sub_idx) => {
                    let i = frame.pop().assert_int();
                    frame
                        .top()
                        .as_ref()
                        .downcast::<GloomObject>()
                        .write_field_int(slot_idx, sub_idx, i);
                }
                ByteCode::WriteFieldNum(slot_idx, sub_idx) => {
                    let n = frame.pop().assert_num();
                    frame
                        .top()
                        .as_ref()
                        .downcast::<GloomObject>()
                        .write_field_num(slot_idx, sub_idx, n);
                }
                ByteCode::WriteFieldChar(slot_idx, sub_idx) => {
                    let c = frame.pop().assert_char();
                    frame
                        .top()
                        .as_ref()
                        .downcast::<GloomObject>()
                        .write_field_char(slot_idx, sub_idx, c);
                }
                ByteCode::WriteFieldBool(slot_idx, sub_idx) => {
                    let b = frame.pop().assert_bool();
                    frame
                        .top()
                        .as_ref()
                        .downcast::<GloomObject>()
                        .write_field_bool(slot_idx, sub_idx, b);
                }
                ByteCode::WriteFieldRef(slot_idx) => {
                    let rf = frame.pop().assert_into_ref();
                    let option = frame
                        .top()
                        .as_ref()
                        .downcast::<GloomObject>()
                        .write_field_ref(slot_idx, rf);
                    self.drop_option_manually(option);
                }
                ByteCode::DropLocal(slot_idx) => {
                    frame.drop_local(self, slot_idx);
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
                    frame.push(Value::Bool(!left.equals(right)));
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
                    frame.push(Value::Ref(GloomFuncObj::new_func(
                        self.status.funcs.get(idx as usize).unwrap().clone(),
                    )));
                }
                ByteCode::LoadNamelessFn(idx) => {
                    let func_ref = self
                        .constant_pool
                        .nameless_fn
                        .get(idx as usize)
                        .unwrap()
                        .clone();
                    let mut captured;
                    {
                        let func = func_ref.inner_mut();
                        captured = Vec::with_capacity(func.info.captures.len());
                        for capture in func.info.captures.iter() {
                            captured.push(frame.read(capture.from_slot_idx, capture.from_sub_idx));
                        }
                    }
                    frame.push(Value::Ref(GloomFuncObj::new_closure(func_ref, captured)));
                }
                ByteCode::CallTopFn { nargs } => {
                    let mut args = Vec::with_capacity(nargs as usize);
                    for _ in 0..nargs {
                        args.push(frame.pop());
                    }
                    args.reverse();
                    let func_rf = frame.pop().assert_into_ref();
                    let func = func_rf.downcast::<GloomFuncObj>();
                    let result = self.call(func, GloomArgs::new(args));
                    frame.push(result);
                }
                ByteCode::CallStaticFn { index, nargs } => {
                    let mut args = Vec::with_capacity(nargs as usize);
                    for _ in 0..nargs {
                        args.push(frame.pop());
                    }
                    args.reverse();
                    let rf = frame.pop().assert_into_ref();
                    let func: RefCount<GloomFunc>;
                    match rf.obj_type() {
                        ObjectType::Class => {
                            let gloom_obj = rf.downcast::<GloomObject>();
                            func = gloom_obj
                                .class
                                .inner()
                                .funcs
                                .get(index as usize)
                                .unwrap()
                                .clone();
                        }
                        ObjectType::Enum => {
                            let enum_obj = rf.downcast::<GloomEnum>();
                            func = enum_obj
                                .class
                                .inner()
                                .funcs
                                .get(index as usize)
                                .unwrap()
                                .clone();
                        }
                        ObjectType::MetaClass => {
                            let class_obj = rf.downcast::<GloomClassObj>();
                            func = class_obj
                                .class
                                .inner()
                                .funcs
                                .get(index as usize)
                                .unwrap()
                                .clone();
                        }
                        ObjectType::MetaBuiltinType => {
                            let class = rf.downcast::<BuiltinClassObj>();
                            func = class
                                .class
                                .inner()
                                .funcs
                                .get(index as usize)
                                .unwrap()
                                .clone();
                        }
                        _ => panic!(),
                    };
                    let result = self.call_fn(&*func.inner(), GloomArgs::new(args));
                    frame.push(result);
                }
                ByteCode::CallMethod { index, nargs } => {
                    let mut args = Vec::with_capacity((nargs + 1) as usize);
                    for _ in 0..nargs {
                        args.push(frame.pop());
                    }
                    let obj_val = frame.pop();
                    let func = obj_val.as_ref().method(index, &self.status);
                    args.push(obj_val);
                    args.reverse();
                    let result = self.call_fn(&*func.inner(), GloomArgs::new(args));
                    frame.push(result);
                }
                ByteCode::CallMethodDyn {
                    interface_idx, fn_idx, nargs
                } => {
                    let mut args = Vec::with_capacity((nargs + 1) as usize);
                    for _ in 0..nargs {
                        args.push(frame.pop());
                    }
                    let obj_val = frame.pop();
                    let func = {
                        let obj = obj_val.as_ref().downcast::<GloomObject>();
                        let class = obj.class.inner();
                        class.dynamic_dispatch(interface_idx, fn_idx).clone()
                    };
                    args.push(obj_val);
                    args.reverse();
                    let result = self.call_fn(func.inner().deref(),GloomArgs::new(args));
                    frame.push(result);
                }
                ByteCode::Jump(label) => {
                    pc = label as usize;
                }
                ByteCode::JumpIf(label) => {
                    if frame.pop().assert_bool() {
                        pc = label as usize;
                    }
                }
                ByteCode::JumpIfNot(label) => {
                    if !frame.pop().assert_bool() {
                        pc = label as usize;
                    }
                }
                ByteCode::JumpIfNone(label) => {
                    let is_none = frame.top().is_none();
                    if is_none {
                        pc = label as usize;
                        frame.pop();
                    }
                }
                ByteCode::Return => {
                    if frame.stack_not_empty() {
                        result = frame.pop();
                    }
                    break;
                }
                ByteCode::CollectTuple(len) => {
                    let mut tuple = Vec::with_capacity(len as usize);
                    for _ in 0..len {
                        tuple.push(frame.pop());
                    }
                    frame.push(Value::Ref(GloomTuple::new(tuple)));
                }
                ByteCode::CollectArray(basic_type, len) => match basic_type {
                    BasicType::Int => {
                        let mut array = Vec::with_capacity(len as usize);
                        for _ in 0..len {
                            array.push(frame.pop().assert_int());
                        }
                        frame.push(Value::Ref(GloomArray::new(RawArray::IntVec(array))));
                    }
                    BasicType::Num => {
                        let mut array = Vec::with_capacity(len as usize);
                        for _ in 0..len {
                            array.push(frame.pop().assert_num());
                        }
                        frame.push(Value::Ref(GloomArray::new(RawArray::NumVec(array))));
                    }
                    BasicType::Char => {
                        let mut array = Vec::with_capacity(len as usize);
                        for _ in 0..len {
                            array.push(frame.pop().assert_char());
                        }
                        frame.push(Value::Ref(GloomArray::new(RawArray::CharVec(array))));
                    }
                    BasicType::Bool => {
                        let mut array = Vec::with_capacity(len as usize);
                        for _ in 0..len {
                            array.push(frame.pop().assert_bool());
                        }
                        frame.push(Value::Ref(GloomArray::new(RawArray::BoolVec(array))));
                    }
                    BasicType::Ref => {
                        let mut array = Vec::with_capacity(len as usize);
                        for _ in 0..len {
                            array.push(frame.pop().assert_into_ref());
                        }
                        frame.push(Value::Ref(GloomArray::new(RawArray::RefVec(array))));
                    }
                },
                ByteCode::CollectQueue(basic_type, len) => match basic_type {
                    BasicType::Int => {
                        let mut queue = VecDeque::with_capacity(len as usize);
                        for _ in 0..len {
                            queue.push_back(frame.pop().assert_int());
                        }
                        frame.push(Value::Ref(GloomQueue::new(RawQueue::IntQue(queue))));
                    }
                    BasicType::Num => {
                        let mut queue = VecDeque::with_capacity(len as usize);
                        for _ in 0..len {
                            queue.push_back(frame.pop().assert_num());
                        }
                        frame.push(Value::Ref(GloomQueue::new(RawQueue::NumQue(queue))));
                    }
                    BasicType::Char => {
                        let mut queue = VecDeque::with_capacity(len as usize);
                        for _ in 0..len {
                            queue.push_back(frame.pop().assert_char());
                        }
                        frame.push(Value::Ref(GloomQueue::new(RawQueue::CharQue(queue))));
                    }
                    BasicType::Bool => {
                        let mut queue = VecDeque::with_capacity(len as usize);
                        for _ in 0..len {
                            queue.push_back(frame.pop().assert_bool());
                        }
                        frame.push(Value::Ref(GloomQueue::new(RawQueue::BoolQue(queue))));
                    }
                    BasicType::Ref => {
                        let mut queue = VecDeque::with_capacity(len as usize);
                        for _ in 0..len {
                            queue.push_back(frame.pop().assert_into_ref());
                        }
                        frame.push(Value::Ref(GloomQueue::new(RawQueue::RefQue(queue))));
                    }
                },
                ByteCode::Construct(class_idx) => {
                    frame.push(Value::Ref(GloomObject::new(
                        self.status.classes.get(class_idx as usize).unwrap().clone(),
                    )));
                }
                ByteCode::RangeIter => {
                    let start = frame.pop().assert_int();
                    let end = frame.pop().assert_int();
                    let step = frame.pop().assert_int();
                    frame.push(Value::Ref(RangeIter::new(start, end, step)));
                }
                ByteCode::InvokeIter => {
                    let iter = frame.pop().assert_into_ref().iter();
                    frame.push(Value::Ref(iter));
                }
                ByteCode::InvokeNext => {
                    let next = frame.top().as_ref().next();
                    frame.push(next);
                }
            }
            /*println!("{:?}",frame);
            println!("---");*/
        }
        result
    }

    #[inline]
    pub fn drop_object(&self, rf: &GloomObjRef) {
        if rf.count() == 1 {
            rf.drop_by_vm(self);
        }
    }
    #[inline]
    pub fn drop_object_manually(&self, mut rf: ManuallyDrop<GloomObjRef>) {
        if rf.count() == 1 {
            rf.drop_by_vm(self);
        }
        unsafe {
            ManuallyDrop::drop(&mut rf);
        }
    }
    #[inline]
    pub fn drop_option_manually(&self, option: Option<ManuallyDrop<GloomObjRef>>) {
        if let Some(mut rf) = option {
            if rf.count() == 1 {
                rf.drop_by_vm(self);
            }
            unsafe {
                ManuallyDrop::drop(&mut rf);
            }
        }
    }
}
