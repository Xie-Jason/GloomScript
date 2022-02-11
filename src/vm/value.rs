use std::fmt::{Debug, Formatter};

use crate::builtin::boxed::{GloomBool, GloomChar, GloomInt, GloomNum};
use crate::obj::object::{GloomObjRef, ObjectType};

#[derive(Clone)]
pub enum Value{
    Int(i64),
    Num(f64),
    Char(char),
    Bool(bool),
    Ref(GloomObjRef),
    None
}

impl Debug for Value{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f,"{}",i),
            Value::Num(i) => write!(f,"{}",i),
            Value::Char(i) => write!(f,"'{}'",i),
            Value::Bool(i) => write!(f,"{}",i),
            Value::Ref(rf) => write!(f,"{:?}",rf),
            Value::None => write!(f,"none")
        }
    }
}

impl Value {
    #[inline(always)]
    pub fn as_int(&self) -> Option<i64>{
        match self {
            Value::Int(i) => Option::Some(*i),
            Value::Ref(obj) => {
                if let ObjectType::Int = obj.obj_type() {
                    Option::Some(obj.downcast::<GloomInt>().0.get())
                }else{
                    Option::None
                }
            }
            _ => Option::None
        }
    }

    #[inline(always)]
    pub fn assert_int(&self) -> i64{
        self.as_int().unwrap()
    }

    #[inline(always)]
    pub fn assert_int_include_num(&self) -> i64{
        match self.as_int() {
            None => match self.as_num() {
                None => panic!(),
                Some(i) => i as i64
            }
            Some(i) => i
        }
    }

    #[inline(always)]
    pub fn assert_int_form_num_liked(&self) -> i64{
        match self {
            Value::Int(int) => *int,
            Value::Num(num) => *num as i64,
            Value::Char(ch) => *ch as i64,
            Value::Ref(obj_ref) => {
                match obj_ref.obj_type() {
                    ObjectType::Int => obj_ref.downcast::<GloomInt>().0.get(),
                    ObjectType::Num => obj_ref.downcast::<GloomNum>().0.get() as i64,
                    ObjectType::Char => obj_ref.downcast::<GloomChar>().0.get() as i64,
                    _ => panic!("{:?} as i64 ?",self)
                }
            }
            _ => panic!("{:?} as i64 ?",self)
        }
    }

    #[inline(always)]
    pub fn as_num(&self) -> Option<f64>{
        match self {
            Value::Num(i) => Option::Some(*i),
            Value::Ref(obj) => {
                if let ObjectType::Num = obj.obj_type() {
                    Option::Some(obj.downcast::<GloomNum>().0.get())
                }else{
                    Option::None
                }
            }
            _ => Option::None
        }
    }
    #[inline(always)]
    pub fn assert_num(&self) -> f64{
        self.as_num().unwrap()
    }

    #[inline(always)]
    pub fn assert_num_include_int(&self) -> f64{
        match self.as_num() {
            None => match self.as_int() {
                None => panic!(),
                Some(i) => i as f64
            }
            Some(i) => i
        }
    }

    #[inline(always)]
    pub fn as_char(&self) -> Option<char>{
        match self {
            Value::Char(i) => Option::Some(*i),
            Value::Ref(obj) => {
                if let ObjectType::Char = obj.obj_type() {
                    Option::Some(obj.downcast::<GloomChar>().0.get())
                }else{
                    Option::None
                }
            }
            _ => Option::None
        }
    }
    #[inline(always)]
    pub fn assert_char(&self) -> char{
        self.as_char().unwrap()
    }

    #[inline(always)]
    pub fn assert_char_include_int(&self) -> char{
        match self {
            Value::Char(i) => *i,
            Value::Int(i) => *i as u8 as char,
            Value::Ref(obj) => {
                match obj.obj_type() {
                    ObjectType::Int => obj.downcast::<GloomInt>().0.get() as u8 as char,
                    ObjectType::Char => obj.downcast::<GloomChar>().0.get(),
                    _ => panic!("{:?} as char ?",self)
                }
            }
            _ => panic!("{:?} as char ?",self)
        }
    }

    #[inline(always)]
    pub fn as_bool(&self) -> Option<bool>{
        match self {
            Value::Bool(i) => Option::Some(*i),
            Value::Ref(obj) => {
                if let ObjectType::Bool = obj.obj_type() {
                    Option::Some(obj.downcast::<GloomBool>().0.get())
                }else{
                    Option::None
                }
            }
            _ => Option::None
        }
    }

    #[inline(always)]
    pub fn assert_bool(&self) -> bool{
        self.as_bool().unwrap()
    }

    #[inline]
    pub fn as_ref(&self) -> &GloomObjRef{
        if let Value::Ref(rf) = self {
            rf
        }else{
            panic!()
        }
    }

