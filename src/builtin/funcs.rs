use std::rc::Rc;
use std::io::prelude::*;

use hashbrown::HashMap;

use crate::obj::class::IsPub;
use crate::obj::func::{GloomFunc, Param, ReturnType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};
use crate::vm::value::Value;

pub struct BuiltInFuncs;

pub type IsBuiltIn = bool;

impl BuiltInFuncs {
    fn func_println(empty: Rc<String>) -> RefCount<GloomFunc> {
        let params = vec![Param::new(empty, DataType::Ref(RefType::Any))];
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("println")),
            params,
            ReturnType::Void,
            false,
            Rc::new(|_, mut args| {
                let obj = args.vec.pop().unwrap();
                println!("{:?}", obj);
                Value::None
            }),
        ))
    }
    fn func_print(empty: Rc<String>) -> RefCount<GloomFunc> {
        let params = vec![Param::new(empty, DataType::Ref(RefType::Any))];
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("println")),
            params,
            ReturnType::Void,
            false,
            Rc::new(|_, mut args| {
                let obj = args.vec.pop().unwrap();
                print!("{:?}", obj);
                std::io::stdout().flush().unwrap();
                Value::None
            }),
        ))
    }
    pub fn func_list() -> Vec<RefCount<GloomFunc>> {
        let empty_name = Rc::new(String::from(""));
        vec![Self::func_println(empty_name.clone()),Self::func_print(empty_name)]
    }
    pub fn func_map() -> HashMap<String, (u16, IsBuiltIn, IsPub, u16)> {
        let mut map = HashMap::new();
        map.insert(String::from("println"), (0, true, true, 0));
        map.insert(String::from("print"), (0, true, true, 0));
        map
    }
}
