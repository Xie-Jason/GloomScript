use std::any::Any;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::slice::Iter;
use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use crate::exec::executor::Executor;
use crate::exec::value::Value;
use crate::frontend::ast::{Statement};
use crate::frontend::index::SlotIndexer;
use crate::obj::func::{GloomFunc, Param, ReturnType};
use crate::obj::types::{DataType, RefType};
use crate::obj::refcount::RefCount;
use crate::obj::interface::Interface;
use crate::obj::object::{GloomObjRef, Object, ObjectType};

pub struct GloomClass{
    pub name: Rc<String>,
    parent : Option<RefCount<GloomClass>>,
    impls : Vec<RefCount<Interface>>,
    pub map: HashMap<String,(u16,u8,IsPub,IsMemFunc)>,
    pub field_indexer : SlotIndexer,
    pub funcs : Vec<RefCount<GloomFunc>>,
    pub file_index : u16,
    pub field_count : u16,
    pub fn_drop_idx : u16,
}
pub type IsMemFunc = bool;
pub type IsPub = bool;

impl GloomClass {
    pub fn new(class_name : Rc<String>, file_index : u16) -> GloomClass{
        GloomClass{
            name: class_name,
            parent: Option::None,
            impls: Vec::with_capacity(0),
            map: HashMap::new(),
            funcs: Vec::new(),
            file_index,
            field_indexer: SlotIndexer::new(),
            fn_drop_idx: u16::MAX,
            field_count: 0
        }
    }

    // 如有父类，需要被首先调用 if this class have parent class, this function need to be called first
    pub fn set_parent(&mut self, parent : RefCount<GloomClass>){
        let parent_ref = parent.inner();
        self.impls = parent_ref.impls.clone();
        self.map = parent_ref.map.clone();
        self.funcs = parent_ref.funcs.clone();
        self.field_indexer = parent_ref.field_indexer.clone();
        std::mem::drop(parent_ref);
        self.parent = Some(parent);
    }

    // 最后再调用，因为会检查接口抽象方法是否实现 last to call this function,
    // because this function will check the abstract functions declared in the interface are implemented by this class or not
    pub fn add_impl(&mut self, interface : RefCount<Interface>){
        for abstract_func in &interface.inner().funcs {
            let name = &abstract_func.name;
            let param_types = &abstract_func.param_types;
            let return_type = &abstract_func.return_type;
            match self.map.get(name.as_str()) {
                None => panic!("function {} that declared at interface {} need be implemented by class {}",
                               name,
                               interface.inner().name,
                               self.name),
                Some((index,_,_,is_func)) => {
                    if ! is_func {
                        panic!("{} in class {} is not a function but a field with type {:?}", name, self.name, self.field_indexer.get_type(*index))
                    }
                    // check param type and return type
                    let func = self.funcs.get(*index as usize).unwrap();
                    let func_ref = func.inner_mut();
                    let params = &func_ref.info.params;
                    let real_return_type = &func_ref.info.return_type;
                    if ! real_return_type.eq(return_type) {
                        panic!("the return type of function {} that declared in interface {} is {:?} but in fact found {:?} in the implemented class {}",
                               name,
                               interface.inner().name,
                               return_type,
                               real_return_type,
                               self.name)
                    }
                    if params.len() != param_types.len() {
                        panic!("the params length of function {} that declared in interface {} is different from the implementation function in class {}",
                               name,
                               interface.inner().name,
                               self.name)
                    }
                    let mut equal = true;
                    let mut index = 0;
                    for param in params.iter() {
                        let real_type = &param.data_type;
                        let found_type = param_types.get(index).unwrap();
                        index+=1;
                        if ! found_type.belong_to(real_type)  {
                            equal = false;
                            break
                        }
                    }
                    if ! equal {
                        panic!("the param types of function {} that declared in interface {} is {:?}, which different from the implemented function in class {}",
                               name,
                               interface.inner().name,
                               param_types,
                               self.name)
                    }
                }
            }
        }
        self.impls.push(interface);
    }

    pub fn add_field(&mut self, is_pub : bool, field_name : String, data_type : DataType){
        self.field_count += 1;
        let (slot_idx,sub_idx) = self.field_indexer.put(data_type);
        self.map.insert(field_name, (slot_idx,sub_idx,is_pub,false));
    }

    pub fn add_func(&mut self, is_pub : bool,
                    func_name : Rc<String>,
                    params: Vec<Param>,
                    return_type: ReturnType,
                    body : Vec<Statement>){
        let index = self.funcs.len() as u16;
        match self.map.entry(func_name.deref().clone()) {
            Entry::Vacant(entry) => {
                entry.insert((index,0,is_pub,true));
            }
            Entry::Occupied(entry) => {
                panic!("the function name {} of class {} is already occupied : {:?}",
                       func_name,self.name,entry)
            }
        }
        // found drop fn
        if func_name.deref().eq("drop")
            && return_type.is_void()
            && params.len() == 1
            && params.get(0).unwrap().name.deref().eq("self"){
            self.fn_drop_idx = index;
        }
        self.funcs.push(RefCount::new(
            GloomFunc::new(func_name,self.file_index,params,return_type,body)
        ));
    }

    #[inline]
    pub fn handle_instance_func(&self, myself : RefCount<GloomClass>){
        let data_type = DataType::Ref(RefType::Class(myself));
        for func in self.funcs.iter() {
            func.inner_mut().handle_instance_func(&data_type);
        }
    }

    #[inline]
    pub fn is_derived_from(&self, class : &RefCount<GloomClass>) -> bool {
        match &self.parent {
            None => false,
            Some(real_parent) => {
                real_parent.eq(class) || real_parent.inner().is_derived_from(class)
            }
        }
    }

    #[inline]
    pub fn is_impl_from(&self, interface : &RefCount<Interface>) -> bool{
        for real_impl in self.impls.iter() {
            if real_impl.eq(interface) || real_impl.inner().derived_from(interface) {
                return true
            }
        }
        false
    }

    #[inline]
    pub fn len(&self) -> u16{
        self.field_indexer.size()
    }

    #[inline]
    pub fn ref_index_iter(&self) -> Iter<'_,u16> {
        self.field_indexer.curr_drop_vec().iter()
    }
}

impl Display for GloomClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.name)
    }
}

impl Debug for GloomClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.parent.is_none() {
            writeln!(f, "Class {} impl {:?} {:?} {:?}",
                     self.name,
                     self.impls,
                     self.field_indexer,
                     self.funcs)
        }else{
            writeln!(f, "Class {} : {} impl {:?} {:?} {:?}",
                     self.name,
                     self.parent.as_ref().unwrap().inner(),
                     self.impls,
                     self.field_indexer,
                     self.funcs)
        }
    }
}

pub struct GloomClassObj{
    pub class : RefCount<GloomClass>
}

impl Object for GloomClassObj {
    fn obj_type(&self) -> ObjectType {
        ObjectType::MetaClass
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn drop_by_exec(&self, _ : &Executor, _ : &GloomObjRef) {}

    fn at(&self, _ : &mut usize) -> Option<Value> {
        panic!()
    }
}

impl Debug for GloomClassObj {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.class)
    }
}

impl GloomClassObj {
    #[inline]
    pub fn new(class : RefCount<GloomClass>) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomClassObj{
            class
        }))
    }
}