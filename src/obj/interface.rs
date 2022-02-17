use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use hashbrown::HashMap;

use crate::obj::func::{FuncBody, FuncInfo, GloomFunc, ReturnType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};

pub struct Interface {
    pub name: Rc<String>,
    pub parents: Vec<RefCount<Interface>>,
    //              func name    params type   return type  have self
    pub funcs: Vec<RefCount<GloomFunc>>,
    pub map: HashMap<Rc<String>, u16>,
    pub interface_index: u16,
}

impl Interface {
    pub fn new(name: Rc<String>, index: u16) -> Interface {
        Interface {
            name,
            parents: Vec::new(),
            funcs: Vec::new(),
            map: HashMap::new(),
            interface_index: index,
        }
    }
    #[inline]
    pub fn add_func(&mut self, func: RefCount<GloomFunc>) {
        for occupied_func in self.funcs.iter() {
            if func.inner().info.name.eq(&occupied_func.inner().info.name) {
                return;
            }
        }
        self.funcs.push(func);
    }
    pub fn add_parent(&mut self, myself: &RefCount<Interface>, parent: &RefCount<Interface>) {
        for func in parent.inner().funcs.iter() {
            let func = func.inner();
            let mut new_func = GloomFunc {
                info: FuncInfo::clone(&func.info),
                body: FuncBody::None,
            };
            if func.info.need_self {
                new_func.info.params.get_mut(0).unwrap().data_type =
                    DataType::Ref(RefType::Interface(myself.clone()));
            }
            let new_func = RefCount::new(new_func);
            self.add_func(new_func);
        }
    }
    pub fn derived_from(&self, interface: &RefCount<Interface>) -> bool {
        for real_parent in self.parents.iter() {
            if real_parent.eq(interface) || real_parent.inner().derived_from(interface) {
                return true;
            }
        }
        false
    }
    pub fn len(&self) -> usize {
        self.funcs.len()
    }
}

impl Debug for Interface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Interface {} {:?}", self.name, self.funcs)
    }
}

impl Display for Interface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
