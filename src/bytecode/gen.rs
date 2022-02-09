use std::borrow::Borrow;
use crate::builtin::string::GloomString;
use crate::bytecode::code::ByteCode;
use crate::frontend::ast::{Chain, Expression, ExprType, LeftValue, Statement, Var};
use crate::frontend::ops::{BinOp, LeftValueOp};
use crate::frontend::status::GloomStatus;
use crate::obj::func::{FuncBody, GloomFunc};
use crate::obj::types::{BasicType, DataType, RefType};
use crate::vm::constant::ConstantPool;
use std::ops::Deref;

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
        match &func.body {
            FuncBody::AST(vec) => {
                let mut context = GenerateContext::new(func.info.local_size * 4);
                self.generate_statements(vec, &mut context);
                context.push(ByteCode::Return);
                func.info.stack_size = context.stack_size();
                func.body = FuncBody::ByteCodes(context.bytecodes());
            }
            FuncBody::Builtin(_) => {}
            _ => panic!(),
        };

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
                        _ => panic!(),
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
                                    _ => panic!(),
                                };
                            };
                            match operation {
                                LeftValueOp::Assign(expr) => {
                                    self.generate_expression(expr, context);
                                }
                                LeftValueOp::PlusEq(expr) => {
                                    self.generate_expression(expr, context);
                                    context.push(ByteCode::Plus);
                                }
                                LeftValueOp::SubEq(expr) => {
                                    self.generate_expression(expr, context);
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
                                Var::LocalInt(i1, i2) => ByteCode::WriteLocalInt(*i1, *i2),
                                Var::LocalNum(i1, i2) => ByteCode::WriteLocalNum(*i1, *i2),
                                Var::LocalChar(i1, i2) => ByteCode::WriteLocalChar(*i1, *i2),
                                Var::LocalBool(i1, i2) => ByteCode::WriteLocalBool(*i1, *i2),
                                Var::LocalRef(i) => ByteCode::WriteLocalRef(*i),
                                Var::StaticInt(i1, i2) => ByteCode::WriteStaticInt(*i1, *i2),
                                Var::StaticNum(i1, i2) => ByteCode::WriteLocalNum(*i1, *i2),
                                Var::StaticChar(i1, i2) => ByteCode::WriteLocalChar(*i1, *i2),
                                Var::StaticBool(i1, i2) => ByteCode::WriteLocalBool(*i1, *i2),
                                Var::StaticRef(i) => ByteCode::WriteLocalRef(*i),
                                _ => panic!(),
                            };
                        }
                        LeftValue::Chain(first_elem, chains) => {
                            self.generate_expression(first_elem, context);
                            // the last chain must be a field-access
                            for chain in chains.as_slice()[0..chains.len() - 1].iter() {
                                self.generate_chain(chain, context);
                            }
                            let ((slot_idx, sub_idx), field_type) =
                                if let Chain::Access(var_id, field_type) = chains.last().unwrap() {
                                    (var_id.index(), *field_type)
                                } else {
                                    panic!()
                                };
                            if let LeftValueOp::Assign(_) = operation {
                                // do nothing
                            } else {
                                // need read field and do some calculation before write field
                                context.push(ByteCode::ReadField(slot_idx, sub_idx));
                            }
                            match operation {
                                LeftValueOp::Assign(expr) => {
                                    self.generate_expression(expr, context);
                                }
                                LeftValueOp::PlusEq(expr) => {
                                    self.generate_expression(expr, context);
                                    context.push(ByteCode::Plus);
                                }
                                LeftValueOp::SubEq(expr) => {
                                    self.generate_expression(expr, context);
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
                                BasicType::Int => ByteCode::WriteFieldInt(slot_idx, sub_idx),
                                BasicType::Num => ByteCode::WriteFieldNum(slot_idx, sub_idx),
                                BasicType::Char => ByteCode::WriteFieldChar(slot_idx, sub_idx),
                                BasicType::Bool => ByteCode::WriteFieldBool(slot_idx, sub_idx),
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
            Expression::None => {}
            Expression::Int(i) => {
                let code = if *i <= i32::MAX as i64 && *i >= i32::MIN as i64 {
                    ByteCode::LoadDirectInt(*i as i32)
                } else {
                    let idx = self.constant_pool.int.len();
                    self.constant_pool.int.push(*i);
                    ByteCode::LoadConstInt(idx as u16)
                };
                context.push(code);
            }
            Expression::Num(n) => {
                let code = if *n <= f32::MAX as f64 && *n >= f32::MIN as f64 {
                    ByteCode::LoadDirectNum(*n as f32)
                } else {
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
                let idx = self.constant_pool.str.len() as u16;
                self.constant_pool.str.push(GloomString::new(String::clone(str)));
                context.push(ByteCode::LoadConstString(idx));
            }
            Expression::Var(var) => {
                let code = match var.deref() {
                    Var::LocalInt(i1, i2)
                    | Var::LocalNum(i1, i2)
                    | Var::LocalChar(i1, i2)
                    | Var::LocalBool(i1, i2) => ByteCode::ReadLocal(*i1, *i2),
                    Var::LocalRef(i) => ByteCode::ReadLocal(*i, 0),
                    Var::StaticInt(i1, i2)
                    | Var::StaticNum(i1, i2)
                    | Var::StaticChar(i1, i2)
                    | Var::StaticBool(i1, i2) => ByteCode::ReadStatic(*i1,*i2),
                    Var::StaticRef(i) => ByteCode::ReadStatic(*i,0),
                    Var::Class(i) => ByteCode::LoadClass(*i),
                    Var::Enum(i) => ByteCode::LoadEnum(*i),
                    Var::BuiltinType(i) => ByteCode::LoadBuiltinType(*i),
                    Var::DirectFn(i) => ByteCode::LoadDirectDefFn(*i),
                    _ => panic!(),
                };
                context.push(code);
            }
            Expression::Tuple(tuple) => {
                for expr in tuple.deref().iter().rev() {
                    self.generate_expression(expr,context);
                }
                context.push(ByteCode::CollectTuple(tuple.len() as u16));
            }
            Expression::Array(array) => {
                let (array,basic_type,is_queue) = array.deref();
                for expr in array.iter().rev() {
                    self.generate_expression(expr,context);
                }
                let code = if *is_queue {
                    ByteCode::CollectQueue(*basic_type,array.len() as u16)
                }else {
                    ByteCode::CollectArray(*basic_type,array.len() as u16)
                };
                context.push(code);
            }
            Expression::NegOp(expr) => {
                self.generate_expression(expr,context);
                context.push(ByteCode::NegOp);
            }
            Expression::NotOp(expr) => {
                self.generate_expression(expr,context);
                context.push(ByteCode::NotOp);
            }
            Expression::BinaryOp(bin_op_vec) => {
                self.generate_expression(&bin_op_vec.left,context);
                for (bin_op,expr) in bin_op_vec.vec.iter() {
                    self.generate_expression(expr,context);
                    context.push(match bin_op {
                        BinOp::Plus => ByteCode::Plus,
                        BinOp::Sub => ByteCode::Sub,
                        BinOp::Mul => ByteCode::Mul,
                        BinOp::Div => ByteCode::Div,
                        BinOp::Gt => ByteCode::GreaterThan,
                        BinOp::Lt => ByteCode::LessThan,
                        BinOp::GtEq => ByteCode::GreaterThanEquals,
                        BinOp::LtEq => ByteCode::LessThanEquals,
                        BinOp::Eqs => ByteCode::Equals,
                        BinOp::NotEq => ByteCode::NotEquals,
                        BinOp::And => ByteCode::LogicAnd,
                        BinOp::Or => ByteCode::LogicOr,
                    });
                }
            }
            Expression::Construct(construction) => {
                let class_index = match construction.class_type.borrow() {
                    ExprType::Analyzed(DataType::Ref(RefType::Class(class))) => {
                        class.inner().class_index
                    }
                    _ => panic!()
                };
                context.push(ByteCode::Construct(class_index));
                for (var_idx,field_type,expr) in construction.fields.iter(){
                    self.generate_expression(expr,context);
                    let (slot_idx,sub_idx) = var_idx.index();
                    context.push(match field_type {
                        BasicType::Int => ByteCode::WriteFieldInt(slot_idx,sub_idx),
                        BasicType::Num => ByteCode::WriteFieldNum(slot_idx,sub_idx),
                        BasicType::Char => ByteCode::WriteFieldChar(slot_idx,sub_idx),
                        BasicType::Bool => ByteCode::WriteFieldBool(slot_idx,sub_idx),
                        BasicType::Ref => ByteCode::WriteFieldRef(slot_idx),
                    });
                }
            }
            Expression::Chain(chain) => {
                let (expr,chains) = chain.deref();
                self.generate_expression(expr,context);
                for chain in chains.iter() {
                    self.generate_chain(chain,context);
                }
            }
            Expression::IfElse(if_else) => {
                let start_idx = context.bytecodes.len();
                let mut last_cond_idx ;
                let max_idx = if_else.branches.len() - 1;
                for (idx,branch) in if_else.branches.iter().enumerate() {
                    self.generate_expression(&branch.condition,context);
                    last_cond_idx = context.bytecodes.len();
                    // 如果条件为真 就顺序执行 如果为假 则跳转到下一个条件判断
                    // if condition is true, execute orderly, or if false, jump to next condition judge
                    context.push(ByteCode::JumpIfNot(Self::INVALID_LABEL));
                    self.generate_statements(&branch.statements,context);

                    // 执行完某一个分支，应当跳转到这一系列 if/elseif 分支的末尾
                    // should jump to the end index of this a series of if/elseif branch after execution in a branch
                    if idx < max_idx {
                        context.push(ByteCode::Jump(Self::INVALID_LABEL));
                    }

                    // 本次if分支的条件判断JumpIfNot应该指向到当下位置
                    // the JumpIfNot code that used to judge the condition of this if-branch should pointer to current index
                    let curr_idx = context.bytecodes.len();
                    let jump_code = context.bytecodes.get_mut(last_cond_idx).unwrap();
                    if let ByteCode::JumpIfNot(label) = jump_code {
                        if *label == Self::INVALID_LABEL {
                            *label = curr_idx as u32;
                        }
                    }else{
                        panic!()
                    }
                }

                // 将所有的Jump指令（跳转到一系列if/elseif末尾）中的label替换为真正结尾的索引
                // replace all the label of Jump instruction (jump to the end of a series if/elseif branches) with real end index
                let end_idx = context.bytecodes.len();
                for byte_code in context.bytecodes.as_mut_slice()[start_idx..end_idx].iter_mut() {
                    if let ByteCode::Jump(label) = byte_code {
                        if *label == Self::INVALID_LABEL {
                            *label = end_idx as u32;
                        }
                    }
                }
            }
            Expression::While(_) => {}
            Expression::For(_) => {}
            Expression::Cast(_) => {}
            Expression::Func(_) => {}
            Expression::Match(_) => {}
        }
    }
    #[inline]
    fn generate_chain(&mut self, chain: &Chain, context: &mut GenerateContext) {
        match chain {
            Chain::Access(field, _) => {
                let (slot_idx, sub_idx) = field.index();
                context.push(ByteCode::ReadField(slot_idx, sub_idx));
            }
            Chain::FnCall {
                func,
                need_self,
                args,
            } => {
                for arg_expr in args.iter() {
                    self.generate_expression(arg_expr, context);
                }
                let call_code = if *need_self {
                    ByteCode::CallMethod {
                        index: func.index().0,
                        nargs: args.len() as u16,
                    }
                } else {
                    ByteCode::CallStaticFn {
                        index: func.index().0,
                        nargs: args.len() as u16,
                    }
                };
                context.push(call_code);
            }
            Chain::Call(args) => {
                for arg_expr in args.iter() {
                    self.generate_expression(arg_expr, context);
                }
                context.push(ByteCode::CallTopFn {
                    nargs: args.len() as u16,
                })
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
        self.curr_stack_size += stack_affect;
        if self.curr_stack_size as u16 > self.max_stack_size {
            self.max_stack_size = self.curr_stack_size as u16;
        }
        self.bytecodes.push(bytecode);
    }
    fn stack_size(&self) -> u16 {
        self.max_stack_size
    }
}
