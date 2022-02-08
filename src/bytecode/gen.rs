use crate::bytecode::code::ByteCode;
use crate::frontend::ast::{Expression, LeftValue, Statement, Var};
use crate::frontend::ops::LeftValueOp;
use crate::frontend::status::GloomStatus;
use crate::obj::func::{FuncBody, GloomFunc};
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
        let mut context = GenerateContext::new(func.info.local_size * 4);
        match &func.body {
            FuncBody::AST(vec) => {
                self.generate_statements(vec, &mut context);
            }
            FuncBody::Builtin(_) => {},
            _ => panic!()
        };
        func.info.stack_size = context.stack_size();
        std::mem::replace(&mut func.body, FuncBody::ByteCodes(context.bytecodes()));
    }
    fn generate_statements(&mut self, statements: &Vec<Statement>, context: &mut GenerateContext) {
        for stmt in statements.iter() {
            match stmt {
                Statement::Let(let_info) => {
                    let (var, _, expr, line) = let_info.deref();
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
                        Var::StaticInt(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticNum(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticChar(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticBool(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticRef(i) => ByteCode::WriteStatic(*i, 0),
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

                                }
                                LeftValueOp::SubEq(expr) => {
                                    self.generate_expression(expr,context);

                                }
                                LeftValueOp::PlusOne => {
                                    context.push(ByteCode::LoadDirectInt(1));
                                }
                                LeftValueOp::SubOne => {
                                    context.push(ByteCode::LoadDirectInt(1));

                                }
                            }
                            match var {
                                Var::LocalInt(_, _) => {}
                                Var::LocalNum(_, _) => {}
                                Var::LocalChar(_, _) => {}
                                Var::LocalBool(_, _) => {}
                                Var::LocalRef(_) => {}
                                Var::StaticInt(_, _) => {}
                                Var::StaticNum(_, _) => {}
                                Var::StaticChar(_, _) => {}
                                Var::StaticBool(_, _) => {}
                                Var::StaticRef(_) => {}
                                _ => panic!()
                            }
                        }
                        LeftValue::Chain(first_elem, chains) => match operation {
                            LeftValueOp::Assign(expr) => {}
                            LeftValueOp::PlusEq(expr) => {}
                            LeftValueOp::SubEq(expr) => {}
                            LeftValueOp::PlusOne => {}
                            LeftValueOp::SubOne => {}
                        },
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
    fn generate_expression(&mut self, expr: &Expression, context: &mut GenerateContext) {}
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
