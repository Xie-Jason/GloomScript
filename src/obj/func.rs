use std::any::Any;
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use crate::exec::executor::Executor;
use crate::exec::result::GloomResult;
use crate::exec::value::{GloomArgs, Value};
use crate::frontend::ast::Statement;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{BasicType, DataType, RefType};

pub struct GloomFuncObj{
    pub func : RefCount<GloomFunc>,
    pub captures : RefCell<Vec<Value>>
}

impl Object for GloomFuncObj {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Func
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_exec(&self, exec: &Executor, _ : &GloomObjRef) {
        for value in self.captures.borrow().iter() {
            if let Value::Ref(rf) = value {
                exec.drop_object(rf);
            }
        }
    }

    fn at(&self, _ : &mut usize) -> Option<Value> {
        panic!()
    }
}

impl GloomFuncObj {
    #[inline]
    pub fn new_closure(func : RefCount<GloomFunc>,captures : Vec<Value>) -> GloomObjRef{
        GloomObjRef::new(
        Rc::new(GloomFuncObj{
            func,
            captures : RefCell::new(captures)
        }))
    }
    #[inline]
    pub fn new_func(func : RefCount<GloomFunc>) -> GloomObjRef{
        GloomObjRef::new(
        Rc::new(GloomFuncObj{
            func,
            captures : RefCell::new(Vec::with_capacity(0))
        }))
    }
}

impl Debug for GloomFuncObj {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?} {:?}",self.func,self.captures.borrow())
    }
}

pub struct GloomFunc {
    pub info : FuncInfo,
    pub body : FuncBody
}

pub type BuiltinFn = Rc<dyn Fn(&Executor,GloomArgs) -> GloomResult>;

impl GloomFunc {
    pub fn new(name : Rc<String>, file_index : u16, params : Vec<Param>, return_type : ReturnType, statements : Vec<Statement>) -> GloomFunc{
        GloomFunc{
            info : FuncInfo{
                name,
                params,
                return_type,
                captures: Vec::with_capacity(0),
                drop_slots: Vec::with_capacity(0),
                local_size: 0,
                need_self: false,
                file_index
            },
            body: FuncBody::Gloom(statements)
        }
    }
    pub fn new_builtin_fn(name : Rc<String>, params : Vec<Param>, return_type : ReturnType, need_self : bool, func : BuiltinFn ) -> GloomFunc {
        GloomFunc{
            info : FuncInfo{
                name,
                params,
                return_type,
                captures: Vec::with_capacity(0),
                drop_slots: Vec::with_capacity(0),
                local_size: 0,
                need_self,
                file_index: 0
            },
            body: FuncBody::Builtin(func)
        }
    }
    #[inline]
    pub fn handle_instance_func(&mut self, class : &DataType){
        let len = self.info.params.len();
        if len >= 1 {
            let param = self.info.params.get_mut(0).unwrap();
            let data_type = &mut param.data_type;
            if let DataType::Ref(RefType::MySelf) = data_type {
                *data_type = class.clone();
                self.info.need_self = true;
            }
        }
    }
    #[inline]
    pub fn get_type(&self) -> DataType{
        let mut param_types = Vec::new();
        for param in self.info.params.iter() {
            param_types.push(param.data_type.clone());
        }
        DataType::Ref(RefType::Func(Box::new((param_types,self.info.return_type.clone(),false))))
    }
    #[inline]
    pub fn get_ref_type(&self) -> RefType{
        let mut param_types = Vec::new();
        for param in self.info.params.iter() {
            param_types.push(param.data_type.clone());
        }
        RefType::Func(Box::new((param_types,self.info.return_type.clone(),false)))
    }
    #[inline]
    pub fn have_capture(&self) -> bool{
        self.info.captures.len() > 0
    }
}

impl Debug for GloomFunc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Func {} {:?}->{:?} {:?}",self.info.name,self.info.params,self.info.return_type,self.info.captures)
    }
}


#[derive(Debug,Clone)]
pub struct FuncInfo{
    pub name : Rc<String>,
    pub params : Vec<Param>,
    pub return_type : ReturnType,
    pub captures : Vec<Capture>,
    pub drop_slots : Vec<u16>,
    pub local_size : u16,
    pub need_self : bool,
    pub file_index : u16,
}

#[derive(Clone)]
pub struct Param{
    pub name : Rc<String>,
    pub data_type : DataType,
    pub index : (u16,u8)
}

impl Debug for Param {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{} {} {:?}",self.data_type,self.name,self.index)
    }
}

impl Param {
    pub fn new(name: Rc<String>, data_type: DataType) -> Self {
        Param {
            name,
            data_type,
            index: (0,0)
        }
    }
}

#[derive(PartialEq,Clone)]
pub enum ReturnType{
    Void,
    Have(DataType)
}

impl ReturnType {
    #[inline]
    pub fn is_void(&self) -> bool {
        match self {
            ReturnType::Void => true,
            ReturnType::Have(_) => false
        }
    }
    #[inline]
    pub fn data_type(&self) -> &DataType{
        match self {
            ReturnType::Void => &DataType::Ref(RefType::None),
            ReturnType::Have(tp) => tp
        }
    }
}

impl Debug for ReturnType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReturnType::Void => write!(f,"void"),
            ReturnType::Have(data_type) => {
                write!(f,"{}",data_type)
            }
        }
    }
}

impl Display for ReturnType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self)
    }
}

impl PartialEq<Option<DataType>> for ReturnType  {
    fn eq(&self, other: &Option<DataType>) -> bool {
        match self {
            ReturnType::Void => {
                other.is_none()
            }
            ReturnType::Have(data_type) => {
                if other.is_none() {
                    false
                }else{
                    data_type.eq(other.as_ref().unwrap())
                }
            }
        }
    }
}

pub enum FuncBody{
    Builtin(BuiltinFn),
    Gloom(Vec<Statement>),
    None,
}

impl Debug for FuncBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FuncBody::Builtin(_) => {
                write!(f,"BuiltinFunc")
            }
            FuncBody::Gloom(vec) => {
                write!(f,"{:?}",vec)
            }
            FuncBody::None => {
                write!(f,"None")
            }
        }
    }
}

#[derive(Clone)]
pub struct Capture {
    pub from_slot_idx : u16,
    pub from_sub_idx  : u8 ,
    pub to_slot_idx   : u16,
    pub to_sub_idx    : u8,
    pub basic_type    : BasicType
}

impl Capture {
    pub fn new(from_slot_idx: u16, from_sub_idx: u8, to_slot_idx: u16, to_sub_idx: u8, basic_type : BasicType) -> Self {
        Capture {
            from_slot_idx,
            from_sub_idx,
            to_slot_idx,
            to_sub_idx,
            basic_type
        }
    }
}

impl Debug for Capture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"({},{})->({},{})<{:?}>",self.from_slot_idx,self.from_sub_idx,self.to_slot_idx,self.to_sub_idx,self.basic_type)
    }
}