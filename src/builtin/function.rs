use hashbrown::HashMap;
use crate::builtin::classes::BuiltinClass;
use crate::obj::func::ReturnType;
use crate::obj::types::{DataType, RefType};

pub fn gloom_func_class() -> BuiltinClass{
    let mut map = HashMap::new();
    let mut funcs = Vec::new();

    BuiltinClass{
        name: "String".to_string(),
        map,
        funcs,
        get_ref_type_fn: Box::new(|option| {
            if let Some(mut generic) = option {
                if generic.len() == 0 {
                    Result::Ok(RefType::Func(Box::new((Vec::new(),ReturnType::Void))))
                }else if generic.len() == 1 {
                    let generic_type = generic.pop().unwrap();
                    let param_types = match generic_type {
                        DataType::Ref(RefType::Tuple(vec_box)) => {
                            *vec_box
                        }
                        param_type => vec![param_type]
                    };
                    Result::Ok(RefType::Func(Box::new((param_types,ReturnType::Void))))
                }else if generic.len() == 2 {
                    let generic_return_type = generic.pop().unwrap();
                    let generic_param_type = generic.pop().unwrap();
                    let param_types = match generic_param_type {
                        DataType::Ref(RefType::Tuple(vec_box)) => {
                            *vec_box
                        }
                        param_type => vec![param_type]
                    };
                    if generic_return_type.is_none() {
                        Result::Ok(RefType::Func(Box::new((param_types,ReturnType::Void))))
                    }else {
                        Result::Ok(RefType::Func(Box::new((param_types,ReturnType::Have(generic_return_type)))))
                    }
                }else{
                    Result::Err(format!("unexpect generic type {:?} of Func, expect 1 or 2 generic type parameter",generic))
                }
            }else{
                Result::Ok(RefType::Func(Box::new((Vec::new(),ReturnType::Void))))
            }
        })
    }
}