use std::any::Any;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::slice::Iter;

use hashbrown::hash_map::Entry;
use hashbrown::HashMap;

use crate::frontend::ast::Statement;
use crate::frontend::error::AnalysisError;
use crate::frontend::index::SlotIndexer;
use crate::frontend::status::GloomStatus;
use crate::obj::func::{GloomFunc, GloomFuncObj, Param, ReturnType};
use crate::obj::interface::Interface;
use crate::obj::object::{GloomObjRef, Object, ObjectType};
use crate::obj::refcount::RefCount;
use crate::obj::types::{DataType, RefType};
use crate::vm::machine::GloomVM;
use crate::vm::value::Value;

pub struct GloomClass {
    pub name: Rc<String>,
    parent: Option<RefCount<GloomClass>>,
    pub impls: Vec<InterfaceImpl>,
    pub map: HashMap<String, (u16, u8, IsPub, IsMemFunc)>,
    pub field_indexer: SlotIndexer,
    pub funcs: Vec<RefCount<GloomFunc>>,
    pub file_index: u16,
    pub class_index: u16,
    pub field_count: u16,
    pub fn_drop_idx: u16,
    pub is_filled: bool,
}

pub type IsMemFunc = bool;
pub type IsPub = bool;

#[derive(Debug, Clone)]
pub struct InterfaceImpl {
    pub interface: RefCount<Interface>,
    fn_table: Vec<u16>,
    // index : interface_fn_index, elem : class_fn_index
}

impl GloomClass {
    pub fn new(class_name: Rc<String>, file_index: u16, class_index: u16) -> GloomClass {
        GloomClass {
            name: class_name,
            parent: Option::None,
            impls: Vec::with_capacity(0),
            map: HashMap::new(),
            funcs: Vec::new(),
            file_index,
            field_indexer: SlotIndexer::new(),
            fn_drop_idx: u16::MAX,
            field_count: 0,
            class_index,
            is_filled: false,
        }
    }

    // 如有父类，需要被首先调用 if this class have parent class, this function need to be called first
    pub fn set_parent(&mut self, parent: RefCount<GloomClass>) {
        let parent_ref = parent.inner();
        self.impls = parent_ref.impls.clone();
        self.map = parent_ref.map.clone();
        self.funcs = parent_ref.funcs.clone();
        self.field_indexer = parent_ref.field_indexer.clone();
        self.field_count = parent_ref.field_count;
        self.fn_drop_idx = parent_ref.fn_drop_idx;

        std::mem::drop(parent_ref);
        self.parent = Some(parent);
    }

    // 最后再调用，因为会检查接口抽象方法是否实现 last to call this function,
    // because this function will check the abstract functions declared in the interface are implemented by this class or not
    pub fn add_impl(&self, interface_rf: RefCount<Interface>) -> Result<(), AnalysisError> {
        let interface = interface_rf.inner();
        let mut fn_table = Vec::with_capacity(interface.funcs.len());
        for abstract_func in interface.funcs.iter() {
            let abstract_func = abstract_func.inner();
            let name = &abstract_func.info.name;
            let expect_params = &abstract_func.info.params;
            let expect_return_type = &abstract_func.info.return_type;
            match self.map.get(name.as_str()) {
                None => {
                    return Result::Err(AnalysisError::FnNotImpl {
                        class: self.name.to_string(),
                        interface: interface.name.to_string(),
                        func: name.to_string(),
                    })
                }
                Some((index, _, _, is_func)) => {
                    fn_table.push(*index);
                    if !is_func {
                        return Result::Err(AnalysisError::FnNotImpl {
                            class: self.name.to_string(),
                            interface: interface.name.to_string(),
                            func: name.to_string(),
                        });
                    }
                    // check param type and return type
                    let func = self.funcs.get(*index as usize).unwrap();
                    let func_ref = func.inner_mut();
                    let found_params = &func_ref.info.params;
                    let real_return_type = &func_ref.info.return_type;
                    if !real_return_type.belongs_to(expect_return_type) {
                        return Result::Err(AnalysisError::MismatchedImplReturnType {
                            func: name.to_string(),
                            inter: interface.name.to_string(),
                            class: self.name.to_string(),
                            expect: expect_return_type.clone(),
                            found: real_return_type.clone(),
                        });
                    }
                    if found_params.len() != expect_params.len() {
                        return Result::Err(AnalysisError::MismatchImplParamLen {
                            func: name.to_string(),
                            inter: interface.name.to_string(),
                            class: self.name.to_string(),
                            expect: expect_params.len(),
                            found: found_params.len(),
                        });
                    }
                    let mut param_iter = found_params.iter().zip(expect_params.iter()).enumerate();
                    if func_ref.info.need_self {
                        param_iter.next();
                    }
                    for (idx, (found_param, expect_param)) in param_iter {
                        let expect_type = &expect_param.data_type;
                        let found_type = &found_param.data_type;
                        if !found_type.belong_to(expect_type) {
                            return Result::Err(AnalysisError::MismatchedImplParamType {
                                idx: idx + 1,
                                func: name.to_string(),
                                inter: interface.name.to_string(),
                                class: self.name.to_string(),
                                expect: expect_type.clone(),
                                found: found_type.clone(),
                            });
                        }
                    }
                }
            }
        }

        let mut vec = unsafe {
            &mut *(&(self.impls) as *const Vec<InterfaceImpl> as *mut Vec<InterfaceImpl>)
        };
        vec.push(InterfaceImpl {
            interface: interface_rf.clone(),
            fn_table,
        });
        Result::Ok(())
    }

