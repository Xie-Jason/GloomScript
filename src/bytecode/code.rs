use crate::obj::types::BasicType;

#[derive(Debug, Copy, Clone)]
pub enum ByteCode {
    Pop,
    CopyTop,

    LoadConstString(u16),
    LoadConstInt(u16),
    LoadDirectInt(i32),
    LoadDirectNum(f32),
    LoadConstNum(u16),
    LoadDirectChar(char),
    LoadDirectBool(bool),

    LoadClass(u16),
    LoadEnum(u16),
    LoadBuiltinType(u16),

    ReadLocal(u16, u8),

    WriteLocalInt(u16, u8),
    WriteLocalNum(u16, u8),
    WriteLocalChar(u16, u8),
    WriteLocalBool(u16, u8),
    WriteLocalRef(u16),

    ReadStatic(u16, u8),

    WriteStaticInt(u16, u8),
    WriteStaticNum(u16, u8),
    WriteStaticChar(u16, u8),
    WriteStaticBool(u16, u8),
    WriteStaticRef(u16),

    ReadField(u16, u8),

    WriteFieldInt(u16, u8),
    WriteFieldNum(u16, u8),
    WriteFieldChar(u16, u8),
    WriteFieldBool(u16, u8),
    WriteFieldRef(u16),

    DropLocal(u16),
    NotOp,
    NegOp,

    Plus,
    Sub,
    Mul,
    Div,
    GreaterThan,
    LessThan,
    GreaterThanEquals,
    LessThanEquals,
    Equals,
    NotEquals,
    LogicAnd,
    LogicOr,

    LoadDirectDefFn(u16),

    // the arg of fn call is reversed, the first popped value if the last arg
    // pop the fn obj of stack top and call it, push result
    CallTopFn { nargs: u16 },
    // call a fn in the class of obj
    CallStaticFn { index: u16, nargs: u16 },
    // call a fn in the class of obj, need self
    CallMethod { index: u16, nargs: u16 },

    CollectTuple(u16),
    CollectArray(BasicType,u16),
    CollectQueue(BasicType,u16),

    // pop three int and push RangeIter
    RangeIter,

    // pop
    InvokeIter,
    InvokeNext,

    Construct(u16),

    // pop the condition after jump
    JumpIf(u32),
    JumpIfNot(u32),
    Jump(u32),
    JumpIfNone(u32),

    Return,
}

impl ByteCode {
    #[inline]
    pub fn stack_affect(self) -> i16 {
        match self {
            ByteCode::Pop => -1,

            ByteCode::LoadConstString(_)
            | ByteCode::LoadConstInt(_)
            | ByteCode::LoadConstNum(_)
            | ByteCode::LoadDirectInt(_)
            | ByteCode::LoadDirectNum(_)
            | ByteCode::LoadDirectChar(_)
            | ByteCode::LoadDirectBool(_)
            | ByteCode::LoadClass(_)
            | ByteCode::LoadEnum(_)
            | ByteCode::ReadStatic(_, _)
            | ByteCode::LoadBuiltinType(_)
            | ByteCode::ReadField(_, _)
            | ByteCode::LoadDirectDefFn(_)
            | ByteCode::ReadLocal(_, _)
            | ByteCode::CopyTop => 1,

            ByteCode::WriteLocalInt(_, _)
            | ByteCode::WriteLocalNum(_, _)
            | ByteCode::WriteLocalChar(_, _)
            | ByteCode::WriteLocalBool(_, _)
            | ByteCode::WriteLocalRef(_)
            | ByteCode::WriteStaticInt(_, _)
            | ByteCode::WriteStaticNum(_, _)
            | ByteCode::WriteStaticChar(_, _)
            | ByteCode::WriteStaticBool(_, _)
            | ByteCode::WriteStaticRef(_)
            | ByteCode::WriteFieldInt(_, _)
            | ByteCode::WriteFieldNum(_, _)
            | ByteCode::WriteFieldChar(_, _)
            | ByteCode::WriteFieldBool(_, _)
            | ByteCode::WriteFieldRef(_) => -1,

            ByteCode::JumpIf(_)
            | ByteCode::JumpIfNot(_)
            | ByteCode::JumpIfNone(_) => -1,

            ByteCode::Jump(_) | ByteCode::DropLocal(_) | ByteCode::NotOp | ByteCode::NegOp => 0,

            ByteCode::CallTopFn { nargs }
            | ByteCode::CallStaticFn { index: _, nargs }
            | ByteCode::CallMethod { index: _, nargs } => -(nargs as i16),

            ByteCode::Return => 0,

            ByteCode::Plus
            | ByteCode::Sub
            | ByteCode::Mul
            | ByteCode::Div
            | ByteCode::GreaterThan
            | ByteCode::LessThan
            | ByteCode::GreaterThanEquals
            | ByteCode::LessThanEquals
            | ByteCode::Equals
            | ByteCode::NotEquals
            | ByteCode::LogicAnd
            | ByteCode::LogicOr => -1,

            ByteCode::CollectTuple(i)
            | ByteCode::CollectArray(_,i)
            | ByteCode::CollectQueue(_,i) => - (i as i16),

            ByteCode::Construct(_) => 1,

            // pop source object after invoke iter() fn
            ByteCode::InvokeIter => 0,

            ByteCode::InvokeNext => 1,

            ByteCode::RangeIter => -2,
        }
    }
}
