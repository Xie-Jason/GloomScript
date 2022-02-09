use crate::bytecode::code::ByteCode;
use crate::frontend::ast::{Chain, Expression, LeftValue, Statement, Var};
use crate::frontend::ops::LeftValueOp;
use crate::frontend::status::GloomStatus;
use crate::obj::func::{FuncBody, GloomFunc};
use crate::vm::constant::ConstantPool;
use std::ops::Deref;
use crate::obj::types::BasicType;

pub struct CodeGenerator {
    constant_pool: ConstantPool,
}

impl CodeGenerator {
    // 在generate整个block之前，我们不知道一个循环体或多个 if-else分支的结束位置的索引。
    // 所以用 INVALID_LABEL 作为label的临时值。
    // 整个block生成结束后，Generator 将遍历字节码并重新给标签赋值。
    // We don't know the end index of a loop block or a multiple if-else block before the whole block are generated,
    // So we will use INVALID_LABEL as temp value of label.
    // After the generation of whole block, Generator will travel the bytecode and re-assign the label.
    const INVALID_LABEL: u32 = u32::MAX;

    pub fn generate(mut self, status: &mut GloomStatus) -> ConstantPool {
        for script_body in status.script_bodies.iter_mut() {
            self.generate_func(&mut script_body.inner_mut().func);
        }
        for func in status.funcs.iter() {
            self.generate_func(&mut func.inner_mut());
        }
        for class in status.classes.iter() {
            for func in class.inner().funcs.iter() {
                self.generate_func(&mut func.inner_mut());
            }
        }
        for class in status.enums.iter() {
            for func in class.inner().funcs.iter() {
                self.generate_func(&mut func.inner_mut());
            }
        }
        self.constant_pool
    }
    fn generate_func(&mut self, func: &mut GloomFunc) {
        let mut context = GenerateContext::new(func.info.local_size * 4);
        match &func.body {
            FuncBody::AST(vec) => {
                self.generate_statements(vec, &mut context);
            }
            FuncBody::Builtin(_) => {},
            _ => panic!()
        };
        func.info.stack_size = context.stack_size();
        func.body = FuncBody::ByteCodes(context.bytecodes());
    }
    fn generate_statements(&mut self, statements: &Vec<Statement>, context: &mut GenerateContext) {
        for stmt in statements.iter() {
            match stmt {
                Statement::Let(let_info) => {
                    let (var, _, expr, _) = let_info.deref();
                    self.generate_expression(expr, context);
                    let code = match var {
                        Var::LocalInt(i1, i2) => ByteCode::WriteLocalInt(*i1, *i2),
                        Var::LocalNum(i1, i2) => ByteCode::WriteLocalNum(*i1, *i2),
                        Var::LocalChar(i1, i2) => ByteCode::WriteLocalChar(*i1, *i2),
                        Var::LocalBool(i1, i2) => ByteCode::WriteLocalBool(*i1, *i2),
                        Var::LocalRef(i) => ByteCode::WriteLocalRef(*i),
                        _ => panic!(),
                    };
                    context.push(code);
                }
                Statement::Static(static_info) | Statement::PubStatic(static_info) => {
                    let (var, _, expr) = static_info.deref();
                    self.generate_expression(expr, context);
                    let code = match var {
                        Var::StaticInt(i1, i2) => ByteCode::WriteStaticInt(*i1, *i2),
                        Var::StaticNum(i1, i2) => ByteCode::WriteStaticNum(*i1, *i2),
                        Var::StaticChar(i1, i2) => ByteCode::WriteStaticChar(*i1, *i2),
                        Var::StaticBool(i1, i2) => ByteCode::WriteStaticBool(*i1, *i2),
                        Var::StaticRef(i) => ByteCode::WriteStaticRef(*i),
                        _ => panic!()
                    };
                    context.push(code);
                }
                Statement::LeftValueOp(op_info) => {
                    let (left_value, operation) = op_info.deref();
                    match left_value {
                        LeftValue::Var(var) => {
                            if let LeftValueOp::Assign(_) = operation {
                                // do nothing
                            } else {
                                // 需要先读取值 need read value firstly
                                match var {
                                    Var::LocalInt(i1, i2)
                                    | Var::LocalNum(i1, i2)
                                    | Var::LocalChar(i1, i2)
                                    | Var::LocalBool(i1, i2) => ByteCode::ReadLocal(*i1, *i2),
                                    Var::LocalRef(i) => ByteCode::ReadLocal(*i, 0),
                                    Var::StaticInt(i1, i2)
                                    | Var::StaticNum(i1, i2)
                                    | Var::StaticChar(i1, i2)
                                    | Var::StaticBool(i1, i2) => ByteCode::ReadStatic(*i1, *i2),
                                    Var::StaticRef(i) => ByteCode::ReadStatic(*i, 0),
                                    _ => panic!()
                                };
                            };
                            match operation {
                                LeftValueOp::Assign(expr) => {
                                    self.generate_expression(expr,context);
                                }
                                LeftValueOp::PlusEq(expr) => {
                                    self.generate_expression(expr,context);
                                    context.push(ByteCode::Plus);
                                }
                                LeftValueOp::SubEq(expr) => {
                                    self.generate_expression(expr,context);
                                    context.push(ByteCode::Sub);
                                }
                                LeftValueOp::PlusOne => {
                                    context.push(ByteCode::LoadDirectInt(1));
                                    context.push(ByteCode::Plus);
                                }
                                LeftValueOp::SubOne => {
                                    context.push(ByteCode::LoadDirectInt(1));
                                    context.push(ByteCode::Sub);
                                }
                            }
                            match var {
                                Var::LocalInt(i1, i2) => ByteCode::WriteLocalInt(*i1,*i2),
                                Var::LocalNum(i1, i2) => ByteCode::WriteLocalNum(*i1,*i2),
                                Var::LocalChar(i1, i2) => ByteCode::WriteLocalChar(*i1,*i2),
                                Var::LocalBool(i1, i2) => ByteCode::WriteLocalBool(*i1,*i2),
                                Var::LocalRef(i) => ByteCode::WriteLocalRef(*i),
                                Var::StaticInt(i1, i2) => ByteCode::WriteStaticInt(*i1,*i2),
                                Var::StaticNum(i1, i2) => ByteCode::WriteLocalNum(*i1,*i2),
                                Var::StaticChar(i1, i2) => ByteCode::WriteLocalChar(*i1,*i2),
                                Var::StaticBool(i1, i2) => ByteCode::WriteLocalBool(*i1,*i2),
                                Var::StaticRef(i) =>  ByteCode::WriteLocalRef(*i),
                                _ => panic!()
                            };
                        }
                        LeftValue::Chain(first_elem, chains) => {
                            self.generate_expression(first_elem,context);
                            // the last chain must be a field-access
                            for chain in chains.as_slice()[0..chains.len()-1].iter() {
                                self.generate_chain(chain,context);
                            }
                            let ((slot_idx,sub_idx),field_type) = if let Chain::Access(var_id,field_type) = chains.last().unwrap() {
                                (var_id.index(),*field_type)
                            }else{
                                panic!()
                            };
                            if let LeftValueOp::Assign(_) = operation {
                                // do nothing
                            }else{
                                // need read field and do some calculation before write field
                                context.push(ByteCode::ReadField(slot_idx,sub_idx));
                            }
                            match operation {
                                LeftValueOp::Assign(expr) => {
                                    self.generate_expression(expr,context);
                                }
                                LeftValueOp::PlusEq(expr) => {
                                    self.generate_expression(expr,context);
                                    context.push(ByteCode::Plus);
                                }
                                LeftValueOp::SubEq(expr) => {
                                    self.generate_expression(expr,context);
                                    context.push(ByteCode::Sub);
                                }
                                LeftValueOp::PlusOne => {
                                    context.push(ByteCode::LoadDirectInt(1));
                                    context.push(ByteCode::Plus);
                                }
                                LeftValueOp::SubOne => {
                                    context.push(ByteCode::LoadDirectInt(1));
                                    context.push(ByteCode::Sub);
                                }
                            };
                            let write_field_code = match field_type {
                                BasicType::Int => ByteCode::WriteFieldInt(slot_idx,sub_idx),
                                BasicType::Num => ByteCode::WriteFieldNum(slot_idx,sub_idx),
                                BasicType::Char => ByteCode::WriteFieldChar(slot_idx,sub_idx),
                                BasicType::Bool => ByteCode::WriteFieldBool(slot_idx,sub_idx),
                                BasicType::Ref => ByteCode::WriteFieldRef(slot_idx),
                            };
                            context.push(write_field_code);
                        }
                    }
                }
                Statement::Expr(expr, _) | Statement::Discard(expr, _) => {
                    self.generate_expression(expr, context);
                    context.push(ByteCode::Pop);
                }
                Statement::Continue(_) => {
                    context.push(ByteCode::Jump(Self::INVALID_LABEL));
                }
                Statement::Break(expr, _) => {
                    self.generate_expression(expr, context);
                    context.push(ByteCode::Jump(Self::INVALID_LABEL));
                }
                Statement::Return(expr, _) => {
                    self.generate_expression(expr, context);
                    context.push(ByteCode::Return);
                }
                Statement::IfResult(expr, _) => {
                    self.generate_expression(expr, context);
                }
            }
        }
    }
    fn generate_expression(&mut self, expr: &Expression, context: &mut GenerateContext) {
        match expr {
            Expression::None => {},
            Expression::Int(i) => {
                let code = if *i <= i32::MAX as i64 && *i >= i32::MIN as i64 {
                    ByteCode::LoadDirectInt(*i as i32)
                }else{
                    let idx = self.constant_pool.int.len();
                    self.constant_pool.int.push(*i);
                    ByteCode::LoadConstInt(idx as u16)
                };
                context.push(code);
            }
            Expression::Num(n) => {
                let code = if *n <= f32::MAX as f64 && *n >= f32::MIN as f64 {
                    ByteCode::LoadDirectNum(*n as f32)
                }else{
                    let idx = self.constant_pool.num.len();
                    self.constant_pool.num.push(*n);
                    ByteCode::LoadConstNum(idx as u16)
                };
                context.push(code);
            }
            Expression::Char(ch) => {
                context.push(ByteCode::LoadDirectChar(*ch));
            }
            Expression::Bool(bl) => {
                context.push(ByteCode::LoadDirectBool(*bl));
            }
            Expression::Str(str) => {

            }
            Expression::Var(_) => {}
            Expression::Tuple(_) => {}
            Expression::Array(_) => {}
            Expression::Construct(_) => {}
            Expression::BinaryOp(_) => {}
            Expression::Cast(_) => {}
            Expression::NegOp(_) => {}
            Expression::NotOp(_) => {}
            Expression::IfElse(_) => {}
            Expression::While(_) => {}
            Expression::For(_) => {}
            Expression::Match(_) => {}
            Expression::Chain(_) => {}
            Expression::Func(_) => {}
        }
    }
    #[inline]
    fn generate_chain(&mut self, chain : &Chain, context : &mut GenerateContext){
        match chain {
            Chain::Access(field,_) => {
                let (slot_idx,sub_idx) = field.index();
                context.push(ByteCode::ReadField(slot_idx,sub_idx));
            }
            Chain::FnCall{
                func,
                need_self,
                args
            } => {
                for arg_expr in args.iter(){
                    self.generate_expression(arg_expr,context);
                }
                let call_code = if *need_self {
                    ByteCode::CallMethod {
                        index: func.index().0,
                        nargs: args.len() as u16
                    }
                }else {
                    ByteCode::CallStaticFn {
                        index: func.index().0,
                        nargs: args.len() as u16
                    }
                };
                context.push(call_code);
            }
            Chain::Call(args) => {
                for arg_expr in args.iter(){
                    self.generate_expression(arg_expr,context);
                }
                context.push(ByteCode::CallTopFn{ nargs: args.len() as u16 })
            }
        }
    }
    pub fn new() -> Self {
        CodeGenerator {
            constant_pool: ConstantPool::new(),
        }
    }
}

struct GenerateContext {
    bytecodes: Vec<ByteCode>,
    max_stack_size: u16,
    curr_stack_size: i16,
}

impl GenerateContext {
    fn new(codes: u16) -> Self {
        GenerateContext {
            bytecodes: Vec::with_capacity(codes as usize),
            max_stack_size: 0,
            curr_stack_size: 0,
        }
    }
    fn bytecodes(self) -> Vec<ByteCode> {
        self.bytecodes
    }
    #[inline]
    fn push(&mut self, bytecode: ByteCode) {
        let stack_affect = bytecode.stack_affect();
        self.curr_stack_size += stack_affect as i16;
        if self.curr_stack_size as u16 > self.max_stack_size {
            self.max_stack_size = self.curr_stack_size as u16;
        }
        self.bytecodes.push(bytecode);
    }
    fn stack_size(&self) -> u16 {
        self.max_stack_size
    }
}
