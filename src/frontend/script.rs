use std::rc::Rc;

use crate::frontend::ast::{ParsedClass, ParsedEnum, ParsedFunc, ParsedInterface, Statement};
use crate::obj::func::GloomFunc;

pub struct ParsedFile {
    pub imports: Vec<ParsedFile>,
    pub classes: Vec<(ParsedClass, bool)>,
    pub interfaces: Vec<(ParsedInterface, bool)>,
    pub enums: Vec<(ParsedEnum, bool)>,
    pub funcs: Vec<(Rc<String>, ParsedFunc, bool)>,
    pub statements: Vec<Statement>,
    pub path : String,
    pub index: u16,
}

#[derive(Debug)]
pub struct ScriptBody {
    pub file_index: u16,
    pub func: GloomFunc,
}

impl ScriptBody {
    pub fn new(func: GloomFunc, file_index: u16) -> ScriptBody {
        ScriptBody {
            file_index,
            func,
        }
    }
}