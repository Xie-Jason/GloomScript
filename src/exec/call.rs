use std::mem::MaybeUninit;

use crate::exec::executor::{BlockType, Executor};
use crate::exec::result::GloomResult;
use crate::exec::scope::Scope;
use crate::exec::value::{GloomArgs, Value};
use crate::frontend::ast::Statement;
use crate::obj::func::{Capture, FuncBody, GloomFunc, Param};
use crate::obj::slot::Slot;
use crate::obj::table::Table;
use crate::obj::types::DataType;

impl GloomFunc {

    pub fn call(&self, exec : &Executor, args : GloomArgs, captured_values : Vec<Value>) -> GloomResult {
        match &self.body {
            FuncBody::Builtin(func) => {
                func(exec,args)
            }
            FuncBody::Gloom(body) => {
                let local_size = self.info.local_size;
                if local_size <= 16 {
                    if local_size <= 8 {
                        if local_size <= 4 {
                            // 3*if
                            self.execute_stack_alloc::<4>(exec,body, args, captured_values)
                        }else{
                            self.execute_stack_alloc::<8>(exec,body, args, captured_values)
                        }
                    }else{
                        if local_size <= 12 {
                            self.execute_stack_alloc::<12>(exec,body, args, captured_values)
                        }else{
                            self.execute_stack_alloc::<16>(exec,body, args, captured_values)
                        }
                    }
                }else{
                    if local_size <= 32{
                        if local_size <= 24 {
                            // 3*if
                            self.execute_stack_alloc::<24>(exec,body, args, captured_values)
                        }else{
                            self.execute_stack_alloc::<32>(exec,body, args, captured_values)
                        }
                    }else if local_size <= 64{
                        if local_size <= 48 {
                            // 4*if
                            self.execute_stack_alloc::<48>(exec,body, args, captured_values)
                        }else{
                            self.execute_stack_alloc::<64>(exec,body, args, captured_values)
                        }
                    }else if local_size < 128{
                        if local_size <= 96 {
                            // 5*if
                            self.execute_stack_alloc::<96>(exec,body, args, captured_values)
                        }else{
                            self.execute_stack_alloc::<128>(exec,body, args, captured_values)
                        }
                    }else{
                        self.execute_heap_alloc(exec,body, args, captured_values)
                    }
                }
            }
            FuncBody::None => panic!()
        }
    }

    pub fn execute_stack_alloc<const SIZE : usize>(&self,
                                                   exec : &Executor,
                                                   statements : &Vec<Statement>,
                                                   args : GloomArgs,
                                                   captured_values : Vec<Value>) -> GloomResult {
        // 栈分配内存空间 stack alloc memory space
        let mut slots : [MaybeUninit<Slot>;SIZE] = unsafe {
            MaybeUninit::uninit().assume_init()
        };
        let mut local = Scope::from_slice(&mut slots);
        GloomFunc::fill_args(
            &mut local,
            args,
            &self.info.params,
            &self.info.captures,
            captured_values
        );
        let return_value = exec.execute_statement(statements, &mut local,BlockType::Func);
        self.drop_local(exec,local);
        return_value
    }
    pub fn execute_heap_alloc(&self,
                              exec : &Executor,
                              statements : &Vec<Statement>,
                              args : GloomArgs,
                              captured_values : Vec<Value> ) -> GloomResult {
        let table = Table::new(self.info.local_size);
        let slice = table.as_slice(self.info.local_size);
        let mut local = Scope::from_slice(slice);
        GloomFunc::fill_args(
            &mut local,
            args,
            &self.info.params,
            &self.info.captures,
            captured_values
        );
        let return_value = exec.execute_statement(statements, &mut local,BlockType::Func);
        self.drop_local(exec,local);
        return_value
    }
    #[inline]
    fn drop_local(&self,exec : &Executor,mut local : Scope){
        for idx in self.info.drop_slots.iter() {
            exec.drop_object(local.take_ref(*idx));
        }
    }

    #[inline(always)]
    pub fn fill_args(local : &mut Scope,
                     args : GloomArgs,
                     params : &Vec<Param>,
                     captures: &Vec<Capture>,
                     captured_values : Vec<Value>) {
        let mut param_iter = params.iter();
        let mut arg_iter = args.vec.into_iter();
        loop {
            if let Some(param) = param_iter.next() {
                let arg = arg_iter.next().unwrap();
                let (slot_idx,sub_idx) = param.index;
                match param.data_type{
                    DataType::Int => local.write_int(slot_idx,sub_idx,arg.as_int().unwrap()),
                    DataType::Num => local.write_num(slot_idx,sub_idx,arg.as_num().unwrap()),
                    DataType::Char => local.write_char(slot_idx,sub_idx,arg.as_char().unwrap()),
                    DataType::Bool => local.write_bool(slot_idx,sub_idx,arg.as_bool().unwrap()),
                    DataType::Ref(_) => local.write_ref_firstly(slot_idx, arg.into_ref().unwrap()),
                }
            }else{
                break
            }
        }
        let mut captured_value_iter = captured_values.into_iter();
        for capture in captures.iter() {
            match captured_value_iter.next().unwrap() {
                Value::Int(i) => local.write_int(capture.to_slot_idx,capture.to_sub_idx,i),
                Value::Num(n) => local.write_num(capture.to_slot_idx,capture.to_sub_idx,n),
                Value::Char(c) => local.write_char(capture.to_slot_idx,capture.to_sub_idx,c),
                Value::Bool(b) => local.write_bool(capture.to_slot_idx, capture.to_sub_idx, b),
                Value::Ref(r) => local.write_ref_firstly(capture.to_slot_idx,r),
            }
        }
    }
}