    #[inline(always)]
    pub fn into_ref(self) -> Option<GloomObjRef>{
        match self {
            Value::Int(i) => Option::Some(GloomInt::new(i)),
            Value::Num(i) => Option::Some(GloomNum::new(i)),
            Value::Char(i) => Option::Some(GloomChar::new(i)),
            Value::Bool(i) => Option::Some(GloomBool::new(i)),
            Value::Ref(obj) => Option::Some(obj),
            Value::None => Option::None
        }
    }
    #[inline(always)]
    pub fn assert_into_ref(self) -> GloomObjRef{
        self.into_ref().unwrap()
    }

    #[inline(always)]
    pub fn is_none(&self) -> bool {
        match self {
            Value::None => true,
            _ => false
        }
    }

    #[inline]
    pub fn not(&mut self){
        match self {
            Value::Bool(bl) => {
                *bl = ! *bl;
            }
            Value::Ref(rf) => {
                if let ObjectType::Bool = rf.obj_type() {
                    let bl = rf.downcast::<GloomBool>();
                    bl.0.set(! bl.0.get());
                }
            }
            _ => panic!()
        }
    }

    #[inline]
    pub fn neg(&mut self){
        match self {
            Value::Int(i) => {
                *i = - *i;
            }
            Value::Num(n) => {
                *n = - *n;
            }
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => {
                        let i = rf.downcast::<GloomInt>();
                        i.0.set(i.0.get());
                    }
                    ObjectType::Num => {
                        let n = rf.downcast::<GloomNum>();
                        n.0.set(n.0.get());
                    }
                    _ => panic!()
                }
            }
            _ => panic!()
        }
    }

    #[inline]
    pub fn plus(&mut self,val : Value){
        match self {
            Value::Int(int) => {
                *int += val.assert_int_include_num();
            }
            Value::Num(num) => {
                *num += val.assert_num_include_int();
            }
            Value::Ref(obj_ref) => {
                match obj_ref.obj_type() {
                    ObjectType::Int => {
                        let int_obj = obj_ref.downcast::<GloomInt>();
                        let int_val = int_obj.0.get() + val.assert_int();
                        int_obj.0.set(int_val);
                    }
                    ObjectType::Num => {
                        let obj = obj_ref.downcast::<GloomNum>();
                        let num_val = obj.0.get() + val.assert_num();
                        obj.0.set(num_val);
                    }
                    _ => panic!()
                }
            }
            _ => panic!()
        }
    }
    #[inline]
    pub fn sub(&mut self,val : Value){
        match self {
            Value::Int(int) => {
                *int -= val.assert_int_include_num();
            }
            Value::Num(num) => {
                *num -= val.assert_num_include_int();
            }
            Value::Ref(obj_ref) => {
                match obj_ref.obj_type() {
                    ObjectType::Int => {
                        let int_obj = obj_ref.downcast::<GloomInt>();
                        let int_val = int_obj.0.get() - val.assert_int();
                        int_obj.0.set(int_val);
                    }
                    ObjectType::Num => {
                        let obj = obj_ref.downcast::<GloomNum>();
                        let num_val = obj.0.get() - val.assert_num();
                        obj.0.set(num_val);
                    }
                    _ => panic!()
                }
            }
            _ => panic!()
        }
    }
    #[inline]
    pub fn plus_one(&mut self){
        match self {
            Value::Int(int) => {
                *int += 1;
            }
            Value::Num(num) => {
                *num += 1.0;
            }
            Value::Ref(obj_ref) => {
                match obj_ref.obj_type() {
                    ObjectType::Int => {
                        let int_obj = obj_ref.downcast::<GloomInt>();
                        let int_val = int_obj.0.get() + 1;
                        int_obj.0.set(int_val);
                    }
                    ObjectType::Num => {
                        let obj = obj_ref.downcast::<GloomNum>();
                        let num_val = obj.0.get() + 1.0;
                        obj.0.set(num_val);
                    }
                    _ => panic!()
                }
            }
            _ => panic!()
        }
    }
    #[inline]
    pub fn sub_one(&mut self){
        match self {
            Value::Int(int) => {
                *int -= 1;
            }
            Value::Num(num) => {
                *num -= 1.0;
            }
            Value::Ref(obj_ref) => {
                match obj_ref.obj_type() {
                    ObjectType::Int => {
                        let int_obj = obj_ref.downcast::<GloomInt>();
                        let int_val = int_obj.0.get() - 1;
                        int_obj.0.set(int_val);
                    }
                    ObjectType::Num => {
                        let obj = obj_ref.downcast::<GloomNum>();
                        let num_val = obj.0.get() - 1.0;
                        obj.0.set(num_val);
                    }
                    _ => panic!()
                }
            }
            _ => panic!()
        }
    }

    #[inline]
    pub fn equals(&self, other : Value) -> bool{
        match self {
            Value::Int(int) => *int == other.assert_int_include_num(),
            Value::Num(num) => *num == other.assert_num_include_int(),
            Value::Char(ch) => *ch == other.assert_char(),
            Value::Bool(bl) => *bl == other.assert_bool(),
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => rf.downcast::<GloomInt>().0.get() == other.assert_int_include_num(),
                    ObjectType::Num => rf.downcast::<GloomNum>().0.get() == other.assert_num_include_int(),
                    ObjectType::Char => rf.downcast::<GloomChar>().0.get() == other.assert_char(),
                    ObjectType::Bool => rf.downcast::<GloomBool>().0.get() == other.assert_bool(),
                    _ => rf.addr_eqs(&other.assert_into_ref())
                }
            }
            Value::None => false
        }
    }

    #[inline]
    pub fn greater_than(&self, other : Value) -> bool{
        match self {
            Value::Int(int) => *int > other.assert_int_include_num(),
            Value::Num(num) => *num > other.assert_num_include_int(),
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => rf.downcast::<GloomInt>().0.get() > other.assert_int_include_num(),
                    ObjectType::Num => rf.downcast::<GloomNum>().0.get() > other.assert_num_include_int(),
                    _ => panic!("{:?} > {:?} ?",self,other)
                }
            }
            _ => panic!("{:?} > {:?} ?",self,other)
        }
    }

    #[inline]
    pub fn less_than(&self, other : Value) -> bool{
        match self {
            Value::Int(int) => *int < other.assert_int_include_num(),
            Value::Num(num) => *num < other.assert_num_include_int(),
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => rf.downcast::<GloomInt>().0.get() < other.assert_int_include_num(),
                    ObjectType::Num => rf.downcast::<GloomNum>().0.get() < other.assert_num_include_int(),
                    _ => panic!("{:?} < {:?} ?",self,other)
                }
            }
            _ => panic!("{:?} < {:?} ?",self,other)
        }
    }

    #[inline]
    pub fn greater_equal(&self, other : Value) -> bool{
        match self {
            Value::Int(int) => *int >= other.assert_int_include_num(),
            Value::Num(num) => *num >= other.assert_num_include_int(),
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => rf.downcast::<GloomInt>().0.get() >= other.assert_int_include_num(),
                    ObjectType::Num => rf.downcast::<GloomNum>().0.get() >= other.assert_num_include_int(),
                    _ => panic!("{:?} >= {:?} ?",self,other)
                }
            }
            _ => panic!("{:?} >= {:?} ?",self,other)
        }
    }

    #[inline]
    pub fn less_equal(&self, other : Value) -> bool{
        match self {
            Value::Int(int) => *int <= other.assert_int_include_num(),
            Value::Num(num) => *num <= other.assert_num_include_int(),
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => rf.downcast::<GloomInt>().0.get() <= other.assert_int_include_num(),
                    ObjectType::Num => rf.downcast::<GloomNum>().0.get() <= other.assert_num_include_int(),
                    _ => panic!("{:?} <= {:?} ?",self,other)
                }
            }
            _ => panic!("{:?} <= {:?} ?",self,other)
        }
    }

    #[inline]
    pub fn multiply(&mut self, other : Value){
        match self {
            Value::Int(int) => *int = *int * other.assert_int_include_num(),
            Value::Num(num) => *num = *num * other.assert_num_include_int(),
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => {
                        let int = rf.downcast::<GloomInt>();
                        int.0.set(int.0.get() * other.assert_int_include_num());
                    }
                    ObjectType::Num => {
                        let num = rf.downcast::<GloomNum>();
                        num.0.set(num.0.get() * other.assert_num_include_int());
                    }
                    _ => panic!("{:?} * {:?} ?",self,other)
                }
            }
            _ => panic!("{:?} * {:?} ?",self,other)
        }
    }

    #[inline]
    pub fn divide(&mut self, other : Value){
        match self {
            Value::Int(int) => *int = *int / other.assert_int_include_num(),
            Value::Num(num) => *num = *num / other.assert_num_include_int(),
            Value::Ref(rf) => {
                match rf.obj_type() {
                    ObjectType::Int => {
                        let int = rf.downcast::<GloomInt>();
                        int.0.set(int.0.get() / other.assert_int_include_num());
                    }
                    ObjectType::Num => {
                        let num = rf.downcast::<GloomNum>();
                        num.0.set(num.0.get() / other.assert_num_include_int());
                    }
                    _ => panic!("{:?} / {:?} ?",self,other)
                }
            }
            _ => panic!("{:?} / {:?} ?",self,other)
        }
    }
}

#[derive(Debug)]
pub struct GloomArgs{
    pub vec : Vec<Value>
}

impl GloomArgs {
    #[inline(always)]
    pub fn new(vec : Vec<Value>) -> GloomArgs {
        GloomArgs{
            vec
        }
    }
    #[inline(always)]
    pub fn empty() -> GloomArgs {
        GloomArgs{
            vec : Vec::with_capacity(0)
        }
    }
}