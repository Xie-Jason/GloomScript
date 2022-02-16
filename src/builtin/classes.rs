use core::fmt::{Debug, Formatter};

use hashbrown::HashMap;

use crate::frontend::status::TypeIndex;
use crate::obj::func::GloomFunc;
use crate::obj::refcount::RefCount;
use crate::obj::types::{BuiltinType, DataType, RefType};

pub struct BuiltinClass {
    pub name: String,
    pub map: HashMap<String, u16>,
    pub funcs: Vec<RefCount<GloomFunc>>,
    pub get_ref_type_fn: Box<dyn Fn(Option<Vec<DataType>>) -> Result<RefType, String>>,
}

impl BuiltinClass {
    pub fn get_ref_type(&self, generic: Option<Vec<DataType>>) -> Result<RefType, String> {
        (self.get_ref_type_fn)(generic)
    }
    pub fn classes() -> Vec<RefCount<BuiltinClass>> {
        let mut vec = Vec::new();
        vec.push(RefCount::new(Self::gloom_string_class()));
        vec.push(RefCount::new(Self::gloom_func_class()));
        vec.push(RefCount::new(Self::gloom_array_class()));
        vec
    }
    pub fn class_map() -> HashMap<String, TypeIndex> {
        let mut map = HashMap::new();
        map.insert(String::from("String"), TypeIndex::builtin(0));
        map.insert(String::from("Func"), TypeIndex::builtin(1));
        map.insert(String::from("Array"), TypeIndex::builtin(2));
        map
    }
    pub fn builtin_type_map() -> HashMap<BuiltinType, u16> {
        let mut map = HashMap::new();
        map.insert(BuiltinType::String, 0);
        map.insert(BuiltinType::Func, 1);
        map
    }

    pub const STRING_INDEX: usize = 0;
    pub const FUNC_INDEX: usize = 1;

    pub fn none_generic_fn(
        ref_type: RefType,
    ) -> Box<dyn Fn(Option<Vec<DataType>>) -> Result<RefType, String>> {
        Box::new(move |option| {
            if let Some(generic) = option {
                if generic.len() > 0 {
                    return Result::Err(format!("unexpected generic type {:?}", generic));
                }
            }
            Result::Ok(ref_type.clone())
        })
    }
}

impl Debug for BuiltinClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BuiltinType {} {:?} {:?}",
            self.name.as_str(),
            self.map,
            self.funcs
        )
    }
}
