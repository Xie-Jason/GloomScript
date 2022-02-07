use crate::bytecode::code::ByteCode;
use crate::frontend::ast::{Expression, Statement, Var};
use crate::frontend::status::GloomStatus;
use crate::obj::func::{FuncBody, GloomFunc};
use std::ops::Deref;
use crate::vm::constant::ConstantPool;

pub struct CodeGenerator {
    constant_pool : ConstantPool,
}

impl CodeGenerator {
    pub fn generate(mut self, status : &mut GloomStatus) -> ConstantPool {
        self.constant_pool
    }
    fn generate_func(&mut self, func: &mut GloomFunc) {
        let mut context = GenerateContext::new(func.info.local_size * 4);
        match &func.body {
            FuncBody::AST(vec) => {
                self.generate_statements(vec, &mut context);
            }
            _ => panic!(),
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
                        Var::LocalInt(i1,i2) => ByteCode::WriteLocalInt(*i1,*i2),
                        Var::LocalNum(i1,i2) => ByteCode::WriteLocalNum(*i1,*i2),
                        Var::LocalChar(i1,i2) => ByteCode::WriteLocalChar(*i1,*i2),
                        Var::LocalBool(i1,i2) => ByteCode::WriteLocalBool(*i1,*i2),
                        Var::LocalRef(i) =>  ByteCode::WriteLocalRef(*i),
                        _ => panic!()
                    };
                    context.push(code);
                }
                Statement::Static(static_info)
                |Statement::PubStatic(static_info) => {
                    let (var,_,expr) = static_info.deref();
                    self.generate_expression(expr,context);
                    let code = match var {
                        Var::StaticInt(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticNum(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticChar(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticBool(i1, i2) => ByteCode::WriteStatic(*i1, *i2),
                        Var::StaticRef(i) => ByteCode::WriteStatic(*i,0),
                        _ => panic!()
                    };
                    context.push(code);
                }
                Statement::LeftValueOp(op_info) => {

                }
                Statement::Expr(expr, _) | Statement::Discard(expr, _) => {
                    self.generate_expression(expr,context);
                    context.push(ByteCode::Pop);
                }
                Statement::Continue(_) => {}
                Statement::Break(_, _) => {}
                Statement::Return(expr, _) => {
                    self.generate_expression(expr,context);
                    context.push(ByteCode::Return);
                }
                Statement::IfResult(expr, _) => {
                    self.generate_expression(expr,context);
                }
            }
        }
    }
    fn generate_expression(&mut self, expr: &Expression, context: &mut GenerateContext) {}
    pub fn new() -> Self {
        CodeGenerator {
            constant_pool: ConstantPool::new()
        }
    }
}

struct GenerateContext {
    bytecodes: Vec<ByteCode>,
    max_stack_size: u16,
    curr_stack_size: u16,
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
        if self.curr_stack_size > self.max_stack_size {
            self.max_stack_size = self.curr_stack_size;
        }
        self.bytecodes.push(bytecode);
    }
    fn stack_size(&self) -> u16 {
        self.max_stack_size
    }
}
