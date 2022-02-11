use std::rc::Rc;
use crate::obj::func::{GloomFunc, GloomFuncObj, Param, ReturnType};
use crate::obj::types::{DataType, RefType};
use crate::vm::value::Value;

pub fn gloom_func_from_ptr(func_ptr:*const u8) -> GloomFunc {
    GloomFunc::new_builtin_fn(
        Rc::new(String::from("printBody")),
        vec![Param::new(
            Rc::new(String::from("")),
            DataType::Ref(RefType::Func(Box::new((Vec::with_capacity(0),ReturnType::Void,true))))
        )],
        ReturnType::Void,
        true,
        Rc::new(|_ , mut args| {
            let obj_ref = args.vec.pop().unwrap().assert_into_ref();
            println!("{:?}", obj_ref.downcast::<GloomFuncObj>().func.inner().body);
            Value::None
        })
    )
}