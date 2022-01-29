use std::any::Any;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use hashbrown::HashMap;
use crate::builtin::classes::BuiltinClass;
use crate::exec::result::GloomResult;
use crate::obj::func::{BuiltinFn, GloomFunc, Param, ReturnType};
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};

pub struct GloomString(pub RefCell<String>);

impl Debug for GloomString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"\"{}\"",self.0.borrow())
    }
}

impl Object for GloomString {
    fn obj_type(&self) -> ObjectType {
        ObjectType::String
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl GloomString {
    #[inline]
    pub fn new(str : String) -> GloomObjRef{
        GloomObjRef::new(Rc::new(
            GloomString(RefCell::new(str))
        ))
    }
}

pub fn gloom_string_class() -> BuiltinClass{
    let mut map = HashMap::new();
    let mut funcs = Vec::new();
    let empty_string = Rc::new(String::new());
    funcs.push(RefCount::new(GloomFunc::new_builtin_fn(
        Rc::new(String::from("append")),
        vec![
            Param::new(empty_string.clone(),DataType::Ref(RefType::String)),
            Param::new(empty_string.clone(),DataType::Ref(RefType::String))
        ],
        ReturnType::Void,
        true,
        Rc::new(|_, args| {
            let mut iter = args.vec.into_iter();
            let myself_ref = iter.next().unwrap().assert_into_ref();
            let myself = myself_ref.downcast::<GloomString>();
            let other_ref = iter.next().unwrap().assert_into_ref();
            let other = other_ref.downcast::<GloomString>();
            myself.0.borrow_mut().push_str(other.0.borrow().as_str());
            GloomResult::ReturnVoid
        })
    )));
    map.insert(String::from("append"),0);
    BuiltinClass{
        name: "String".to_string(),
        map,
        funcs,
        get_ref_type_fn: BuiltinClass::none_generic_fn(RefType::String)
    }
}