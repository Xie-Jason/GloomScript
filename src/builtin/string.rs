use std::any::Any;
use std::cell::RefCell;
use core::fmt::{Debug, Formatter};
extern crate alloc;
use alloc::rc::Rc;

use hashbrown::HashMap;

use crate::builtin::classes::BuiltinClass;
use crate::builtin::iter::GloomListIter;
use crate::frontend::status::GloomStatus;
use crate::obj::func::{GloomFunc, Param, ReturnType};
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;

pub struct GloomString(pub RefCell<String>);

impl Debug for GloomString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0.borrow())
    }
}

impl Object for GloomString {
    fn obj_type(&self) -> ObjectType {
        ObjectType::String
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_vm(&self, _: &GloomVM, _: &GloomObjRef) {}

    fn iter(&self, rf: &GloomObjRef) -> GloomObjRef {
        GloomListIter::new(rf.clone())
    }

    fn at(&self, index: &mut usize) -> Option<Value> {
        let string = self.0.borrow();
        if *index < string.len() {
            // find bytes read limit
            let remain = string.len() - *index;
            // replace the if
            let limit = [remain, 4][(remain >= 4) as usize];

            let mut bytes = [0; 4];
            let ptr = string.as_ptr();
            for idx in 0..limit {
                unsafe {
                    bytes[idx] = ptr.add(*index + idx).read();
                }
            }
            let (step, ch) = GloomString::decode_utf8(bytes);
            *index += step;
            Option::Some(Value::Char(ch))
        } else {
            Option::None
        }
    }

    fn next(&self) -> Value {
        panic!()
    }

    fn method(&self, index: u16, status: &GloomStatus) -> RefCount<GloomFunc> {
        status
            .builtin_classes
            .get(BuiltinClass::STRING_INDEX)
            .unwrap()
            .inner()
            .funcs
            .get(index as usize)
            .unwrap()
            .clone()
    }

    fn field(&self, _: u16, _: u8) -> Value {
        panic!()
    }
}

impl GloomString {
    #[inline]
    pub fn new(str: String) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomString(RefCell::new(str))))
    }

    // IMPORTANT
    // 下面几个常量魔数和函数decode_utf8()的一部分来自 `core::str::validation`
    // The const magic numbers and part of fn decode_utf8() are copied from `core::str::validation`

    // 连续字节的掩码
    // Mask of the value bits of a continuation byte.
    const CONT_MASK: u8 = 0b0011_1111;

    #[inline]
    pub fn decode_utf8(bytes: [u8; 4]) -> (usize, char) {
        let x = bytes[0];
        if x < 128 {
            return (1, x as char);
        }
        let mut step;
        let y = bytes[1];
        let init = (x & (0x7F >> 2)) as u32;
        let mut ch = (init << 6) | (y & GloomString::CONT_MASK) as u32;
        step = 2;
        if x >= 0xE0 {
            // [[x y z] w] case
            // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
            let z = bytes[2];
            let y_z =
                (((y & GloomString::CONT_MASK) as u32) << 6) | (z & GloomString::CONT_MASK) as u32;
            ch = init << 12 | y_z;
            step = 3;
            if x >= 0xF0 {
                // [x y z w] case
                // use only the lower 3 bits of `init`
                let w = bytes[3];
                ch = (init & 7) << 18 | (y_z << 6) | (w & GloomString::CONT_MASK) as u32;
                step = 4;
            }
        }
        (step, char::from_u32(ch).unwrap())
    }
}

impl BuiltinClass {
    pub fn gloom_string_class() -> BuiltinClass {
        let mut map = HashMap::new();
        let mut funcs = Vec::new();
        let empty_string = Rc::new(String::new());
        funcs.push(RefCount::new(GloomFunc::new_builtin_fn(
            Rc::new(String::from("append")),
            vec![
                Param::new(empty_string.clone(), DataType::Ref(RefType::String)),
                Param::new(empty_string.clone(), DataType::Ref(RefType::String)),
            ],
            ReturnType::Have(DataType::Ref(RefType::String)),
            true,
            Rc::new(|_, args| {
                let mut iter = args.vec.into_iter();
                let myself = iter.next().unwrap().assert_into_ref();
                let mut string = myself.downcast::<GloomString>().0.borrow().clone();
                let other_ref = iter.next().unwrap().assert_into_ref();
                let other = other_ref.downcast::<GloomString>();
                string.push_str(other.0.borrow().as_str());
                Value::Ref(GloomString::new(string))
            }),
        )));
        map.insert(String::from("append"), 0);
        BuiltinClass {
            name: "String".to_string(),
            map,
            funcs,
            get_ref_type_fn: BuiltinClass::none_generic_fn(RefType::String),
        }
    }
}
