use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use hashbrown::HashMap;
use crate::obj::func::ReturnType;
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};

pub struct Interface{
    pub name : Rc<String>,
    pub parents : Vec<RefCount<Interface>>,
    //              func name    params type   return type  have self
    pub funcs : Vec<AbstractFunc>,
    pub map : HashMap<Rc<String>,u16>
}

#[derive(Debug)]
pub struct AbstractFunc{
    pub name : Rc<String>,
    pub param_types : Vec<DataType>,
    pub return_type : ReturnType,
    pub have_self : bool
}

impl Interface {
    pub fn new(name : Rc<String>) -> Interface{
        Interface{
            name,
            parents: Vec::new(),
            funcs : Vec::new(),
            map: HashMap::new(),
        }
    }
    #[inline]
    pub fn add_func(&mut self, func : AbstractFunc){
        for occupied_func in self.funcs.iter() {
            if func.name.eq(&occupied_func.name) {
                return;
            }
        }
        self.funcs.push(func);
    }
    pub fn add_parent(&mut self, myself :&RefCount<Interface>, parent : &RefCount<Interface>){
        for func in parent.inner().funcs.iter() {
            let mut new_param_types = func.param_types.clone();
            if func.have_self {
                *new_param_types.get_mut(0).unwrap() = DataType::Ref(RefType::Interface(myself.clone()));
            }
            self.add_func(AbstractFunc{
                name: func.name.clone(),
                param_types: new_param_types,
                return_type: func.return_type.clone(),
                have_self: func.have_self
            });
        }
    }
    pub fn derived_from(&self, interface : &RefCount<Interface>) -> bool {
        for real_parent in self.parents.iter() {
            if real_parent.eq(interface) || real_parent.inner().derived_from(interface) {
                return true
            }
        }
        false
    }
    pub fn len(&self) -> usize{
        self.funcs.len()
    }
}

impl AbstractFunc {
    pub fn func_type(&self) -> DataType{
        DataType::Ref(RefType::Func(
            Box::new((
                self.param_types.clone(),
                self.return_type.clone()
            ))
        ))
    }
}

impl Debug for Interface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Interface {} {:?}",self.name,self.funcs)
    }
}

impl Display for Interface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.name)
    }
}