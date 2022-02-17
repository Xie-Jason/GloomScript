use std::fmt::{Debug, Display, Formatter};

use crate::builtin::classes::BuiltinClass;
use crate::builtin::funcs::BuiltInFuncs;
use crate::frontend::script::ScriptBody;
use crate::obj::class::GloomClass;
use crate::obj::func::GloomFunc;
use crate::obj::gloom_enum::GloomEnumClass;
use crate::obj::interface::Interface;
use crate::obj::refcount::RefCount;

// 120bytes
pub struct GloomStatus {
    pub builtin_classes: Vec<RefCount<BuiltinClass>>,
    pub classes: Vec<RefCount<GloomClass>>,
    pub interfaces: Vec<RefCount<Interface>>,
    pub enums: Vec<RefCount<GloomEnumClass>>,
    pub funcs: Vec<RefCount<GloomFunc>>,
    pub script_bodies: Vec<RefCount<ScriptBody>>,
}

impl Debug for GloomStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "GloomStatus\r\nClass :\r\n{:?} \r\nInterface :\r\n{:?} \r\nEnum :\r\n{:?} \r\nFuncs :\r\n{:?}",
                 self.classes,
                 self.interfaces,
                 self.enums,
                 self.funcs)
    }
}

pub struct TypeIndex {
    pub index: u16,
    pub file_index: u16,
    pub is_public: bool,
    pub tp: MetaType,
}

impl TypeIndex {
    #[inline]
    pub fn builtin(index: u16) -> TypeIndex {
        TypeIndex {
            index,
            file_index: 0,
            is_public: true,
            tp: MetaType::Builtin,
        }
    }
}

#[derive(PartialEq)]
pub enum MetaType {
    Interface,
    Class,
    Enum,
    Builtin,
}

impl Display for MetaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MetaType::Interface => "Interface",
                MetaType::Class => "Class",
                MetaType::Enum => "Enum",
                MetaType::Builtin => "BuiltinType",
            }
        )
    }
}

impl TypeIndex {
    pub fn from(index: u16, is_public: bool, file_index: u16, tp: MetaType) -> TypeIndex {
        TypeIndex {
            index,
            file_index,
            is_public,
            tp,
        }
    }
}

impl GloomStatus {
    pub fn new() -> GloomStatus {
        GloomStatus {
            builtin_classes: BuiltinClass::classes(),
            classes: Vec::new(),
            interfaces: Vec::new(),
            enums: Vec::new(),
            funcs: BuiltInFuncs::func_list(),
            script_bodies: Vec::new(),
        }
    }
}
