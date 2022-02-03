use std::collections::VecDeque;
use std::mem::ManuallyDrop;
use std::ops::{Deref};
use std::rc::Rc;
use crate::builtin::array::{GloomArray, RawArray};
use crate::builtin::boxed::{GloomChar, GloomInt, GloomNum};
use crate::builtin::classes::BuiltinClass;
use crate::builtin::obj::BuiltinClassObj;
use crate::builtin::queue::{GloomQueue, RawQueue};
use crate::builtin::string::GloomString;
use crate::exec::result::GloomResult;
use crate::exec::scope::Scope;
use crate::exec::static_table::StaticTable;
use crate::exec::value::{GloomArgs, Value};
use crate::frontend::ast::{Chain, Expression, ExprType, ForIter, FuncExpr, LeftValue, Statement, Var, VarId};
use crate::frontend::ops::{BinOp, LeftValueOp};
use crate::frontend::status::GloomStatus;
use crate::obj::func::{ GloomFunc, GloomFuncObj};
use crate::obj::gloom_class::GloomClassObj;
use crate::obj::gloom_enum::GloomEnum;
use crate::obj::gloom_object::GloomObject;
use crate::obj::gloom_tuple::GloomTuple;
use crate::obj::object::{GloomObjRef, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{BasicType, DataType, RefType};

pub struct Executor {
    status : GloomStatus,
    static_table : StaticTable
}

impl Drop for Executor {
    fn drop(&mut self) {
        // drop static table
        let table = &self.static_table.table;
        for idx in self.static_table.drop_vec.iter() {
            self.drop_object_manually(table.take_slot_ref(*idx));
        }
    }
}

#[derive(Copy,Clone,Debug)]
pub enum BlockType{
    Func,
    Loop,
    IfElse
}

impl Executor {
    pub fn new(status : GloomStatus, static_table : StaticTable) -> Executor{
        Executor{
            status,
            static_table
        }
    }
    pub fn exec(mut executor : Executor){
        let scripts = std::mem::replace(
            executor.status.script_bodies.as_mut(),
            Vec::with_capacity(0)
        );
        for script_body in scripts.iter() {
            // println!("{:?}",script_body.inner().func.body);
            script_body.inner_mut().func.call(
                &executor,
                GloomArgs::empty(),
                Vec::with_capacity(0)
            );
        }
    }
    #[inline]
    pub fn execute_statement(&self, statements : &Vec<Statement>, local : &mut Scope, block_type : BlockType ) -> GloomResult {
        for statement in statements.iter() {
            match statement {
                Statement::Let(let_tuple) => {
                    let (var,_,expr,_) = let_tuple.deref();
                    let val = self.execute_expr(expr,local).assert_into_value();
                    Executor::write_local_var(var,val,local);
                }
                Statement::Static(static_box) | Statement::PubStatic(static_box) => {
                    let (var,_,expr) = static_box.deref();
                    let value = self.execute_expr(expr,local).assert_into_value();
                    match var {
                        Var::StaticInt(slot_idx,sub_idx) => {
                            self.static_table.write_int(*slot_idx,*sub_idx,value.assert_int());
                        }
                        Var::StaticNum(slot_idx,sub_idx) => {
                            self.static_table.write_num(*slot_idx,*sub_idx,value.assert_num());
                        }
                        Var::StaticChar(slot_idx,sub_idx) => {
                            self.static_table.write_char(*slot_idx,*sub_idx,value.assert_char());
                        }
                        Var::StaticBool(slot_idx,sub_idx) => {
                            self.static_table.write_bool(*slot_idx,*sub_idx,value.assert_bool());
                        }
                        Var::StaticRef(slot_idx) => {
                            self.static_table.write_ref_firstly(*slot_idx,value.assert_into_ref());
                        }
                        _ => panic!()
                    }
                }
                Statement::LeftValueOp(left) => {
                    let (left_value,left_value_op) = left.deref();
                    match left_value {
                        LeftValue::Var(var) => {
                            match left_value_op {
                                LeftValueOp::Assign(expr) => {
                                    let value = self.execute_expr(expr,local).assert_into_value();
                                    match var {
                                        Var::LocalInt(slot_idx,sub_idx) =>
                                            local.write_int(*slot_idx, *sub_idx, value.assert_int()),
                                        Var::LocalNum(slot_idx,sub_idx) =>
                                            local.write_num(*slot_idx, *sub_idx, value.assert_num()),
                                        Var::LocalChar(slot_idx,sub_idx) =>
                                            local.write_char(*slot_idx, *sub_idx, value.assert_char()),
                                        Var::LocalBool(slot_idx,sub_idx) =>
                                            local.write_bool(*slot_idx, *sub_idx, value.assert_bool()),
                                        Var::LocalRef(slot_idx) => {
                                            let manually_drop_ref
                                                = local.replace_ref(*slot_idx,value.assert_into_ref());
                                            self.drop_object_manually(manually_drop_ref);
                                        }
                                        Var::StaticInt(slot_idx,sub_idx) =>
                                            self.static_table.write_int(*slot_idx, *sub_idx, value.assert_int()),
                                        Var::StaticNum(slot_idx,sub_idx) =>
                                            self.static_table.write_num(*slot_idx, *sub_idx, value.assert_num()),
                                        Var::StaticChar(slot_idx,sub_idx) =>
                                            self.static_table.write_char(*slot_idx, *sub_idx, value.assert_char()),
                                        Var::StaticBool(slot_idx,sub_idx) =>
                                            self.static_table.write_bool(*slot_idx, *sub_idx, value.assert_bool()),
                                        Var::StaticRef(slot_idx) => {
                                            let manually_drop_ref
                                                = self.static_table.replace_ref(*slot_idx, value.assert_into_ref());
                                            self.drop_object_manually(manually_drop_ref);
                                        }
                                        _ => panic!()
                                    }
                                }
                                LeftValueOp::PlusEq(expr) => {
                                    let value = self.execute_expr(expr,local).assert_into_value();
                                    match var {
                                        Var::LocalInt(slot_idx,sub_idx) =>{
                                            let new_val = local.read_int(*slot_idx,*sub_idx) + value.assert_int_include_num();
                                            local.write_int(*slot_idx, *sub_idx,new_val);
                                        }
                                        Var::LocalNum(slot_idx,sub_idx) =>{
                                            let new_val = local.read_num(*slot_idx,*sub_idx) + value.assert_num_include_int();
                                            local.write_num(*slot_idx, *sub_idx, new_val);
                                        }
                                        Var::StaticInt(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_int(*slot_idx,*sub_idx) + value.assert_int_include_num();
                                            self.static_table.write_int(*slot_idx,*sub_idx,new_val);
                                        }
                                        Var::StaticNum(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_num(*slot_idx,*sub_idx) + value.assert_num_include_int();
                                            self.static_table.write_num(*slot_idx,*sub_idx,new_val);
                                        }
                                        _ => panic!()
                                    }
                                }
                                LeftValueOp::SubEq(expr) => {
                                    let value = self.execute_expr(expr,local).assert_into_value();
                                    match var {
                                        Var::LocalInt(slot_idx,sub_idx) =>{
                                            let new_val = local.read_int(*slot_idx,*sub_idx) - value.assert_int_include_num();
                                            local.write_int(*slot_idx, *sub_idx,new_val);
                                        }
                                        Var::LocalNum(slot_idx,sub_idx) =>{
                                            let new_val = local.read_num(*slot_idx,*sub_idx) - value.assert_num_include_int();
                                            local.write_num(*slot_idx, *sub_idx, new_val);
                                        }
                                        Var::StaticInt(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_int(*slot_idx,*sub_idx) - value.assert_int_include_num();
                                            self.static_table.write_int(*slot_idx,*sub_idx,new_val);
                                        }
                                        Var::StaticNum(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_num(*slot_idx,*sub_idx) - value.assert_num_include_int();
                                            self.static_table.write_num(*slot_idx,*sub_idx,new_val);
                                        }
                                        _ => panic!()
                                    }
                                }
                                LeftValueOp::PlusOne => {
                                    match var {
                                        Var::LocalInt(slot_idx,sub_idx) =>{
                                            let new_val = local.read_int(*slot_idx,*sub_idx) + 1;
                                            local.write_int(*slot_idx, *sub_idx,new_val);
                                        }
                                        Var::LocalNum(slot_idx,sub_idx) =>{
                                            let new_val = local.read_num(*slot_idx,*sub_idx) + 1.0;
                                            local.write_num(*slot_idx, *sub_idx, new_val);
                                        }
                                        Var::StaticInt(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_int(*slot_idx,*sub_idx) + 1;
                                            self.static_table.write_int(*slot_idx,*sub_idx,new_val);
                                        }
                                        Var::StaticNum(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_num(*slot_idx,*sub_idx) + 1.0;
                                            self.static_table.write_num(*slot_idx,*sub_idx,new_val);
                                        }
                                        _ => panic!()
                                    }
                                }
                                LeftValueOp::SubOne => {
                                    match var {
                                        Var::LocalInt(slot_idx,sub_idx) =>{
                                            let new_val = local.read_int(*slot_idx,*sub_idx) - 1;
                                            local.write_int(*slot_idx, *sub_idx,new_val);
                                        }
                                        Var::LocalNum(slot_idx,sub_idx) =>{
                                            let new_val = local.read_num(*slot_idx,*sub_idx) - 1.0;
                                            local.write_num(*slot_idx, *sub_idx, new_val);
                                        }
                                        Var::StaticInt(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_int(*slot_idx,*sub_idx) - 1;
                                            self.static_table.write_int(*slot_idx,*sub_idx,new_val);
                                        }
                                        Var::StaticNum(slot_idx,sub_idx) => {
                                            let new_val = self.static_table.read_num(*slot_idx,*sub_idx) - 1.0;
                                            self.static_table.write_num(*slot_idx,*sub_idx,new_val);
                                        }
                                        _ => panic!()
                                    }
                                }
                            }
                        }
                        LeftValue::Chain(first_expr,chains) => {
                            let mut value = self.execute_expr(first_expr,local).assert_into_value();
                            let max_chain_idx = chains.len() - 1;
                            for (chain_idx,chain) in chains[..max_chain_idx].iter().enumerate() {
                                match chain {
                                    Chain::Access(var_id,field_type) => {
                                        let obj = std::mem::replace(&mut value,Value::Int(0));
                                        let obj_ref = obj.assert_into_ref();
                                        let (slot_idx,sub_idx) = var_id.index();
                                        if chain_idx == max_chain_idx {
                                            match obj_ref.obj_type() {
                                                ObjectType::Class => {
                                                    let obj = obj_ref.downcast::<GloomObject>();
                                                    match left_value_op {
                                                        LeftValueOp::Assign(expr) => {
                                                            let assign_value = self.execute_expr(expr, local).assert_into_value();
                                                            let option =
                                                                obj.write_field(slot_idx, sub_idx, assign_value, *field_type);
                                                            if let Some(rf) = option {
                                                                self.drop_object(rf.deref());
                                                            }
                                                        }
                                                        LeftValueOp::PlusEq(expr) => {
                                                            let plus_value = self.execute_expr(expr, local).assert_into_value();
                                                            let mut new_value = obj.read_field(slot_idx, sub_idx, *field_type);
                                                            new_value.plus(plus_value);
                                                            let option
                                                                = obj.write_field(slot_idx, sub_idx,new_value, *field_type);
                                                            if let Some(rf) = option {
                                                                self.drop_object(rf.deref());
                                                            }
                                                        }
                                                        LeftValueOp::SubEq(expr) => {
                                                            let sub_val = self.execute_expr(expr, local).assert_into_value();
                                                            let mut new_value = obj.read_field(slot_idx, sub_idx, *field_type);
                                                            new_value.sub(sub_val);
                                                            let option
                                                                = obj.write_field(slot_idx, sub_idx,new_value, *field_type);
                                                            if let Some(rf) = option {
                                                                self.drop_object(rf.deref());
                                                            }
                                                        }
                                                        LeftValueOp::PlusOne => {
                                                            let mut new_value = obj.read_field(slot_idx, sub_idx, *field_type);
                                                            new_value.plus_one();
                                                            let option
                                                                = obj.write_field(slot_idx, sub_idx,new_value, *field_type);
                                                            if let Some(rf) = option {
                                                                self.drop_object(rf.deref());
                                                            }
                                                        }
                                                        LeftValueOp::SubOne => {
                                                            let mut new_value = obj.read_field(slot_idx, sub_idx, *field_type);
                                                            new_value.sub_one();
                                                            let option
                                                                = obj.write_field(slot_idx, sub_idx,new_value, *field_type);
                                                            if let Some(rf) = option {
                                                                self.drop_object(rf.deref());
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => panic!()
                                            }
                                        }else{
                                            match obj_ref.obj_type() {
                                                ObjectType::Class => {
                                                    let obj = obj_ref.downcast::<GloomObject>();
                                                    value = obj.read_field(slot_idx,sub_idx,*field_type);
                                                }
                                                ObjectType::MetaClass => {
                                                    let obj = obj_ref.downcast::<GloomClassObj>();
                                                    let func = obj.class.inner().funcs.get(slot_idx as usize).unwrap().clone();
                                                    value = Value::Ref(GloomFuncObj::new_func(func));
                                                }
                                                _ => panic!()
                                            }
                                        }
                                    }
                                    Chain::FnCall {
                                        func,
                                        need_self,
                                        args,
                                    } => {
                                        let new_value =
                                            self.handle_fn_call(func, *need_self, args, &mut value,local)
                                            .assert_into_value();
                                        value = new_value;
                                    }
                                    Chain::Call(arg_exprs) => {
                                        let mut args = Vec::with_capacity(arg_exprs.len());
                                        for arg_expr in arg_exprs.iter() {
                                            args.push(self.execute_expr(arg_expr, local).assert_into_value());
                                        }
                                        let obj = value.assert_into_ref();
                                        if let ObjectType::Func = obj.obj_type(){
                                            let func_obj = obj.downcast::<GloomFuncObj>();
                                            let func = func_obj.func.inner();
                                            let captured_values = func_obj.captures.borrow().clone();
                                            value = func.call(
                                                self,
                                                GloomArgs::new(args),
                                                captured_values
                                            ).assert_into_value();
                                        }else{
                                            panic!()
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Statement::Expr(expr,_) | Statement::Discard(expr,_) => {
                    match self.execute_expr(expr, local) {
                        GloomResult::ValueVoid | GloomResult::IfElseVoid => {}
                        GloomResult::Value(val) | GloomResult::IfElseResult(val)  => {
                            if let Value::Ref(rf) = val {
                                self.drop_object(&rf);
                            }
                        }
                        result => return result
                    }
                }
                Statement::Return(expr,_) => {
                    return match self.execute_expr(expr, local) {
                        GloomResult::Value(value) => GloomResult::Return(value),
                        GloomResult::ValueVoid => GloomResult::ReturnVoid,
                        _ => panic!()
                    }
                }
                // 在某些情况下，不应当执行 Statement::Continue 和 Statement::Break，但他们应当在语义分析时被过滤了
                // In some situation, executor should not execute Statement::Continue and Statement::Break, but them should be filter during analysis
                Statement::Continue(_) => {
                    return GloomResult::Continue;
                }
                Statement::Break(expr,_) => {
                    return if expr.is_none() {
                        GloomResult::BreakVoid
                    }else {
                        match self.execute_expr(expr, local) {
                            GloomResult::Value(value) => GloomResult::Break(value),
                            GloomResult::ValueVoid => GloomResult::BreakVoid,
                            _ => panic!()
                        }
                    };
                }
                Statement::IfResult(expr,_) => {
                    return match self.execute_expr(expr, local) {
                        GloomResult::Value(value) => GloomResult::IfElseResult(value),
                        GloomResult::ValueVoid => GloomResult::IfElseVoid,
                        _ => panic!()
                    }
                }
            }
        };
        match block_type {
            BlockType::Func => GloomResult::ReturnVoid,
            BlockType::Loop => GloomResult::Continue,
            BlockType::IfElse => GloomResult::IfElseVoid
        }
    }

    #[inline]
    pub fn execute_expr(&self, expr : &Expression, local : &mut Scope ) -> GloomResult {
        match expr {
            Expression::None => GloomResult::ValueVoid,
            Expression::Int(int) => GloomResult::Value(Value::Int(*int)),
            Expression::Num(num) => GloomResult::Value(Value::Num(*num)),
            Expression::Char(ch) => GloomResult::Value(Value::Char(*ch)),
            Expression::Bool(bl) => GloomResult::Value(Value::Bool(*bl)),
            Expression::Str(str) => {
                GloomResult::Value(Value::Ref(
                    GloomString::new(str.deref().clone())
                ))
            }
            Expression::Var(var) => {
                match var.deref() {
                    Var::LocalInt(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Int(local.read_int(*slot_idx, *sub_idx)))
                    }
                    Var::LocalNum(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Num(local.read_num(*slot_idx, *sub_idx)))
                    }
                    Var::LocalChar(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Char(local.read_char(*slot_idx, *sub_idx)))
                    }
                    Var::LocalBool(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Bool(local.read_bool(*slot_idx, *sub_idx)))
                    }
                    Var::LocalRef(slot_idx) => {
                        GloomResult::Value(Value::Ref(local.read_ref(*slot_idx).clone()))
                    }
                    Var::StaticInt(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Int(self.static_table.read_int(*slot_idx, *sub_idx)))
                    }
                    Var::StaticNum(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Num(self.static_table.read_num(*slot_idx, *sub_idx)))
                    }
                    Var::StaticChar(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Char(self.static_table.read_char(*slot_idx, *sub_idx)))
                    }
                    Var::StaticBool(slot_idx,sub_idx) => {
                        GloomResult::Value(Value::Bool(self.static_table.read_bool(*slot_idx, *sub_idx)))
                    }
                    Var::StaticRef(slot_idx) => {
                        GloomResult::Value(Value::Ref(self.static_table.read_ref(*slot_idx).clone()))
                    }
                    Var::Class(idx) => {
                        GloomResult::Value(Value::Ref(
                            GloomClassObj::new(
                                self.status.classes.get(*idx as usize).unwrap().clone()
                            )
                        ))
                    }
                    /*Var::Enum(idx) => {}
                    Var::Interface(idx) => {}*/
                    Var::DirectFn(idx) => {
                        GloomResult::Value(Value::Ref(
                            GloomFuncObj::new_func(
                                self.status.funcs.get(*idx as usize).unwrap().clone()
                            )
                        ))
                    }
                    Var::BuiltinType(idx) => {
                        GloomResult::Value(Value::Ref(
                            BuiltinClassObj::new(self.status.builtin_classes.get(*idx as usize).unwrap().clone())
                        ))
                    }
                    var => panic!("{:?}",var),
                }
            }
            Expression::Tuple(tuple) => {
                let mut values = Vec::with_capacity(tuple.deref().len());
                for expr in tuple.iter() {
                    values.push(self.execute_expr(expr,local).assert_into_value());
                }
                GloomResult::Value(Value::Ref(
                    GloomTuple::new(values)
                ))
            }
            Expression::Array(array) => {
                let (exprs,basic_type,is_queue) = array.deref();
                let len = exprs.len();
                if *is_queue {
                    let queue = match basic_type {
                        BasicType::Int => {
                            let mut vec: VecDeque<i64> = VecDeque::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push_back(self.execute_expr(expr, local).assert_into_value().assert_int())
                            }
                            RawQueue::IntQue(vec)
                        }
                        BasicType::Num => {
                            let mut vec: VecDeque<f64> = VecDeque::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push_back(self.execute_expr(expr, local).assert_into_value().assert_num())
                            }
                            RawQueue::NumQue(vec)
                        }
                        BasicType::Char => {
                            let mut vec: VecDeque<char> = VecDeque::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push_back(self.execute_expr(expr, local).assert_into_value().assert_char())
                            }
                            RawQueue::CharQue(vec)
                        }
                        BasicType::Bool => {
                            let mut vec: VecDeque<bool> = VecDeque::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push_back(self.execute_expr(expr, local).assert_into_value().assert_bool())
                            }
                            RawQueue::BoolQue(vec)
                        }
                        BasicType::Ref => {
                            let mut vec: VecDeque<GloomObjRef> = VecDeque::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push_back(self.execute_expr(expr, local).assert_into_value().assert_into_ref())
                            }
                            RawQueue::RefQue(vec)
                        }
                    };
                    GloomResult::Value(Value::Ref(
                        GloomQueue::new(queue)
                    ))
                }else{
                    let array = match basic_type {
                        BasicType::Int => {
                            let mut vec: Vec<i64> = Vec::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push(self.execute_expr(expr, local).assert_into_value().assert_int());
                            }
                            RawArray::IntVec(vec)
                        }
                        BasicType::Num => {
                            let mut vec: Vec<f64> = Vec::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push(self.execute_expr(expr, local).assert_into_value().assert_num());
                            }
                            RawArray::NumVec(vec)
                        }
                        BasicType::Char => {
                            let mut vec: Vec<char> = Vec::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push(self.execute_expr(expr, local).assert_into_value().assert_char());
                            }
                            RawArray::CharVec(vec)
                        }
                        BasicType::Bool => {
                            let mut vec: Vec<bool> = Vec::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push(self.execute_expr(expr, local).assert_into_value().assert_bool());
                            }
                            RawArray::BoolVec(vec)
                        }
                        BasicType::Ref => {
                            let mut vec: Vec<GloomObjRef> = Vec::with_capacity(len);
                            for expr in exprs.iter() {
                                vec.push(self.execute_expr(expr, local).assert_into_value().assert_into_ref());
                            }
                            RawArray::RefVec(vec)
                        }
                    };
                    GloomResult::Value(Value::Ref(
                        GloomArray::new(array)
                    ))
                }
            }
            Expression::NegOp(expr) => {
                let value = self.execute_expr(expr, local).assert_into_value();
                let new_value = match value {
                    Value::Int(n) => Value::Int(-n),
                    Value::Num(n) => Value::Num(-n),
                    _ => panic!()
                };
                GloomResult::Value(new_value)
            }
            Expression::NotOp(expr) => {
                let value = self.execute_expr(expr, local).assert_into_value();
                let new_value = match value {
                    Value::Bool(b) => Value::Bool(!b),
                    _ => panic!()
                };
                GloomResult::Value(new_value)
            }
            Expression::Chain(chains) => {
                let (expr,chains) = chains.deref();
                let mut value = self.execute_expr(expr,local).assert_into_value();
                for chain in chains.iter() {
                    match chain {
                        Chain::Access(var_id,field_type) => {
                            let obj = std::mem::replace(&mut value,Value::Int(0));
                            let obj_ref = obj.assert_into_ref();
                            let (slot_idx,sub_idx) = var_id.index();
                            match obj_ref.obj_type() {
                                ObjectType::Class => {
                                    let obj = obj_ref.downcast::<GloomObject>();
                                    value = obj.read_field(slot_idx,sub_idx,*field_type);
                                }
                                ObjectType::MetaClass => {
                                    let obj = obj_ref.downcast::<GloomClassObj>();
                                    let func = obj.class.inner().funcs.get(slot_idx as usize).unwrap().clone();
                                    value = Value::Ref(GloomFuncObj::new_func(func));
                                }
                                _ => panic!()
                            }
                        }
                        Chain::FnCall {
                            func,
                            need_self,
                            args
                        } => {
                            let return_value = self.handle_fn_call(func,*need_self,args,&mut value,local);
                            match return_value {
                                GloomResult::Return(return_val) => {
                                    value = return_val;
                                }
                                GloomResult::ReturnVoid => {
                                    return GloomResult::ValueVoid
                                }
                                val => panic!("{:?}",val)
                            }
                        }
                        Chain::Call(arg_exprs) => {
                            let mut args = Vec::with_capacity(arg_exprs.len());
                            for arg_expr in arg_exprs.iter() {
                                args.push(self.execute_expr(arg_expr,local).assert_into_value());
                            }
                            let obj = value.assert_into_ref();
                            if let ObjectType::Func = obj.obj_type(){
                                let func_obj = obj.downcast::<GloomFuncObj>();
                                let func = func_obj.func.inner();
                                let captured_values = func_obj.captures.borrow().clone();
                                let return_value = func.call(
                                    self,
                                    GloomArgs::new(args),
                                    captured_values
                                );
                                match return_value {
                                    GloomResult::Return(return_val) => {
                                        value = return_val;
                                    }
                                    GloomResult::ReturnVoid => {
                                        return GloomResult::ValueVoid
                                    }
                                    val => panic!("{:?}",val)
                                }
                            }else{
                                panic!("{:?}",obj)
                            }
                        }
                    }
                }
                GloomResult::Value(value)
            }
            Expression::BinaryOp(bin_op_vec) => {
                let bin_op_vec = bin_op_vec.deref();
                let mut value = self.execute_expr(&bin_op_vec.left,local).assert_into_value();
                for (bin_op, expr) in bin_op_vec.vec.iter() {
                    let right_value = self.execute_expr(expr, local).assert_into_value();
                    match bin_op {
                        BinOp::Plus => value.plus(right_value),
                        BinOp::Sub => value.sub(right_value),
                        BinOp::Mul => value.multiply(right_value),
                        BinOp::Div => value.divide(right_value),

                        BinOp::Gt => value = Value::Bool(value.greater_than(right_value)),
                        BinOp::Lt => value = Value::Bool(value.less_than(right_value)),
                        BinOp::GtEq => value = Value::Bool(value.greater_equal(right_value)),
                        BinOp::LtEq => value = Value::Bool(value.less_equal(right_value)),

                        BinOp::Eqs => value = Value::Bool(value.equals(right_value)),
                        BinOp::NotEq => value = Value::Bool(!value.equals(right_value)),

                        BinOp::And => value = Value::Bool(value.assert_bool() && right_value.assert_bool()),
                        BinOp::Or => value = Value::Bool(value.assert_bool() || right_value.assert_bool()),
                    }
                }
                GloomResult::Value(value)
            }
            Expression::Cast(cast) => {
                let (expr,_,cast_type) = cast.deref();
                let value = self.execute_expr(expr, local).assert_into_value();
                GloomResult::Value(match cast_type {
                    DataType::Int => Value::Int(value.assert_int_form_num_liked()),
                    DataType::Num => Value::Num(value.assert_num_include_int()),
                    DataType::Char => Value::Char(value.assert_char_include_int()),
                    DataType::Bool => value,
                    DataType::Ref(ref_type) => {
                        Value::Ref(match ref_type {
                            RefType::Int => GloomInt::new(value.assert_int_form_num_liked()),
                            RefType::Num => GloomNum::new(value.assert_num_include_int()),
                            RefType::Char => GloomChar::new(value.assert_char_include_int()),
                            _ => panic!()
                        })
                    }
                })
            }
            Expression::Func(func) => {
                if let FuncExpr::Analysed(func_ref) = func.deref() {
                    let func = func_ref.inner();
                    let captures = &func.info.captures;
                    let mut captured_values = Vec::with_capacity(captures.len());
                    for capture in captures.iter() {
                        captured_values.push(match capture.basic_type {
                            BasicType::Int => Value::Int(local.read_int(capture.from_slot_idx,capture.from_sub_idx)),
                            BasicType::Num => Value::Num(local.read_num(capture.from_slot_idx,capture.from_sub_idx)),
                            BasicType::Char => Value::Char(local.read_char(capture.from_slot_idx,capture.from_sub_idx)),
                            BasicType::Bool => Value::Bool(local.read_bool(capture.from_slot_idx,capture.from_sub_idx)),
                            BasicType::Ref => Value::Ref(local.read_ref(capture.from_slot_idx).clone())
                        })
                    }
                    GloomResult::Value(Value::Ref(
                        GloomFuncObj::new_closure(func_ref.clone(),captured_values)
                    ))
                }else{
                    panic!("{:?}",func)
                }
            }
            Expression::IfElse(if_else) => {
                let mut result = GloomResult::ValueVoid;
                for if_branch in if_else.deref().branches.iter() {
                    let condition = self.execute_expr(&if_branch.condition, local)
                        .assert_into_value()
                        .assert_bool();
                    if condition {
                        let value = self.execute_statement(&if_branch.statements, local,BlockType::IfElse);
                        result = match value {
                            GloomResult::Return(value) => GloomResult::Return(value),
                            GloomResult::ReturnVoid => GloomResult::ReturnVoid,
                            GloomResult::Continue => GloomResult::Continue,
                            GloomResult::Break(value) => GloomResult::Break(value),
                            GloomResult::BreakVoid => GloomResult::BreakVoid,
                            GloomResult::IfElseResult(value) => GloomResult::Value(value),
                            GloomResult::IfElseVoid => GloomResult::ValueVoid,
                            _ => panic!()
                        };
                        for idx in if_branch.drop_vec.iter() {
                            self.drop_object_manually(local.take_ref(*idx));
                        }
                        break
                    }
                }
                result
            }
            Expression::While(while_loop) => {
                let while_loop = while_loop.deref();
                let result = loop {
                    let condition = self.execute_expr(&while_loop.condition, local).assert_into_value().assert_bool();
                    if condition {
                        let result = self.execute_statement(&while_loop.statements, local, BlockType::Loop);
                        match result {
                            GloomResult::Return(value) => break GloomResult::Return(value),
                            GloomResult::ReturnVoid => break GloomResult::ReturnVoid,
                            GloomResult::Break(value) => break GloomResult::Value(value),
                            GloomResult::BreakVoid => break GloomResult::ValueVoid,
                            GloomResult::Continue => {}
                            _ => panic!()
                        }
                    } else {
                        break GloomResult::ValueVoid;
                    }
                    for idx in while_loop.drop_vec.iter() {
                        self.drop_object_manually(local.take_ref(*idx));
                    }
                };
                result
            }
            Expression::For(for_loop) => {
                match &for_loop.for_iter {
                    ForIter::Range(start_expr, end_expr, step_expr) => {
                        let mut index = self.execute_expr(start_expr, local).assert_into_value().assert_int();
                        let end = self.execute_expr(end_expr,local).assert_into_value().assert_int();
                        let step = self.execute_expr(step_expr,local).assert_into_value().assert_int();
                        let result = loop {
                            if index < end{
                                Executor::write_local_var(&for_loop.var,Value::Int(index),local);
                                let result = self.execute_statement(&for_loop.statements, local, BlockType::Loop);
                                match result {
                                    GloomResult::Return(value) => break GloomResult::Return(value),
                                    GloomResult::ReturnVoid => break GloomResult::ReturnVoid,
                                    GloomResult::Break(value) => break GloomResult::Value(value),
                                    GloomResult::BreakVoid => break GloomResult::ValueVoid,
                                    GloomResult::Continue => {}
                                    _ => panic!()
                                }
                                index += step;
                            }else {
                                break GloomResult::ValueVoid
                            }
                        };
                        result
                    }
                    ForIter::Iter(iter_expr) => {
                        let obj = self.execute_expr(iter_expr, local).assert_into_value().assert_into_ref();
                        let mut result = GloomResult::ValueVoid;
                        for val in obj.iterator() {
                            Executor::write_local_var(&for_loop.var,val,local);
                            let temp_result = self.execute_statement(&for_loop.statements, local, BlockType::Loop);
                            let should_break = match temp_result {
                                GloomResult::Return(value) => {
                                    result = GloomResult::Return(value);
                                    true
                                }
                                GloomResult::ReturnVoid => {
                                    result = GloomResult::ReturnVoid;
                                    true
                                }
                                GloomResult::Break(value) => {
                                    result = GloomResult::Value(value);
                                    true
                                }
                                GloomResult::BreakVoid => {
                                    result = GloomResult::ValueVoid;
                                    true
                                }
                                GloomResult::Continue => false,
                                _ => panic!()
                            };
                            if let Var::LocalRef(idx) = &for_loop.var {
                                self.drop_object_manually(local.take_ref(*idx));
                            }
                            if should_break {
                                break
                            }
                        }
                        result
                    }
                }
            }
            Expression::Construct(construct) => {
                if let ExprType::Analyzed(data_type) = &construct.deref().class_type{
                    if let DataType::Ref(RefType::Class(class)) = data_type {
                        let object = GloomObject::new(class.clone());
                        let class = class.inner();
                        for (field, expr) in construct.fields.iter() {
                            let (slot_idx,sub_idx) = field.index();
                            let value = self.execute_expr(expr,local).assert_into_value();
                            let basic_type = class.field_indexer.get_type(slot_idx).as_basic();
                            object.write_field(slot_idx, sub_idx, value, basic_type);
                        }
                        GloomResult::Value(Value::Ref(
                            GloomObjRef::new(Rc::new(
                                object
                            ))
                        ))
                    }else{
                        panic!()
                    }
                }else{
                    panic!()
                }
            }
            expr => panic!("unsupported expression {:?}",expr)
        }
    }

    #[inline(always)]
    fn write_local_var(var : &Var, value : Value, local : &mut Scope){
        match var {
            Var::LocalInt(slot_idx,sub_idx) => {
                local.write_int(*slot_idx, *sub_idx, value.assert_int());
            }
            Var::LocalNum(slot_idx,sub_idx) => {
                local.write_num(*slot_idx, *sub_idx, value.assert_num());
            }
            Var::LocalChar(slot_idx,sub_idx) => {
                local.write_char(*slot_idx, *sub_idx, value.assert_char());
            }
            Var::LocalBool(slot_idx,sub_idx) => {
                local.write_bool(*slot_idx, *sub_idx, value.assert_bool());
            }
            Var::LocalRef(slot_idx) => {
                let rf = value.assert_into_ref();
                local.write_ref_firstly(*slot_idx, rf);
            }
            var => {
                panic!("unexpected Var {:?} when ",var)
            }
        }
    }

    #[inline(always)]
    fn handle_fn_call(&self, func : &VarId, need_self : bool, args : &Vec<Expression>, value : &mut Value, local : &mut Scope) -> GloomResult {
        let (index,_) = func.index();
        let obj = std::mem::replace(value,Value::Int(0));
        let func : RefCount<GloomFunc>;
        match &obj {
            Value::Ref(ref_obj) => {
                match ref_obj.obj_type() {
                    ObjectType::Class => {
                        let gloom_obj = ref_obj.downcast::<GloomObject>();
                        func = gloom_obj.class.inner().funcs.get(index as usize).unwrap().clone();
                    }
                    ObjectType::Enum => {
                        let enum_obj = ref_obj.downcast::<GloomEnum>();
                        func = enum_obj.class.inner().funcs.get(index as usize).unwrap().clone();
                    }
                    ObjectType::MetaClass => {
                        let class_obj = ref_obj.downcast::<GloomClassObj>();
                        func = class_obj.class.inner().funcs.get(index as usize).unwrap().clone();
                    }
                    ObjectType::String => {
                        let class = self.status.builtin_classes.get(BuiltinClass::STRING_INDEX).unwrap();
                        func = class.inner().funcs.get(index as usize).unwrap().clone();
                    }
                    ObjectType::Func => {
                        let class = self.status.builtin_classes.get(BuiltinClass::FUNC_INDEX).unwrap();
                        func = class.inner().funcs.get(index as usize).unwrap().clone();
                    }
                    ObjectType::MetaBuiltinType => {
                        let class = ref_obj.downcast::<BuiltinClassObj>();
                        func = class.class.inner().funcs.get(index as usize).unwrap().clone();
                    }
                    /*
                     ObjectType::MetaEnum => {}
                     ObjectType::Type => {}
                     ObjectType::Array => {}
                     ObjectType::Queue => {}
                     */
                    _ => panic!()
                }
            }
            _ => panic!()
        }
        let mut arg_values = Vec::with_capacity(args.len()+1);
        if need_self {
            arg_values.push(obj);
        }
        for arg_expr in args.iter() {
            arg_values.push(self.execute_expr(arg_expr,local).assert_into_value());
        }
        let func = func.inner();
        func.call(
            self,
            GloomArgs::new(arg_values),
            Vec::with_capacity(0)
        )
    }

    pub fn drop_object_manually(&self, mut rf : ManuallyDrop<GloomObjRef>){
        if rf.deref().count() == 1 {
            rf.drop_by_exec(self);
        }
        unsafe { ManuallyDrop::drop(&mut rf); }
    }
    #[inline]
    pub fn drop_object(&self, rf : &GloomObjRef){
        if rf.deref().count() == 1 {
            rf.drop_by_exec(self);
        }
    }
}