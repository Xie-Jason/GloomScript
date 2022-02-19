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
    fn func_format(empty: Rc<String>) -> RefCount<GloomFunc>{
        let params = vec![Param::new(empty, DataType::Ref(RefType::Any))];
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("format")),
            params,
            ReturnType::Have(DataType::Ref(RefType::String)),
            false,
            Rc::new(|_, mut args| {
                let arg = args.vec.pop().unwrap();
                let mode =arg.as_ref().downcast::<GloomString>().0.borrow().to_string();
                while let Some(obj) = args.vec.pop(){
                    let mode = format!(format!("{}",mode),obj);
                }
                Value::Ref(GloomString::new(mode))
            }),
        ))
    }
    fn func_println(empty: Rc<String>) -> RefCount<GloomFunc> {
        let params = vec![Param::new(empty, DataType::Ref(RefType::Any))];
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("println")),
            params,
            ReturnType::Void,
            false,
            Rc::new(|_, mut args| {
                while let Some(obj) = args.vec.pop(){
                print!("{:?}", obj);
                }
                Value::None
            }),
        ))
    }
    fn func_print(empty: Rc<String>) -> RefCount<GloomFunc> {
        let params = vec![Param::new(empty, DataType::Ref(RefType::Any))];
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("print")),
            params,
            ReturnType::Void,
            false,
            Rc::new(|_, mut args| {
                while let Some(obj) = args.vec.pop(){
                print!("{:?}", obj);
                }
                std::io::stdout().flush().unwrap();
                Value::None
            }),
        ))
    }
    fn func_input() -> RefCount<GloomFunc>{
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("input")),
            Vec::with_capacity(0),
            ReturnType::Have(DataType::Ref(RefType::String)),
            false,
            Rc::new(|_, _| {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf).unwrap();
                Value::Ref(GloomString::new(buf.trim_end().to_string()))
            }),
        ))
    }
    fn func_to_int(empty: Rc<String>) -> RefCount<GloomFunc>{
        let params = vec![Param::new(empty, DataType::Ref(RefType::String))];
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("to_int")),
            params,
            ReturnType::Have(DataType::Int),
            false,
            Rc::new(|_,mut args|{
                let arg  = args.vec.pop().unwrap();// arg:Value::Ref(GloomString(RefCell))
                let rstr = arg.as_ref().downcast::<GloomString>().0.borrow();
                let rint = rstr.parse().unwrap();
                Value::Int(rint)
            })),
        )
    }
    fn func_to_num(empty: Rc<String>) -> RefCount<GloomFunc>{
        let params = vec![Param::new(empty, DataType::Ref(RefType::String))];
        RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("to_num")),
            params,
            ReturnType::Have(DataType::Int),
            false,
            Rc::new(|_,mut args|{
                let arg  = args.vec.pop().unwrap();
                let rstr = arg.as_ref().downcast::<GloomString>().0.borrow();
                let rnum = rstr.parse().unwrap();
                Value::Num(rnum)
            })),
        )
    }
    pub fn func_list() -> Vec<RefCount<GloomFunc>> {
        let empty_name = Rc::new(String::from(""));
        vec![
        Self::func_println(empty_name.clone()),
        Self::func_print(empty_name.clone()),
        Self::func_to_int(empty_name.clone()),
        Self::func_to_num(empty_name.clone()),
        Self::func_format(empty_name.clone()),
        Self::func_input()
        ]
    }
    pub fn func_map() -> HashMap<String, (u16, IsBuiltIn, IsPub, u16)> {
        let mut map = HashMap::new();
        map.insert(String::from("println"), (0, true, true, 0));
        map.insert(String::from("print"  ), (0, true, true, 0));
        map.insert(String::from("input"  ), (0, true, true, 0));
        map.insert(String::from("to_int" ), (0, true, true, 0));
        map.insert(String::from("to_num" ), (0, true, true, 0));
        map.insert(String::from("format" ), (0, true, true, 0));
        map
    }
}
