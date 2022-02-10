use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use hashbrown::HashMap;
use crate::vm::value::Value;
use crate::frontend::ast::{Statement};
use crate::obj::func::{FuncBody, FuncInfo, GloomFunc, Param, ReturnType};
use crate::obj::gloom_class::IsPub;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};
use crate::vm::machine::GloomVM;

// 枚举关联类型只支持类  enum could only related to class type
pub struct GloomEnum{
    pub tag  : Cell<u16>,
    pub val  : RefCell<Value>,
    pub class : RefCount<GloomEnumClass>
}

#[derive(Debug)]
pub struct GloomEnumClass{
    pub name : Rc<String>,
    pub types : Vec<RelatedType>,
    pub enum_map : HashMap<String,u16>,
    pub func_map : HashMap<String,(u16,IsPub)>,
    pub funcs : Vec<RefCount<GloomFunc>>,
    pub file_index : u16
}

pub enum RelatedType{
    None,
    Have(DataType)
}

impl Debug for RelatedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RelatedType::None => write!(f, "none"),
            RelatedType::Have(data_type) => write!(f,"{}",data_type)
        }
    }
}

impl Object for GloomEnum {
    fn obj_type(&self) -> ObjectType {
        ObjectType::Enum
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn drop_by_vm(&self, vm: &GloomVM, _ : &GloomObjRef) {
        if let Value::Ref(rf) = &*self.val.borrow() {
            vm.drop_object(rf);
        }
    }

    fn iter(&self, _ : &GloomObjRef) -> GloomObjRef {
        panic!()
    }

    fn at(&self, _ : &mut usize) -> Option<Value> {
        panic!()
    }

    fn next(&self) -> Option<Value> {
        panic!()
    }
}

impl GloomEnumClass {
    pub fn new(name : Rc<String>, file_index : u16) -> GloomEnumClass {
        GloomEnumClass{
            name,
            types: Vec::new(),
            enum_map: HashMap::new(),
            func_map: HashMap::new(),
            funcs: Vec::new(),
            file_index
        }
    }
    pub fn add_enum_value(&mut self,name : String, related_type : Option<DataType>){
        let index = self.types.len();
        self.enum_map.insert(name,index as u16);
        self.types.push(match related_type {
            None => RelatedType::None,
            Some(data_type) => RelatedType::Have(data_type)
        });
    }
    pub fn add_func(&mut self,
                    func_name : Rc<String>,
                    is_pub: IsPub,
                    params: Vec<Param>,
                    return_type: ReturnType,
                    body: Vec<Statement>){
        let index = self.funcs.len();
        self.func_map.insert(func_name.deref().clone(),(index as u16,is_pub));
        self.funcs.push(RefCount::new(GloomFunc{
            info : FuncInfo{
                name: func_name,
                params,
                return_type,
                captures: Vec::with_capacity(0),
                drop_slots: Vec::with_capacity(0),
                local_size: 0,
                need_self: false,
                file_index: self.file_index,
                stack_size: 0
            },
            body: FuncBody::AST(body)
        }));
    }
    pub fn handle_instance_func(&mut self, myself : RefCount<GloomEnumClass>){
        let data_type = DataType::Ref(RefType::Enum(myself));
        for func in self.funcs.iter_mut() {
            func.inner_mut().handle_instance_func(&data_type);
        }
    }
}

impl Debug for GloomEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Object of {}",self.class.inner().name)
    }
}

impl Display for GloomEnumClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.name)
    }
}