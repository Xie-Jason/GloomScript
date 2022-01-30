use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use crate::obj::func::ReturnType;
use crate::obj::gloom_class::GloomClass;
use crate::obj::gloom_enum::GloomEnumClass;
use crate::obj::refcount::RefCount;
use crate::obj::interface::Interface;


// 16bytes   rustc did optimization here
#[derive(Clone,PartialEq)]
pub enum DataType{
    Int,
    Num,
    Char,
    Bool,
    Ref(RefType)
}


impl DataType {
    pub fn belong_to(&self,other : &DataType) -> bool{
        if self.eq(other)
            || other.as_ref_type().eq(&RefType::Any) ||
            ( self.is_int_or_num() && other.is_int_or_num() ) {
            true
        }else {
            if let DataType::Ref(self_type) = self {
                if let DataType::Ref(other_type) = other {
                    self_type.belong_to(other_type)
                }else{
                    false
                }
            }else{
                false
            }
        }
    }

    #[inline]
    pub fn equal_interface(&self, interface : &RefCount<Interface>) -> bool {
        match self {
            DataType::Ref(RefType::Interface(myself)) => myself.eq(interface),
            _ => false,
        }
    }

    #[inline]
    pub fn is_ref_type(&self) -> bool{
        match self {
            DataType::Ref(_) => true,
            _ => false
        }
    }
    #[inline]
    pub fn is_num_liked(&self) -> bool {
        match self {
            DataType::Int => true,
            DataType::Num => true,
            DataType::Char => true,
            DataType::Ref(RefType::Int) => true,
            DataType::Ref(RefType::Num) => true,
            DataType::Ref(RefType::Char) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_basic(&self) -> bool{
        match self {
            DataType::Int => true,
            DataType::Num => true,
            DataType::Char => true,
            DataType::Bool => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_basic_or_box(&self) -> bool{
        match self {
            DataType::Int => true,
            DataType::Num => true,
            DataType::Char => true,
            DataType::Bool => true,
            DataType::Ref(RefType::Int) => true,
            DataType::Ref(RefType::Num) => true,
            DataType::Ref(RefType::Char) => true,
            DataType::Ref(RefType::Bool) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_int_or_num(&self) -> bool{
        match self {
            DataType::Int => true,
            DataType::Num => true,
            DataType::Ref(RefType::Int) => true,
            DataType::Ref(RefType::Num) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_num(&self) -> bool{
        match self {
            DataType::Num => true,
            DataType::Ref(RefType::Num) => true,
            _ => false
        }
    }
    #[inline]
    pub fn is_int(&self) -> bool{
        match self {
            DataType::Int => true,
            DataType::Ref(RefType::Int) => true,
            _ => false
        }
    }
    #[inline]
    pub fn is_bool(&self) -> bool {
        match self {
            DataType::Bool => true,
            DataType::Ref(RefType::Bool) => true,
            _ => false
        }
    }
    #[inline]
    pub fn is_func(&self) -> bool {
        match self {
            DataType::Ref(RefType::Func(_)) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn as_ref_type(&self) -> RefType{
        match self {
            DataType::Int => RefType::Int,
            DataType::Num => RefType::Num,
            DataType::Char => RefType::Char,
            DataType::Bool => RefType::Bool,
            DataType::Ref(ref_type) => ref_type.clone()
        }
    }
    #[inline]
    pub fn is_none(&self) -> bool{
        match self {
            DataType::Ref(RefType::None) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn as_basic(&self) -> BasicType{
        match self {
            DataType::Int => BasicType::Int,
            DataType::Num => BasicType::Num,
            DataType::Char => BasicType::Char,
            DataType::Bool => BasicType::Bool,
            DataType::Ref(_) => BasicType::Ref
        }
    }
    #[inline]
    pub fn is_queue(&self) -> bool{
        if let DataType::Ref(RefType::Queue(_)) = self {
            true
        }else{
            false
        }
    }
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s : String;
        // 用于保证format!返回的String对象不会被立即释放
        // to sure that the returned String obj won't be drop immediately
        write!(f,"{}",match self {
            DataType::Int => "int",
            DataType::Num => "num",
            DataType::Char => "char",
            DataType::Bool => "bool",
            DataType::Ref(ref_type) => {
                s = format!("{}", ref_type);
                s.as_str()
            }
        })
    }
}

impl Debug for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self)
    }
}

// 16byte
#[derive(Clone,Debug,PartialEq)]
pub enum RefType{
    Any,
    None,
    MySelf,

    // instance of type
    Class(RefCount<GloomClass>),
    Enum(RefCount<GloomEnumClass>),
    Interface(RefCount<Interface>),

    // not instance but type itself
    MetaClass(RefCount<GloomClass>),
    MetaEnum(RefCount<GloomEnumClass>),
    MetaInterface(RefCount<Interface>),
    MataBuiltinType(BuiltinType),

    Tuple(Box<Vec<DataType>>),
    Func(Box<(Vec<DataType>,ReturnType,bool)>),
    Weak(Box<DataType>),
    Array(Box<DataType>),
    Queue(Box<DataType>),

    Int,
    Num,
    Char,
    Bool,
    String,
}

impl RefType {
    pub fn belong_to(&self, other : &RefType) -> bool {
        if let RefType::Any = other {
            return true;
        }
        match self {
            RefType::Class(cls) => {
                match other {
                    RefType::Class(class) => cls.eq(class) || cls.inner().is_derived_from(class),
                    RefType::Interface(interface) => cls.inner().is_impl_from(interface),
                    _ => false
                }
            }
            RefType::Interface(inter) => {
                match other {
                    RefType::Class(class) => class.inner().is_impl_from(inter),
                    RefType::Interface(interface) => inter.eq(interface)
                        || inter.inner().derived_from(interface),
                    _ => false
                }
            }
            RefType::Func(func_type) => {
                if let RefType::Func(other_func_type) = other {
                    let func_type_borrow = func_type.deref();
                    let (vec1,ret_type1,_) = func_type_borrow.deref();
                    let other_fn_type_borrow = other_func_type.deref();
                    let (vec2,ret_type2,all_ok) = other_fn_type_borrow.deref();
                    if *all_ok {
                        return true
                    }
                    vec1.eq(vec2) && ret_type1.eq(ret_type2)
                }else {
                    false
                }
            }
            ref_type => {
                ref_type.eq(other)
            }
        }
    }
    #[inline]
    pub fn as_built_type(&self) -> BuiltinType{
        match self {
            RefType::Func(_) => BuiltinType::Func,
            RefType::Weak(_) => BuiltinType::Weak,
            RefType::Array(_) => BuiltinType::Array,
            RefType::Queue(_) => BuiltinType::Queue,
            RefType::Int => BuiltinType::Int,
            RefType::Num => BuiltinType::Num,
            RefType::Char => BuiltinType::Char,
            RefType::Bool => BuiltinType::Bool,
            RefType::String => BuiltinType::String,
            _ => panic!()
        }
    }
}

impl Display for RefType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",match self {
            RefType::Class(cls) => format!("{}", cls.inner()),
            RefType::Enum(cls) =>  format!("{}", cls.inner()),
            RefType::Interface(inter) =>  format!("{}", inter.inner()),
            RefType::Tuple(vec) => format!("{:?}",vec),
            RefType::Func(func) => format!("Func<{:?}>",func),
            RefType::Weak(generic) => format!("Weak<{:?}>",generic),
            RefType::Array(generic) => format!("Array<{:?}>",generic),
            RefType::Queue(generic) => format!("Queue<{:?}>",generic),
            ref_type => format!("{:?}",ref_type),
        })
    }
}

#[derive(Copy,Clone,Debug,Hash,Eq,PartialEq)]
pub enum BuiltinType{
    Int,
    Num,
    Char,
    Bool,
    String,
    Func,
    Weak,
    Array,
    Queue,
}

impl BuiltinType {
    pub fn try_from_str(name : &str) -> Option<BuiltinType>{
        let builtin_type = match name {
            "int" => BuiltinType::Int,
            "Int" => BuiltinType::Int,
            "num" => BuiltinType::Num,
            "Num" => BuiltinType::Num,
            "char" => BuiltinType::Char,
            "Char" => BuiltinType::Char,
            "bool" => BuiltinType::Bool,
            "Bool" => BuiltinType::Bool,
            "String" => BuiltinType::String,
            "Array" => BuiltinType::Array,
            "Queue" => BuiltinType::Queue,
            "Func" => BuiltinType::Func,
            "Weak" => BuiltinType::Weak,
            _ => return Option::None
        };
        Option::Some(builtin_type)
    }
    pub fn to_str(&self) -> &str{
        match self {
            BuiltinType::Int => "Int",
            BuiltinType::Num => "Num",
            BuiltinType::Char => "Char",
            BuiltinType::Bool => "Bool",
            BuiltinType::String => "String",
            BuiltinType::Func => "Func",
            BuiltinType::Weak => "Weak",
            BuiltinType::Array => "Array",
            BuiltinType::Queue => "Queue",
        }
    }
}

#[derive(Debug,Clone)]
pub enum DeclaredType{
    Class(RefCount<GloomClass>),
    Enum(RefCount<GloomEnumClass>),
    Interface(RefCount<Interface>),
    IsNot
}

impl DeclaredType {
    pub fn equal_class(&self, class : &RefCount<GloomClass>) -> bool{
        match self {
            DeclaredType::Class(myself) => myself.eq(class),
            _ => false
        }
    }
    pub fn equal_enum(&self, class : &RefCount<GloomEnumClass>) -> bool{
        match self {
            DeclaredType::Enum(myself) => myself.eq(class),
            _ => false
        }
    }
    pub fn equal_interface(&self, inter : &RefCount<Interface>) -> bool{
        match self {
            DeclaredType::Interface(myself) => myself.eq(inter),
            _ => false
        }
    }
}

#[derive(Debug,Clone)]
pub enum BreakType{
    Type(DataType),
    Uninit,
    Void
}

impl BreakType {
    pub fn is_void(&self) -> bool {
        match self {
            BreakType::Void => true,
            _ => false
        }
    }
}

impl Display for BreakType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakType::Type(dt) => write!(f, "{}", dt),
            BreakType::Uninit => write!(f, "uninit"),
            BreakType::Void => write!(f, "void"),
        }
    }
}

#[derive(Copy,Clone)]
pub enum BasicType{
    Int,
    Num,
    Char,
    Bool,
    Ref
}

impl Debug for BasicType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",match self {
            BasicType::Int => "int",
            BasicType::Num => "num",
            BasicType::Char => "char",
            BasicType::Bool => "bool",
            BasicType::Ref => "Ref"
        })
    }
}

impl BasicType {
    #[inline(always)]
    pub fn is_ref(&self) -> bool{
        if let BasicType::Ref = self {
            true
        }else {
            false
        }
    }
}