    pub fn add_field(&mut self, is_pub: bool, field_name: String, data_type: DataType) {
        self.field_count += 1;
        let (slot_idx, sub_idx) = self.field_indexer.put(data_type);
        self.map
            .insert(field_name, (slot_idx, sub_idx, is_pub, false));
    }

    pub fn add_func(
        &mut self,
        is_pub: bool,
        func_name: Rc<String>,
        params: Vec<Param>,
        return_type: ReturnType,
        body: Vec<Statement>,
    ) -> Result<(), AnalysisError> {
        let index = self.funcs.len() as u16;
        // found drop fn
        if func_name.deref().eq("drop")
            && return_type.is_void()
            && params.len() == 1
            && params.get(0).unwrap().name.deref().eq("self")
        {
            self.fn_drop_idx = index;
        }
        match self.map.entry(func_name.deref().clone()) {
            Entry::Vacant(entry) => {
                entry.insert((index, 0, is_pub, true));
                self.funcs.push(RefCount::new(GloomFunc::new(
                    func_name,
                    self.file_index,
                    params,
                    return_type,
                    body,
                )));
            }
            Entry::Occupied(mut entry) => {
                /*return Result::Err(AnalysisError::FnAlreadyOccupied {
                    symbol: func_name.to_string(),
                    typ: self.name.to_string()
                })*/
                let (fn_idx, _, _, _) = *entry.get();
                *self.funcs.get_mut(fn_idx as usize).unwrap() = RefCount::new(GloomFunc::new(
                    func_name,
                    self.file_index,
                    params,
                    return_type,
                    body,
                ));
                entry.replace_entry((fn_idx, 0, is_pub, true));
            }
        }
        Result::Ok(())
    }

    #[inline]
    pub fn handle_instance_func(&self, myself: RefCount<GloomClass>) {
        let data_type = DataType::Ref(RefType::Class(myself));
        for func in self.funcs.iter() {
            func.inner_mut().handle_instance_func(&data_type);
        }
    }

    #[inline]
    pub fn is_derived_from(&self, class: &RefCount<GloomClass>) -> bool {
        match &self.parent {
            None => false,
            Some(real_parent) => {
                real_parent.eq(class) || real_parent.inner().is_derived_from(class)
            }
        }
    }

    #[inline]
    pub fn is_impl_from(&self, interface: &RefCount<Interface>) -> bool {
        for real_impl in self.impls.iter() {
            if real_impl.interface.eq(interface)
                || real_impl.interface.inner().derived_from(interface)
            {
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn len(&self) -> u16 {
        self.field_indexer.size()
    }

    #[inline]
    pub fn ref_index_iter(&self) -> Iter<'_, u16> {
        self.field_indexer.curr_drop_vec().iter()
    }

    #[inline]
    pub fn dynamic_dispatch(&self, interface_idx: u16, fn_idx: u16) -> &RefCount<GloomFunc> {
        let result = self.impls.binary_search_by(|impl_table| {
            impl_table
                .interface
                .inner()
                .interface_index
                .cmp(&interface_idx)
        });
        let real_fn_idx = *self
            .impls
            .get(result.unwrap())
            .unwrap()
            .fn_table
            .get(fn_idx as usize)
            .unwrap();
        self.funcs.get(real_fn_idx as usize).unwrap()
    }
}

impl Display for GloomClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Debug for GloomClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.parent.is_none() {
            writeln!(
                f,
                "Class {} impl {:?} {:?} {:?}",
                self.name, self.impls, self.field_indexer, self.funcs
            )
        } else {
            writeln!(
                f,
                "Class {} : {} impl {:?} {:?} {:?}",
                self.name,
                self.parent.as_ref().unwrap().inner(),
                self.impls,
                self.field_indexer,
                self.funcs
            )
        }
    }
}

pub struct GloomClassObj {
    pub class: RefCount<GloomClass>,
}

impl Object for GloomClassObj {
    fn obj_type(&self) -> ObjectType {
        ObjectType::MetaClass
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn drop_by_vm(&self, _: &GloomVM, _: &GloomObjRef) {}

    fn iter(&self, _: &GloomObjRef) -> GloomObjRef {
        todo!()
    }

    fn at(&self, _: &mut usize) -> Option<Value> {
        panic!()
    }

    fn next(&self) -> Value {
        panic!()
    }

    fn method(&self, _: u16, _: &GloomStatus) -> RefCount<GloomFunc> {
        todo!()
    }

    fn field(&self, i1: u16, _: u8) -> Value {
        Value::Ref(GloomFuncObj::new_func(
            self.class.inner().funcs.get(i1 as usize).unwrap().clone(),
        ))
    }
}

impl Debug for GloomClassObj {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.class)
    }
}

impl GloomClassObj {
    #[inline]
    pub fn new(class: RefCount<GloomClass>) -> GloomObjRef {
        GloomObjRef::new(Rc::new(GloomClassObj { class }))
    }
}
