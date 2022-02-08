use crate::frontend::ops::BinOp;

#[derive(Debug, Copy, Clone)]
pub enum ByteCode {
    Pop,
    CopyTop,

    LoadConstString(u16),
    LoadConstInt(u16),
    LoadDirectInt(i32),
    LoadDirectNum(f32),
    LoadConstNum(u16),
    LoadConstChar(char),
    LoadConstBool(bool),

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
    WriteStatic(u16, u8),

    ReadField(u16, u8),
    WriteField(u16, u8),

    DropLocal(u16),
    BinaryOps(BinOp),
    NotOp,
    NegOp,

    LoadDirectFn(u16),

    CallTopFn { index: u16, nargs: u16 }, // call the func obj of stack top 调用栈顶的函数对象
    CallStaticFn { index: u16, nargs: u16 },
    CallMethod { index: u16, nargs: u16 },

    JumpIf(u32),
    Jump(u32),
    Return,
}

impl ByteCode {
    #[inline]
    pub fn stack_affect(self) -> i8 {
        match self {
            ByteCode::Pop => -1,

            ByteCode::LoadConstString(_)
            | ByteCode::LoadConstInt(_)
            | ByteCode::LoadConstNum(_)
            | ByteCode::LoadDirectInt(_)
            | ByteCode::LoadDirectNum(_)
            | ByteCode::LoadConstChar(_)
            | ByteCode::LoadConstBool(_)
            | ByteCode::LoadClass(_)
            | ByteCode::LoadEnum(_)
            | ByteCode::ReadStatic(_, _)
            | ByteCode::LoadBuiltinType(_)
            | ByteCode::ReadField(_, _)
            | ByteCode::LoadDirectFn(_)
            | ByteCode::ReadLocal(_, _)
            | ByteCode::CopyTop => 1,

            ByteCode::WriteLocalInt(_, _)
            | ByteCode::WriteLocalNum(_, _)
            | ByteCode::WriteLocalChar(_, _)
            | ByteCode::WriteLocalBool(_, _)
            | ByteCode::WriteLocalRef(_)
            | ByteCode::WriteStatic(_, _)
            | ByteCode::WriteField(_, _)
            | ByteCode::BinaryOps(_) => -1,

            ByteCode::JumpIf(_) => -1,

            ByteCode::Jump(_)
            | ByteCode::DropLocal(_)
            | ByteCode::NotOp
            | ByteCode::NegOp => 0,

            ByteCode::CallTopFn { index: _, nargs }
            | ByteCode::CallStaticFn { index: _, nargs }
            | ByteCode::CallMethod { index: _, nargs } => - (nargs as i8),

            ByteCode::Return => 0,

        }
    }
}
