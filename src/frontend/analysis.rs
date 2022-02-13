use std::borrow::{Borrow, BorrowMut};
use std::ops::{Add, Deref, DerefMut};
use std::option::Option::Some;
use std::rc::Rc;

use hashbrown::hash_map::Entry;
use hashbrown::HashMap;

use crate::{
    builtin::classes::BuiltinClass,
    builtin::funcs::{BuiltInFuncs, IsBuiltIn},
    frontend::{
        ast::*,
        index::SlotIndexer,
        ops::BinOpType,
        script::{ParsedFile, ScriptBody},
        status::{GloomStatus, MetaType, TypeIndex},
    },
    obj::func::{Capture, FuncBody, GloomFunc, Param, ReturnType},
    obj::gloom_class::{GloomClass, IsPub},
    obj::gloom_enum::GloomEnumClass,
    obj::interface::{AbstractFunc, Interface},
    obj::refcount::RefCount,
    obj::types::{BreakType, BuiltinType, DataType, DeclaredType, RefType},
};
use crate::frontend::ast::BlockType;
use crate::frontend::error::AnalysisError;
use crate::frontend::error::AnalysisError::{NoFieldType, UnknownField};
use crate::frontend::ops::LeftValueOp;
use crate::frontend::token::Token;
use crate::obj::func::FuncInfo;
use crate::vm::static_table::StaticTable;

pub struct Analyzer {
    status: GloomStatus,

    file_count: u16,
    // 声明类型的后面的u16是该类型所属的文件索引，用于检查访问权限
    // the u16 after declared type is script file index, used for check the access auth
    parsed_interfaces: Vec<(ParsedInterface, u16)>,
    parsed_classes: Vec<(RefCount<ParsedClass>, u16)>,
    parsed_enums: Vec<(RefCount<ParsedEnum>, u16)>,
    pub func_map: HashMap<String, (u16, IsBuiltIn, IsPub, u16)>,
    pub type_map: HashMap<String, TypeIndex>,
    pub static_map: RefCount<HashMap<String, (u16, u8)>>,
    builtin_map: HashMap<BuiltinType, u16>,
    static_indexer: RefCount<SlotIndexer>,
    paths: Vec<String>,
}

// 这些字段被存储到Analyzer而非GloomStatus中，这意味着我想要它们在运行前被丢弃。
// these fields are stored in Analyzer rather than GloomStatus, because I want to discard them before execution
type IsLocal = bool;

impl Analyzer {
    pub fn analysis(&mut self, mut script: ParsedFile, debug: bool) -> Result<(),AnalysisError> {
        // load types
        // 加载空的定义类型 load empty declared type : class interface and enum
        self.load_decl(&mut script);
        // 加载原类型以及直接定义的函数 load original class interface enum and directly-declared func
        self.load(script);
        // 最后加载的脚本文件最先执行 first run the last loaded script file
        self.status.script_bodies.sort_by(|b1, b2| {
            b2.inner().file_index.cmp(&b1.inner().file_index)
        });
        // analysis interface, nothing need be filled
        self.analysis_interfaces();
        // fill fields and functions of class
        self.fill_classes();
        // fill enum value declaration and functions of enum
        self.fill_enums();
        // analysis and check executable code
        // functions in classes
        for class in self.status.classes.iter() {
            let class = class.clone();
            let file_index = class.inner().file_index;
            for func in class.inner().funcs.iter() {
                let func = func.clone();
                let mut func_ref = func.inner_mut();
                self.analysis_func(
                    &mut *func_ref,
                    file_index,
                    Option::None,
                    DeclaredType::Class(class.clone())
                )?;
            }
        }
        // function in enums
        for enum_class in self.status.enums.iter() {
            let enum_class = enum_class.clone();
            let file_index = enum_class.inner().file_index;
            for func in enum_class.inner().funcs.iter() {
                let func = func.clone();
                let mut func_ref = func.inner_mut();
                self.analysis_func(
                    &mut *func_ref,
                    file_index,
                    Option::None,
                    DeclaredType::Enum(enum_class.clone())
                )?;
            }
        }
        // functions that declared directly
        for func in self.status.funcs.iter() {
            let func = func.clone();
            let mut func_ref = func.inner_mut();
            let file_index = func_ref.info.file_index;
            self.analysis_func(
                &mut *func_ref,
                file_index,
                Option::None,
                DeclaredType::IsNot
            )?;
        }
        // script executable body
        for script_body in self.status.script_bodies.iter() {
            let file_index = script_body.inner().file_index;
            let script_body_rc = script_body.clone();
            let mut script_body_ref = script_body_rc.inner_mut();
            self.analysis_func(
                &mut script_body_ref.func,
                file_index,
                Option::None,
                DeclaredType::IsNot
            )?;
        }
        if debug {
            println!("{:?}", self.status)
        }
        Result::Ok(())
    }

    fn analysis_func(&self, func: &mut GloomFunc, file_index: u16, out_env: Option<&AnalyzeContext>, belonged_type: DeclaredType) -> Result<(),AnalysisError> {
        let params = &mut func.info.params;
        let func_return_type = &func.info.return_type;
        let mut context = AnalyzeContext::new(
            func.info.name.clone(),
            belonged_type,
            func_return_type.clone(),
            file_index,
            self.paths.get(file_index as usize).unwrap().as_str(),
            out_env);
        // load param into symbol table and allocate local slot for parameters
        for param in params.iter_mut() {
            let param_name = &param.name;
            let param_type = &param.data_type;
            let (slot_idx, sub_idx) = context.indexer.put(param_type.clone());
            param.index = (slot_idx, sub_idx);
            match context.symbol_table.entry(param_name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert((slot_idx, sub_idx, true));
                }
                Entry::Occupied(_) => return Result::Err(AnalysisError::SymbolAlreadyOccupied {
                    info: context.info(),
                    symbol: param_name.deref().clone()
                })
            };
        }
        // analysis per statement
        let body = &mut func.body;
        if let FuncBody::AST(body) = body {
            context.block_stack.push(BlockType::Func);
            self.analysis_statements(&mut context, body)?;
            context.block_stack.pop();
        }
        func.info.captures = context.captures;
        func.info.local_size = context.indexer.size();
        func.info.drop_slots = context.indexer.basic_drop_vec();
        Result::Ok(())
    }

    #[inline]
    fn handle_left_value_op(&self, context: &mut AnalyzeContext, left_val_tuple: &mut Box<(LeftValue, LeftValueOp)>) -> Result<DataType,AnalysisError> {
        let (left_val, left_val_op) = left_val_tuple.deref_mut();
        let left_val_type = match left_val {
            LeftValue::Var(var) => {
                let var_name_ref = var.name().clone();
                match context.symbol_table.get(var_name_ref.as_str()) {
                    Some((slot_idx, sub_idx, is_local)) => {
                        if *is_local {
                            let data_type = context.indexer.get_type(*slot_idx).clone();
                            *var = Var::new_local(*slot_idx, *sub_idx, data_type.as_basic());
                            data_type
                        } else {
                            let static_indexer = self.static_indexer.inner();
                            let data_type = static_indexer.get_type(*slot_idx).clone();
                            *var = Var::new_static(*slot_idx, *sub_idx, data_type.as_basic());
                            data_type
                        }
                    }
                    // TODO captured left value
                    None => return Result::Err(AnalysisError::UndefinedVar {
                        info: context.info(),
                        line: 0,
                        var: var_name_ref.deref().clone()
                    })
                }
            }
            LeftValue::Chain(_, _) => {
                DataType::Int
            }
        };
        Result::Ok(match left_val_op {
            LeftValueOp::Assign(expr) => {
                let expr_type = self.deduce_type(expr, context)?;
                if !expr_type.belong_to(&left_val_type) {
                    return Result::Err(AnalysisError::AssignMismatchedType {
                        info: context.info(),
                        found: expr_type,
                        expect: left_val_type
                    })
                }
                expr_type
            }
            LeftValueOp::PlusEq(expr) => {
                if !left_val_type.is_int_or_num() {
                    return Result::Err(AnalysisError::CalcInplaceLeftMismatchedType {
                        info: context.info(),
                        ops: Token::PlusEq,
                        found: left_val_type.clone()
                    })
                }
                let expr_type = self.deduce_type(expr, context)?;
                if !expr_type.is_int_or_num() {
                    return Result::Err(AnalysisError::CalcInplaceRightMismatchedType {
                        info: context.info(),
                        ops: Token::PlusEq,
                        found: expr_type.clone()
                    })
                }
                left_val_type
            }
            LeftValueOp::SubEq(expr) => {
                if !left_val_type.is_int_or_num() {
                    return Result::Err(AnalysisError::CalcInplaceLeftMismatchedType {
                        info: context.info(),
                        ops: Token::SubEq,
                        found: left_val_type.clone()
                    })
                }
                let expr_type = self.deduce_type(expr, context)?;
                if !expr_type.is_int_or_num() {
                    return Result::Err(AnalysisError::CalcInplaceRightMismatchedType {
                        info: context.info(),
                        ops: Token::SubEq,
                        found: expr_type.clone()
                    })
                }
                left_val_type
            }
            LeftValueOp::PlusOne => {
                if !left_val_type.is_int_or_num() {
                    return Result::Err(AnalysisError::CalcInplaceLeftMismatchedType {
                        info: context.info(),
                        ops: Token::PlusPlus,
                        found: left_val_type.clone()
                    })
                }
                left_val_type
            }
            LeftValueOp::SubOne => {
                if !left_val_type.is_int_or_num() {
                    return Result::Err(AnalysisError::CalcInplaceLeftMismatchedType {
                        info: context.info(),
                        ops: Token::SubSub,
                        found: left_val_type.clone()
                    })
                }
                left_val_type
            }
        })
    }

    #[inline]
    fn handle_chains(&self, context: &mut AnalyzeContext, chains: &mut Box<(Expression, Vec<Chain>)>) -> Result<DataType,AnalysisError> {
        let (expr, chain_vec) = chains.deref_mut();
        let mut expr_type = self.deduce_type(expr, context)?;
        let mut new_type = DataType::Ref(RefType::None);
        let chains_len = chain_vec.len();
        for (chain_idx, chain) in chain_vec.iter_mut().enumerate() {
            match chain {
                Chain::Access(field, basic_type) => {
                    let field_name = field.name();
                    match &expr_type {
                        // find field
                        DataType::Ref(RefType::Class(class)) => {
                            match class.inner().map.get(field_name.as_str()) {
                                Some((slot_idx, sub_idx, is_pub, is_mem_func)) => {
                                    if !*is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                        *field = VarId::Index(*slot_idx, *sub_idx);
                                        new_type = class.inner().field_indexer.get_type(*slot_idx).clone();
                                        *basic_type = new_type.as_basic();
                                    } else {
                                        return Result::Err(AnalysisError::UnknownField {
                                            info: context.info(),
                                            name: field_name.deref().clone(),
                                            class: class.inner().name.deref().clone()
                                        })
                                    }
                                }
                                None => {
                                    return Result::Err(AnalysisError::UnknownField {
                                        info: context.info(),
                                        name: field_name.deref().clone(),
                                        class: class.inner().name.deref().clone()
                                    })
                                }
                            };
                        }
                        // find function
                        DataType::Ref(RefType::MetaClass(class)) => {
                            match class.inner().map.get(field_name.as_str()) {
                                Some((index, _, is_pub, is_mem_func)) => {
                                    if *is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                        *field = VarId::Index(*index, 0);
                                        new_type = class.inner().funcs.get(*index as usize).unwrap().inner().get_type();
                                    } else {
                                        return Result::Err(AnalysisError::UnknownField {
                                            info: context.info(),
                                            name: field_name.deref().clone(),
                                            class: class.inner().name.deref().clone()
                                        })
                                    }
                                }
                                None => return Result::Err(AnalysisError::UnknownField {
                                    info: context.info(),
                                    name: field_name.deref().clone(),
                                    class: class.inner().name.deref().clone()
                                })
                            }
                        }
                        DataType::Ref(RefType::MetaEnum(class)) => {
                            match class.inner().func_map.get(field_name.as_str()) {
                                Some((index, is_pub)) => {
                                    if *is_pub || context.belonged_type.equal_enum(class) {
                                        *field = VarId::Index(*index, 0);
                                        new_type = class.inner().funcs.get(*index as usize).unwrap().inner().get_type();
                                    } else {
                                        return Result::Err(AnalysisError::UnknownField {
                                            info: context.info(),
                                            name: field_name.deref().clone(),
                                            class: class.inner().name.deref().clone()
                                        })
                                    }
                                }
                                None => return Result::Err(AnalysisError::UnknownField {
                                    info: context.info(),
                                    name: field_name.deref().clone(),
                                    class: class.inner().name.deref().clone()
                                })
                            }
                        }
                        DataType::Ref(RefType::MetaInterface(class)) => {
                            match class.inner().map.get(field_name.deref()) {
                                Some(index) => {
                                    *field = VarId::Index(*index, 0);
                                    new_type = class.inner().funcs.get(*index as usize).unwrap().func_type();
                                }
                                None => return Result::Err(AnalysisError::UnknownField {
                                    info: context.info(),
                                    name: field_name.deref().clone(),
                                    class: class.inner().name.deref().clone()
                                })
                            }
                        }
                        DataType::Ref(RefType::MataBuiltinType(builtin_type)) => {
                            match self.builtin_map.get(builtin_type) {
                                Some(index) => {
                                    *field = VarId::Index(*index, 0);
                                    new_type = DataType::Ref(self.status.builtin_classes
                                        .get(*index as usize).unwrap().inner().get_ref_type(Option::None).unwrap());
                                }
                                None => return Result::Err(AnalysisError::UnknownField {
                                    info: context.info(),
                                    name: field_name.deref().clone(),
                                    class: String::from(builtin_type.to_str())
                                })
                            }
                        }
                        other_type => return Result::Err(AnalysisError::NoFieldType {
                            info: context.info(),
                            typ: other_type.clone()
                        })
                    }

                }
                Chain::Call(args) => {
                    match &expr_type {
                        DataType::Ref(RefType::Func(func_type)) => {
                            let (param_types, return_type, _) = func_type.deref();
                            if param_types.len() != args.len() {
                                return Result::Err(AnalysisError::MismatchedArgsNum {
                                    info: context.info(),
                                    func_name: "".to_string(),
                                    func_type: DataType::Ref(RefType::Func(func_type.clone())),
                                    expect: param_types.len(),
                                    found: args.len()
                                })
                            }
                            for (arg_idx, (arg,param_type)) in args.iter_mut().zip(param_types.iter()).enumerate() {
                                let arg_type = self.deduce_type(arg, context)?;
                                if !arg_type.belong_to(param_type) {
                                    return Result::Err(AnalysisError::MismatchedArgType {
                                        info: context.info(),
                                        idx: arg_idx,
                                        func: "".to_string(),
                                        expect: param_type.clone(),
                                        found: arg_type
                                    })
                                }
                            }
                            match return_type {
                                ReturnType::Void => {
                                    if chain_idx != chains_len - 1 {
                                        panic!("{} function {} call return void but some chained operation are followed behind",
                                               context.info(), expr_type)
                                    }
                                }
                                ReturnType::Have(data_type) => {
                                    new_type = data_type.clone();
                                }
                            }
                        }
                        _ => panic!("{} the {} type is not a func type", context.info(), expr_type)
                    }
                }
                Chain::FnCall {
                    func,
                    need_self,
                    args
                } => {
                    let func_name = func.name();
                    let function: RefCount<GloomFunc>;
                    match &expr_type {
                        DataType::Ref(ref_type) => {
                            match ref_type {
                                // caller is object, call member function
                                RefType::Class(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.map.get(func_name.as_str()) {
                                        Some((index, sub_idx, is_pub, is_mem_func)) => {
                                            if *is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                if !target_func.info.need_self {
                                                    panic!("{} function {} of class {} is not a non-static function, which don't have 'self' as first parameter",
                                                           context.info(), func_name, class_ref.name)
                                                }
                                                *need_self = true;
                                                *func = VarId::Index(*index, *sub_idx);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len - 1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(), func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            } else {
                                                panic!("{} the '{}' of class {}  is not a function or not public ",
                                                       context.info(), func_name, class_ref.name)
                                            }
                                        }
                                        None => panic!("{} class {} have no function {}", context.info(), class_ref.name, func_name)
                                    };
                                }
                                RefType::Enum(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.func_map.get(func_name.as_str()) {
                                        Some((index, is_pub)) => {
                                            if *is_pub || context.belonged_type.equal_enum(class) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                if !target_func.info.need_self {
                                                    panic!("{} function {} of enum {} is not a non-static function, which don't have 'self' as first parameter",
                                                           context.info(), func_name, class_ref.name)
                                                }
                                                *need_self = true;
                                                *func = VarId::Index(*index, 0);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len - 1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(), func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            } else {
                                                panic!("{} the function '{}' of enum {} is not public ",
                                                       context.info(), func_name, class_ref.name)
                                            }
                                        }
                                        None => panic!("{} enum {} have no function {}",
                                                       context.info(), class_ref.name, func_name)
                                    }
                                }
                                RefType::Interface(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.map.get(func_name.deref()) {
                                        Some(index) => {
                                            let target_func = class_ref.funcs.get(*index as usize).unwrap();
                                            let mut params = Vec::with_capacity(target_func.param_types.len());
                                            let empty_name = Rc::new("".to_string());
                                            for param_type in target_func.param_types.iter() {
                                                params.push(Param::new(empty_name.clone(), param_type.clone()));
                                            }
                                            function = RefCount::new(GloomFunc {
                                                info: FuncInfo {
                                                    name: empty_name,
                                                    params,
                                                    return_type: ReturnType::Void,
                                                    captures: Vec::with_capacity(0),
                                                    drop_slots: Vec::with_capacity(0),
                                                    local_size: 0,
                                                    need_self: false,
                                                    file_index: 0,
                                                    stack_size: 0,
                                                },
                                                body: FuncBody::None,
                                            });
                                            if !target_func.have_self {
                                                panic!("{} function {} of interface {} is not a non-static function, which don't have 'self' as first parameter",
                                                       context.info(), func_name, class_ref.name)
                                            }
                                            *need_self = true;
                                            *func = VarId::Index(*index, 0);
                                            match &target_func.return_type {
                                                ReturnType::Void => {
                                                    if chain_idx != chains_len - 1 {
                                                        panic!("{} function {} call return void but some chained operation are followed behind",
                                                               context.info(), func_name)
                                                    }
                                                }
                                                ReturnType::Have(return_type) => {
                                                    new_type = return_type.clone();
                                                }
                                            }
                                        }
                                        None => panic!("{} interface {} have no function {}",
                                                       context.info(), class.inner().name, func_name)
                                    }
                                }
                                // caller is type, call member function
                                RefType::MetaClass(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.map.get(func_name.as_str()) {
                                        Some((index, _, is_pub, is_mem_func)) => {
                                            if *is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                *func = VarId::Index(*index, 0);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len - 1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(), func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            } else {
                                                panic!("{} the '{}' of class {}  is not a function or not public ",
                                                       context.info(), func_name, class_ref.name)
                                            }
                                        }
                                        None => panic!("{} class {} have no function {}", context.info(), class_ref.name, func_name)
                                    };
                                }
                                RefType::MetaEnum(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.func_map.get(func_name.as_str()) {
                                        Some((index, is_pub)) => {
                                            if *is_pub || context.belonged_type.equal_enum(class) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                *func = VarId::Index(*index, 0);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len - 1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(), func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            } else {
                                                panic!("{} the function '{}' of enum {} is not public ",
                                                       context.info(), func_name, class_ref.name)
                                            }
                                        }
                                        None => panic!("{} enum {} have no function {}",
                                                       context.info(), class_ref.name, func_name)
                                    }
                                }
                                RefType::MetaInterface(class) => {
                                    panic!("could not access the member function of Interface {}", class.inner())
                                }
                                RefType::MataBuiltinType(builtin_type) => {
                                    match self.builtin_map.get(builtin_type) {
                                        Some(index) => {
                                            let class = self.status.builtin_classes.get(*index as usize).unwrap();
                                            let class = class.inner();
                                            match class.map.get(func_name.as_str()) {
                                                Some(index) => {
                                                    function = class.funcs.get(*index as usize).unwrap().clone();
                                                    let target_func = function.inner();
                                                    *func = VarId::Index(*index, 0);
                                                    match &target_func.info.return_type {
                                                        ReturnType::Void => {
                                                            if chain_idx != chains_len - 1 {
                                                                panic!("{} function {} call return void but some chained operation are followed behind",
                                                                       context.info(), func_name)
                                                            }
                                                        }
                                                        ReturnType::Have(return_type) => {
                                                            new_type = return_type.clone();
                                                        }
                                                    }
                                                }
                                                None => {
                                                    panic!("{} not found function {} in builtin type {}",
                                                           context.info(), func_name, builtin_type.to_str())
                                                }
                                            }
                                        }
                                        None => {
                                            panic!("you may forgot import builtin type {:?} in std library", builtin_type)
                                        }
                                    }
                                }

                                RefType::Any | RefType::None | RefType::MySelf => {
                                    panic!()
                                }
                                // non-static function
                                builtin_type => {
                                    let builtin_type = builtin_type.as_built_type();
                                    match self.builtin_map.get(&builtin_type) {
                                        Some(index) => {
                                            let class = self.status.builtin_classes.get(*index as usize).unwrap();
                                            let class = class.inner();
                                            match class.map.get(func_name.as_str()) {
                                                Some(index) => {
                                                    function = class.funcs.get(*index as usize).unwrap().clone();
                                                    let target_func = function.inner();
                                                    if !target_func.info.need_self {
                                                        panic!("{} function {} of builtin type {} is not a non-static function, which don't have 'self' as first parameter",
                                                               context.info(), func_name, builtin_type.to_str())
                                                    }
                                                    *func = VarId::Index(*index, 0);
                                                    *need_self = true;
                                                    match &target_func.info.return_type {
                                                        ReturnType::Void => {
                                                            if chain_idx != chains_len - 1 {
                                                                panic!("{} function {} call return void but some chained operation are followed behind",
                                                                       context.info(), func_name)
                                                            }
                                                        }
                                                        ReturnType::Have(return_type) => {
                                                            new_type = return_type.clone();
                                                        }
                                                    }
                                                }
                                                None => {
                                                    panic!("{} not found function {} in builtin type {}",
                                                           context.info(), func_name, builtin_type.to_str())
                                                }
                                            }
                                        }
                                        None => {
                                            panic!("you may forgot import builtin type {:?} in std library", builtin_type)
                                        }
                                    }
                                }
                            }
                        }
                        // call non-static function
                        basic_type => {
                            panic!("basic data type value can't be caller of member function, found {} value as caller call function '{}'",
                                   basic_type, func_name)
                        }
                    }
                    let function = function.inner();
                    let mut param_iter = function.info.params.iter();
                    if *need_self {
                        let self_type = match param_iter.next() {
                            Some(param) => &param.data_type,
                            None => panic!("function {:?} have no parameter but need self argument", function),
                        };
                        if !expr_type.belong_to(&self_type) {
                            panic!("{} mismatched argument type in first argument 'self' of function {:?} call, expect {}, found {}",
                                   context.info(), function, self_type, expr_type)
                        }
                    }
                    for (idx, (arg_expr, param)) in args.iter_mut().zip(param_iter).enumerate() {
                        let arg_type = self.deduce_type(arg_expr, context)?;
                        if !arg_type.belong_to(&param.data_type) {
                            panic!("{} mismatched argument type in {}st argument of function {:?} call, expect {}, found {}",
                                   context.info(), idx, function, param.data_type, arg_type)
                        }
                    }
                }
            };
            expr_type = std::mem::replace(&mut new_type, DataType::Ref(RefType::None));
        }
        Result::Ok(expr_type)
    }

    fn deduce_type(&self, expr: &mut Expression, context: &mut AnalyzeContext) -> Result<DataType,AnalysisError> {
        let data_type = match expr {
            Expression::None => DataType::Ref(RefType::None),
            Expression::Int(_) => DataType::Int,
            Expression::Num(_) => DataType::Num,
            Expression::Char(_) => DataType::Char,
            Expression::Bool(_) => DataType::Bool,
            Expression::Str(_) => DataType::Ref(RefType::String),
            Expression::Var(var) => {
                let var_ref = var.deref_mut();
                let var_name = var_ref.name().clone();
                // find as variable
                let mut result_type = match context.symbol_table.get(var_name.as_str()) {
                    Some((slot_idx, sub_idx, is_local)) => {
                        if *is_local {
                            // non-static local variable
                            let data_type = context.indexer.get_type(*slot_idx).clone();
                            *var_ref = Var::new_local(*slot_idx, *sub_idx, data_type.as_basic());
                            data_type
                        } else {
                            // local variable
                            let data_type = self.static_indexer.inner().get_type(*slot_idx).clone();
                            *var_ref = Var::new_static(*slot_idx, *sub_idx, data_type.as_basic());
                            data_type
                        }
                    }
                    None => match self.static_map.inner().get(var_name.as_str()) {
                        Some((slot_idx, sub_idx)) => {
                            // public static variable
                            let data_type = self.static_indexer.inner().get_type(*slot_idx).clone();
                            *var_ref = Var::new_static(*slot_idx, *sub_idx, data_type.as_basic());
                            data_type
                        }
                        None => match context.out_context {
                            Some(out_context) => {
                                if let Some((out_slot_idx, out_sub_idx, is_local)) = out_context.symbol_table.get(var_name.as_str()) {
                                    if *is_local {
                                        // 捕获非静态的局部变量 captured non-static local variable
                                        // 记录捕获 插入符号表 record capture, insert into symbol table
                                        let captured_type = out_context.indexer.get_type(*out_slot_idx).clone();
                                        let cap_basic_type = captured_type.as_basic();
                                        let (slot_idx, sub_idx) = context.indexer.put(captured_type.clone());
                                        // 已经尝试通过该名称获取，所以不需要entry api。 try find this name before, so there are not same name variable here
                                        context.symbol_table.insert(var_name.deref().clone(), (slot_idx, sub_idx, true));
                                        context.captures.push(Capture::new(
                                            *out_slot_idx,
                                            *out_sub_idx,
                                            slot_idx,
                                            sub_idx,
                                            cap_basic_type,
                                        ));
                                        *var_ref = Var::new_local(slot_idx, sub_idx, cap_basic_type);
                                        captured_type
                                    } else {
                                        // captured static variable
                                        let data_type = self.static_indexer.inner().get_type(*out_slot_idx).clone();
                                        *var_ref = Var::new_static(*out_slot_idx, *out_sub_idx, data_type.as_basic());
                                        data_type
                                    }
                                } else {
                                    // capture failed
                                    DataType::Ref(RefType::None)
                                }
                            }
                            // this function have no outside env
                            None => DataType::Ref(RefType::None)
                        },
                    },
                };
                // find as type or function
                let is_none = result_type.is_none();
                if is_none {
                    match self.type_map.get(var_name.as_str()) {
                        // type
                        Some(label) => {
                            if label.is_public || label.file_index == context.file_index {
                                match label.tp {
                                    MetaType::Interface => {
                                        *var_ref = Var::Interface(label.index);
                                        let interface =
                                            self.status.interfaces.get(label.index as usize).unwrap().clone();
                                        result_type = DataType::Ref(RefType::MetaInterface(interface));
                                    }
                                    MetaType::Class => {
                                        *var_ref = Var::Class(label.index);
                                        let class =
                                            self.status.classes.get(label.index as usize).unwrap().clone();
                                        result_type = DataType::Ref(RefType::MetaClass(class));
                                    }
                                    MetaType::Enum => {
                                        *var_ref = Var::Enum(label.index);
                                        let class = self.status.enums.get(label.index as usize).unwrap().clone();
                                        result_type = DataType::Ref(RefType::MetaEnum(class));
                                    }
                                    MetaType::Builtin => {
                                        *var_ref = Var::BuiltinType(label.index);
                                        let ref_type = self.status.builtin_classes
                                            .get(label.index as usize).unwrap()
                                            .inner().get_ref_type(Option::None).unwrap();
                                        result_type = DataType::Ref(RefType::MataBuiltinType(ref_type.as_built_type()));
                                    }
                                }
                            } else {
                                panic!("{} Type {} is not public", context.info(), var_name)
                            }
                        }
                        // function
                        None => {
                            match self.func_map.get(var_name.as_str()) {
                                Some((index, _, is_pub, file_index)) => {
                                    if *is_pub || *file_index == context.file_index {
                                        *var_ref = Var::DirectFn(*index);
                                        let func_type = self.status.funcs.get(*index as usize).unwrap().inner().get_ref_type();
                                        result_type = DataType::Ref(func_type);
                                    } else {
                                        panic!("{} unknown type {}", context.info(), var_name)
                                    }
                                }
                                None => {
                                    panic!("{} unknown type {}", context.info(), var_name)
                                }
                            }
                        }
                    }
                }
                result_type
            }
            Expression::Chain(chains) => self.handle_chains(context, chains)?,
            Expression::Tuple(tuple) => {
                let vec = tuple.deref_mut();
                let mut tuple_types = Vec::with_capacity(vec.len());
                for expr in vec.iter_mut() {
                    tuple_types.push(self.deduce_type(expr, context)?);
                }
                DataType::Ref(RefType::Tuple(Box::new(tuple_types)))
            }
            Expression::Array(array) => {
                let (array, basic_type, _) = array.deref_mut();
                if array.len() == 0 {
                    // without any array item
                    DataType::Ref(RefType::Array(Box::new(DataType::Ref(RefType::Any))))
                } else {
                    // array with generic type
                    let mut iter = array.iter_mut();
                    let first_elem = iter.next().unwrap();
                    let mut data_type = self.deduce_type(first_elem, context)?;
                    if data_type.is_none() {
                        return Result::Err(AnalysisError::VoidInArray {
                            info: context.info(),
                            no: 1,
                            elem: format!("{:?}",first_elem)
                        })
                    }
                    for (idx, expr) in iter.enumerate() {
                        let temp_type = self.deduce_type(expr, context)?;
                        if temp_type.is_none() {
                            return Result::Err(AnalysisError::VoidInArray {
                                info: context.info(),
                                no: idx,
                                elem: format!("{:?}",expr)
                            })
                        }
                        if data_type != temp_type {
                            data_type = DataType::Ref(RefType::Any);
                        }
                    }
                    *basic_type = data_type.as_basic();
                    DataType::Ref(RefType::Array(Box::new(data_type)))
                }
            }
            Expression::Construct(construction) => {
                let con_type = &construction.deref_mut().class_type;
                let class_type = match con_type {
                    ExprType::Parsed(parsed_type) => {
                        self.get_type(parsed_type, context.file_index)
                    }
                    ExprType::Analyzed(data_type) => data_type.clone()
                };
                construction.class_type = ExprType::Analyzed(class_type.clone());
                let class_rc = match &class_type {
                    DataType::Ref(RefType::Class(class)) => class,
                    _ => return Result::Err(AnalysisError::UnexpectedNewClassMark {
                        info: context.info(),
                        wrong_type: class_type
                    })
                };
                let class = class_rc.inner();
                if class.field_count as usize != construction.fields.len() {
                    return Result::Err(AnalysisError::MismatchedNewFieldNum {
                        info: context.info(),
                        found: construction.fields.len(),
                        expect: class.field_count as usize
                    })
                }
                for (var, field_basic_type, expr) in construction.fields.iter_mut() {
                    let field_name = var.name();
                    match class.map.get(field_name.as_str()) {
                        Some((slot_idx, sub_idx, is_pub, is_fn)) => {
                            if *is_pub || context.belonged_type.equal_class(class_rc) {
                                if *is_fn {
                                    return Result::Err(AnalysisError::UnknownField {
                                        info: context.info(),
                                        name: field_name.deref().clone(),
                                        class: format!("{}",class_type)
                                    })
                                } else {
                                    let expr_type = self.deduce_type(expr, context)?;
                                    let field_type = class.field_indexer.get_type(*slot_idx);
                                    *field_basic_type = field_type.as_basic();
                                    if expr_type.belong_to(field_type) {
                                        *var = VarId::Index(*slot_idx, *sub_idx);
                                    } else {
                                        return Result::Err(AnalysisError::MismatchedFieldType {
                                            info: context.info(),
                                            name: field_name.deref().clone(),
                                            expect: field_type.clone(),
                                            found: expr_type,
                                            class: class_type.to_string()
                                        })
                                    }
                                }
                            } else {
                                return Result::Err(AnalysisError::FieldNotPublic {
                                    info: context.info(),
                                    name: field_name.deref().clone(),
                                    typ: class_type.to_string(),
                                    help: "so you can't construct the object of this class except in class member function"
                                })
                            }
                        }
                        None => return Result::Err(AnalysisError::UnknownField {
                            info: context.info(),
                            name: field_name.deref().clone(),
                            class: format!("{}",class_type)
                        })
                    }
                };
                class_type.clone()
            }
            Expression::NegOp(expr) => {
                self.deduce_type(expr.deref_mut(), context)?
            }
            Expression::NotOp(expr) => {
                self.deduce_type(expr.deref_mut(), context)?
            }
            Expression::Cast(cast) => {
                let (expr, parsed_type, data_type) = cast.deref_mut();
                let cast_type = self.get_type(parsed_type, context.file_index);
                let real_type = self.deduce_type(expr, context)?;
                *data_type = cast_type.clone();
                if (cast_type.is_num_liked() && real_type.is_num_liked())
                    || cast_type.belong_to(&real_type)
                    || real_type.belong_to(&cast_type) {
                    cast_type
                } else {
                    panic!("object type cast error, from {} to {}", real_type, cast_type)
                }
            }
            Expression::Func(func) => {
                let func_expr = func.deref_mut();
                // at this time, the func must be un-analyzed
                let mut func = match func_expr {
                    FuncExpr::Parsed(func) => {
                        let mut params = Vec::with_capacity(func.params.len());
                        for (name, parsed_type) in func.params.iter() {
                            params.push(Param::new(
                                name.clone(),
                                self.get_type(parsed_type, context.file_index),
                            ));
                        }
                        let return_type = match &func.return_type {
                            Some(parsed_type) => ReturnType::Have(self.get_type(parsed_type, context.file_index)),
                            None => ReturnType::Void
                        };
                        let statements = std::mem::replace(&mut func.body, Vec::with_capacity(0));
                        GloomFunc::new(
                            Rc::new(String::from("<nameless>")),
                            context.file_index,
                            params,
                            return_type,
                            statements)
                    }
                    FuncExpr::Analysed(_) => panic!(),
                };
                self.analysis_func(
                    &mut func,
                    context.file_index,
                    Option::Some(context),
                    context.belonged_type.clone()
                )?;
                let func_type = func.get_type();
                let is_parsed = func_expr.is_parsed();
                if is_parsed {
                    *func_expr = FuncExpr::Analysed(RefCount::new(func));
                };
                func_type
            }
            Expression::BinaryOp(bin_op) => {
                let bin_op = bin_op.deref_mut();
                let mut left_type = self.deduce_type(&mut bin_op.left, context)?;
                for (op, expr) in bin_op.vec.iter_mut() {
                    match op.to_type() {
                        BinOpType::Calculate => {
                            // number to number
                            if left_type.is_int_or_num() {
                                let right_type = self.deduce_type(expr, context)?;
                                if right_type.is_int_or_num() {
                                    if left_type.is_int() && right_type.is_int() {
                                        left_type = DataType::Int;
                                    } else {
                                        // 两操作数中有一或两个num类型  one or two of two operand are num type
                                        left_type = DataType::Num;
                                    }
                                } else {
                                    panic!("{} binary operator '{}' have wrong right operand type {}", context.info(), op, right_type)
                                }
                            } else {
                                panic!("{} binary operator '{}' have wrong left operand type {}", context.info(), op, left_type)
                            }
                        }
                        BinOpType::Compare => {
                            // number or char to bool
                            if left_type.is_num_liked() {
                                let right_type = self.deduce_type(expr, context)?;
                                if right_type.is_num_liked() {
                                    left_type = DataType::Bool;
                                } else {
                                    panic!("{} binary operator '{}' have wrong right operand type {}", context.info(), op, right_type)
                                }
                            } else {
                                panic!("{} binary operator '{}' have wrong left operand type {}", context.info(), op, left_type)
                            }
                        }
                        BinOpType::Equal => {
                            let right_type = self.deduce_type(expr, context)?;
                            if right_type.belong_to(&left_type) || left_type.belong_to(&right_type) {
                                left_type = DataType::Bool;
                            } else {
                                panic!("{} binary operator '{}' have wrong operand type, left : {} , right : {}", context.info(), op, left_type, right_type)
                            }
                        }
                        BinOpType::Logic => {
                            if left_type.is_bool() {
                                let right_type = self.deduce_type(expr, context)?;
                                if right_type.is_bool() {
                                    left_type = DataType::Bool;
                                } else {
                                    panic!("{} binary operator '{}' have wrong right operand type, expect bool found {}", context.info(), op, right_type)
                                }
                            } else {
                                panic!("{} binary operator '{}' have wrong left operand type, expect bool found {}", context.info(), op, left_type)
                            }
                        }
                    }
                }
                left_type
            }
            Expression::IfElse(if_else) => {
                match self.analysis_if_else(if_else.deref_mut(), context)? {
                    ReturnType::Have(data_type) => data_type,
                    ReturnType::Void => DataType::Ref(RefType::None)
                }
            }
            expr => return Result::Err(AnalysisError::UnsupportedSyntax(format!("{:?}",expr)))
        };
        Result::Ok(data_type)
    }

    fn analysis_while(&self, while_loop: &mut WhileLoop, context: &mut AnalyzeContext) -> Result<(),AnalysisError>{
        let line = while_loop.line;
        // check condition expression type
        let cond_expr = &mut while_loop.condition;
        let cond_type = self.deduce_type(cond_expr, context)?;
        if !cond_type.is_bool() {
            panic!("{} line {}, the loop condition expression is not bool type but {}", context.info(), line, cond_type)
        };
        let statements = &mut while_loop.statements;

        context.expr_stack.push((SyntaxType::While, line));
        context.block_stack.push(BlockType::Loop);
        context.indexer.enter_sub_block();

        self.analysis_statements(context, statements)?;

        context.expr_stack.pop();
        context.block_stack.pop();
        while_loop.drop_slots = context.indexer.level_sub_block();
        Result::Ok(())
    }

    fn analysis_for(&self, for_loop: &mut ForLoop, context: &mut AnalyzeContext) -> Result<(),AnalysisError> {
        let var_name;
        match &mut for_loop.for_iter {
            ForIter::Range(start, end, step) => {
                let range_type = self.deduce_type(start, context)?;
                if !range_type.is_int() {
                    panic!("{} expect int in start of for-in loop, found {}", context.info(), range_type)
                }
                let range_type = self.deduce_type(end, context)?;
                if !range_type.is_int() {
                    panic!("{} expect int in end of for-in loop, found {}", context.info(), range_type)
                }
                let range_type = self.deduce_type(step, context)?;
                if !range_type.is_int() {
                    panic!("{} expect int in step of for-in loop, found {}", context.info(), range_type)
                }

                let var = &mut for_loop.var;
                var_name = var.name();
                let (slot_idx, sub_idx) = context.indexer.put(DataType::Int);
                match context.symbol_table.entry(var_name.deref().clone()) {
                    Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx, true)),
                    Entry::Occupied(_) => panic!("{} variable '{}' already occupied", context.info(), var_name),
                };
                *var = Var::LocalInt(slot_idx, sub_idx);
            }
            ForIter::Iter(iter_expr) => {
                let mut iter_type = self.deduce_type(iter_expr, context)?;
                let item_type = match &mut iter_type {
                    DataType::Ref(RefType::Array(item_type)) => DataType::clone(&item_type),
                    DataType::Ref(RefType::Queue(item_type)) => DataType::clone(&item_type),
                    DataType::Ref(RefType::String) => DataType::Char,
                    other_type => {
                        panic!("{} can't apply for-in iteration in type {}", context.info(), other_type)
                    }
                };

                var_name = for_loop.var.name();
                let basic_type = item_type.as_basic();
                let (slot_idx, sub_idx) = context.indexer.put(item_type);
                match context.symbol_table.entry(var_name.deref().clone()) {
                    Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx, true)),
                    Entry::Occupied(_) => panic!("{} variable '{}' already occupied", context.info(), var_name),
                };
                for_loop.var = Var::new_local(slot_idx, sub_idx, basic_type);
            }
        }

        context.expr_stack.push((SyntaxType::ForIn, for_loop.line));
        context.block_stack.push(BlockType::Loop);
        context.indexer.enter_sub_block();

        self.analysis_statements(context, &mut for_loop.statements)?;

        context.symbol_table.remove(var_name.as_str());
        context.block_stack.pop();
        for_loop.drop_slots = context.indexer.level_sub_block();
        context.expr_stack.pop();
        Result::Ok(())
    }

    #[inline]
    fn analysis_statements(&self, context: &mut AnalyzeContext, statements: &mut Vec<Statement>) -> Result<(),AnalysisError> {
        let mut last_is_expr = false;
        let mut last_type = ReturnType::Void;

        let mut temp_var_table = Vec::new();
        let curr_block_type = *context.block_stack.last().unwrap();
        let var_is_temp = match curr_block_type {
            BlockType::Func => false,
            BlockType::Loop => true,
            BlockType::IfElse => true
        };
        let max_idx = if statements.len() > 0 { statements.len() - 1 } else { 0 };
        for (idx, statement) in statements.iter_mut().enumerate() {
            match statement {
                Statement::Let(let_tuple) => {
                    let (var, marked_type, expr, line) = let_tuple.deref_mut();
                    match marked_type {
                        None => {
                            // 未标记变量类型 without type mark
                            let deduced_type = self.deduce_type(expr, context)?;
                            let basic_type = deduced_type.as_basic();
                            // 检查变量名是否重复 check if the variable name occupied
                            let (slot_idx, sub_idx) = context.indexer.put(deduced_type);
                            match context.symbol_table.entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx, true)),
                                Entry::Occupied(_) => return Result::Err(AnalysisError::SymbolAlreadyOccupied {
                                    info: context.info(),
                                    symbol: var.name().deref().clone()
                                }),
                            };
                            if var_is_temp {
                                temp_var_table.push(var.name().deref().clone());
                            }
                            *var = Var::new_local(slot_idx, sub_idx, basic_type);
                        }
                        Some(data_type) => {
                            // 已标记变量类型 with type mark
                            let data_type = self.get_type(data_type, context.file_index);
                            let basic_type = data_type.as_basic();
                            let expr_type = self.deduce_type(expr, context)?;
                            if !expr_type.belong_to(&data_type) {
                                return Result::Err(AnalysisError::VarDeclMismatchedType {
                                    info: context.info(),
                                    var: var.name().deref().clone(),
                                    expect: data_type,
                                    found: expr_type
                                })
                            }
                            let (slot_idx, sub_idx) = context.indexer.put(data_type);
                            // 检查变量名是否重复 check if the variable name occupied
                            match context.symbol_table.entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx, true)),
                                Entry::Occupied(_) =>  return Result::Err(AnalysisError::SymbolAlreadyOccupied {
                                    info: context.info(),
                                    symbol: var.name().deref().clone()
                                }),
                            };
                            if var_is_temp {
                                temp_var_table.push(var.name().deref().clone());
                            }
                            *var = Var::new_local(slot_idx, sub_idx, basic_type);
                        }
                    }
                }
                Statement::LeftValueOp(left) => {
                    self.handle_left_value_op(context, left)?;
                }
                Statement::Expr(expr, _) => {
                    let expr_type = self.deduce_type(expr, context)?;
                    if max_idx == idx {
                        last_is_expr = true;
                        let is_void = expr_type.is_none();
                        if !is_void {
                            last_type = ReturnType::Have(expr_type)
                        };
                    }
                }
                Statement::Discard(expr, line) => {
                    self.deduce_type(expr, context)?;
                    if max_idx == idx {
                        match curr_block_type {
                            BlockType::Func => {
                                if context.func_return_type.is_void() {} else {
                                    return Result::Err(AnalysisError::MismatchedReturnType {
                                        info: context.info(),
                                        expect: context.func_return_type.clone(),
                                        found: ReturnType::Void
                                    })
                                }
                            }
                            BlockType::IfElse => {
                                match context.break_stack.last().unwrap() {
                                    BreakType::Type(data_type) => {
                                        return Result::Err(AnalysisError::MismatchedIfElseResultType {
                                            info: context.info(),
                                            expect: ReturnType::Have(data_type.clone()),
                                            found: ReturnType::Void
                                        })
                                    }
                                    // nothing to do
                                    BreakType::Uninit => {}
                                    BreakType::Void => {}
                                }
                            }
                            BlockType::Loop => {}
                        }
                    }
                }
                Statement::Break(line) => {
                    if context.block_stack.iter().rfind(|block| {
                        if let BlockType::Loop = block { true } else { false }
                    }).is_none() {
                        return Result::Err(AnalysisError::UnexpectBreak {
                            info: context.info(),
                            line: *line
                        })
                    }
                }
                Statement::Return(expr, line) => {
                    context.expr_stack.push((SyntaxType::Return, *line));
                    let data_type = self.deduce_type(expr, context)?;
                    match &context.func_return_type {
                        ReturnType::Have(return_type) => {
                            if !data_type.belong_to(return_type) {
                                return Result::Err(AnalysisError::MismatchedReturnType {
                                    info: context.info(),
                                    expect: ReturnType::Have(return_type.clone()),
                                    found: ReturnType::Have(data_type.clone())
                                })
                            }
                        }
                        ReturnType::Void => {
                            if !data_type.is_none() {
                                return Result::Err(AnalysisError::MismatchedReturnType {
                                    info: context.info(),
                                    expect: ReturnType::Void,
                                    found: ReturnType::Have(data_type.clone())
                                })
                            }
                        }
                    }
                    context.expr_stack.pop();
                }
                Statement::Continue(line) => {
                    if context.block_stack.iter().rfind(|block| {
                        if let BlockType::Loop = block { true } else { false }
                    }).is_none() {
                        return Result::Err(AnalysisError::UnexpectContinue {
                            info: context.info(),
                            line: *line
                        })
                    }
                }
                Statement::Static(static_tuple) => {
                    if let BlockType::Func = curr_block_type {} else {
                        return Result::Err(AnalysisError::StaticInLoopOfIfElse {
                            info: context.info(),
                            line: 0
                        })
                    }
                    let (var, parsed_type, expr) = static_tuple.deref_mut();
                    match parsed_type {
                        Some(parsed_type) => {
                            let marked_type = self.get_type(parsed_type, context.file_index);
                            let basic_type = marked_type.as_basic();
                            let expr_type = self.deduce_type(expr, context)?;
                            if !expr_type.belong_to(&marked_type) {
                                return Result::Err(AnalysisError::VarDeclMismatchedType {
                                    info: context.info(),
                                    var: var.name().deref().clone(),
                                    expect: marked_type,
                                    found: expr_type
                                })
                            }
                            let (slot_idx, sub_idx) = self.static_indexer.inner_mut().put(marked_type);
                            match context.symbol_table.entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx, false)),
                                Entry::Occupied(_) => return Result::Err(AnalysisError::SymbolAlreadyOccupied {
                                    info: context.info(),
                                    symbol: var.name().deref().clone()
                                })
                            };
                            *var = Var::new_static(slot_idx, sub_idx, basic_type);
                        }
                        None => {
                            let expr_type = self.deduce_type(expr, context)?;
                            let basic_type = expr_type.as_basic();
                            let (slot_idx, sub_idx) = self.static_indexer.inner_mut().put(expr_type);
                            match context.symbol_table.entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx, false)),
                                Entry::Occupied(_) => return Result::Err(AnalysisError::SymbolAlreadyOccupied {
                                    info: context.info(),
                                    symbol: var.name().deref().clone()
                                })
                            };
                            *var = Var::new_static(slot_idx, sub_idx, basic_type);
                        }
                    };
                }
                Statement::PubStatic(static_tuple) => {
                    if let BlockType::Func = curr_block_type {} else {
                        return Result::Err(AnalysisError::StaticInLoopOfIfElse {
                            info: context.info(),
                            line: 0
                        })
                    }
                    let (var, parsed_type, expr) = static_tuple.deref_mut();
                    let pub_static_symbol_table = self.static_map.clone();
                    match parsed_type {
                        Some(parsed_type) => {
                            let marked_type = self.get_type(parsed_type, context.file_index);
                            let basic_type = marked_type.as_basic();
                            let expr_type = self.deduce_type(expr, context)?;
                            if !expr_type.belong_to(&marked_type) {
                                return Result::Err(AnalysisError::VarDeclMismatchedType {
                                    info: context.info(),
                                    var: var.name().deref().clone(),
                                    expect: marked_type,
                                    found: expr_type
                                })
                            }
                            let (slot_idx, sub_idx) = self.static_indexer.inner_mut().put(marked_type);
                            match pub_static_symbol_table.inner_mut().entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx)),
                                Entry::Occupied(_) => return Result::Err(AnalysisError::SymbolAlreadyOccupied {
                                    info: context.info(),
                                    symbol: var.name().deref().clone()
                                })
                            };
                            *var = Var::new_static(slot_idx, sub_idx, basic_type);
                        }
                        None => {
                            let expr_type = self.deduce_type(expr, context)?;
                            let basic_type = expr_type.as_basic();
                            let (slot_idx, sub_idx) = self.static_indexer.inner_mut().put(expr_type);
                            match pub_static_symbol_table.inner_mut().entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx, sub_idx)),
                                Entry::Occupied(_) => return Result::Err(AnalysisError::SymbolAlreadyOccupied {
                                    info: context.info(),
                                    symbol: var.name().deref().clone()
                                })
                            };
                            *var = Var::new_static(slot_idx, sub_idx, basic_type);
                        }
                    };
                }
                Statement::While(while_loop) => {
                    self.analysis_while(while_loop.deref_mut(), context)?;
                }
                Statement::For(for_loop) => {
                    self.analysis_for(for_loop.deref_mut(), context)?;
                }
                stmt => {
                    return Result::Err(AnalysisError::UnsupportedSyntax(format!("{:?}",stmt)))
                }
            }
        }
        if last_is_expr {
            match curr_block_type {
                BlockType::Func => {
                    if last_type.eq(&context.func_return_type) {
                        let last_statement = statements.last_mut().unwrap();
                        let return_statement = if let Statement::Expr(expr, line) = last_statement {
                            Statement::Return(
                                std::mem::replace(expr, Expression::None),
                                *line,
                            )
                        } else {
                            panic!()
                        };
                        *last_statement = return_statement;
                    } else if context.func_return_type.is_void() {
                        // nothing to do
                    } else {
                        panic!()
                    }
                }
                BlockType::Loop => {
                    // nothing to do
                }
                BlockType::IfElse => {
                    let last_statement = statements.last_mut().unwrap();
                    match context.break_stack.last_mut().unwrap() {
                        BreakType::Void => {
                            // nothing to do
                        }
                        BreakType::Type(already_type) => {
                            if already_type.is_none() && last_type.is_void() {
                                // nothing to do
                            } else if last_type.data_type().belong_to(already_type) {
                                let result_statement = if let Statement::Expr(expr, line) = last_statement {
                                    Statement::IfResult(
                                        std::mem::replace(expr, Expression::None),
                                        *line,
                                    )
                                } else {
                                    panic!()
                                };
                                *last_statement = result_statement;
                            } else if already_type.belong_to(last_type.data_type()) {
                                *already_type = last_type.data_type().clone();
                                let result_statement = if let Statement::Expr(expr, line) = last_statement {
                                    Statement::IfResult(
                                        std::mem::replace(expr, Expression::None),
                                        *line,
                                    )
                                } else {
                                    panic!()
                                };
                                *last_statement = result_statement;
                            }
                        }
                        // BreakType::UnInit
                        break_type => {
                            let result_statement = if let Statement::Expr(expr, line) = last_statement {
                                Statement::IfResult(
                                    std::mem::replace(expr, Expression::None),
                                    *line,
                                )
                            } else {
                                panic!()
                            };
                            *last_statement = result_statement;
                            *break_type = BreakType::Type(last_type.data_type().clone());
                        }
                    }
                }
            }
        }
        if var_is_temp {
            for var_name in temp_var_table.iter() {
                context.symbol_table.remove(var_name.as_str());
            }
        }
        Result::Ok(())
    }

    fn analysis_if_else(&self, if_else: &mut IfElse, context: &mut AnalyzeContext) -> Result<ReturnType,AnalysisError> {
        context.block_stack.push(BlockType::IfElse);
        context.break_stack.push(BreakType::Uninit);
        for (branch_idx, branch) in if_else.branches.iter_mut().enumerate() {
            // 处理每个分支  handle every branch
            let statements = &mut branch.statements;
            let cond_type = self.deduce_type(&mut branch.condition, context)?;
            if !cond_type.is_bool() {
                return Result::Err(AnalysisError::ConditionNotBool {
                    info: context.info(),
                    no: branch_idx,
                    line: branch.line,
                    found: cond_type
                })
            }
            context.expr_stack.push((SyntaxType::IfElseBranch, branch.line));
            context.indexer.enter_sub_block();

            if let Result::Err(err) = self.analysis_statements(context, statements) {
                return Result::Err(err)
            }

            // 处理完一个分支的全部语句 handle all the statements of one branch
            // 清理分支内声明的变量的信息 clear the info of the variables declared in branch
            branch.drop_vec = context.indexer.level_sub_block();
            context.expr_stack.pop();
        }
        context.block_stack.pop();
        Result::Ok(match context.break_stack.pop().unwrap() {
            BreakType::Type(data_type) => {
                if data_type.is_none() {
                    if_else.return_void = true;
                    ReturnType::Void
                } else {
                    ReturnType::Have(data_type)
                }
            }
            BreakType::Uninit => {
                if_else.return_void = true;
                ReturnType::Void
            }
            BreakType::Void => {
                if_else.return_void = true;
                ReturnType::Void
            }
        })
    }

    fn fill_classes(&mut self) {
        let mut index = 0;
        for class in self.status.classes.iter() {
            self.fill_class(class.clone(), index);
            index += 1;
        }
    }
    fn fill_class(&self, class: RefCount<GloomClass>, index: usize) {
        let (parsed_class, file_index) = self.parsed_classes.get(index).unwrap();
        let parsed_class = parsed_class.clone();
        // handle parent class
        if let Option::Some(parent_name) = &parsed_class.inner().parent {
            match self.type_map.get(parent_name.as_str()) {
                None => { panic!("the parent class {} of {} not found", parent_name, parsed_class.inner().name) }
                Some(label) => {
                    if label.tp != MetaType::Class {
                        panic!("declared parent class {} of {} is not a class", parent_name, class.inner().name)
                    }
                    if label.is_public || label.file_index == *file_index {
                        let parent_class = self.status.classes.get(label.index as usize).unwrap().clone();
                        if parent_class.inner().len() == 0 {
                            // means parent class is not uninitialized, fill it recursively
                            self.fill_class(parent_class.clone(), label.index as usize);
                        }
                        class.inner_mut().set_parent(parent_class);
                    } else {
                        panic!("the parent class {} of {} is not public", parent_name, parsed_class.inner().name)
                    }
                }
            }
        }
        // fill fields
        for (is_pub, parsed_type, name) in parsed_class.inner().fields.iter() {
            class.inner_mut().add_field(
                *is_pub,
                name.deref().clone(),
                self.get_type(parsed_type, *file_index),
            );
        }
        // fill funcs
        for (is_pub, name, func) in parsed_class.inner_mut().funcs.iter_mut() {
            let mut params = Vec::with_capacity(func.params.len());
            for (param_name, parsed_type) in func.params.iter() {
                params.push(Param::new(
                    param_name.clone(),
                    self.get_type(parsed_type, *file_index),
                ));
            }
            let return_type: ReturnType = match &func.return_type {
                None => ReturnType::Void,
                Some(parsed_type) => ReturnType::Have(self.get_type(parsed_type, *file_index)),
            };
            // 在不需要move ParsedFunc 的情况下，仅使用ParsedFunc的可变引用将函数体的Vec<Statement> move至status中的GloomClass中
            let body: Vec<Statement> = std::mem::replace(&mut func.body, Vec::with_capacity(0));
            class.inner_mut().add_func(*is_pub, name.clone(), params, return_type, body);
        }
        // handle instance funcs
        class.inner().handle_instance_func(class.clone());
        // handle implemented interface
        for interface_name in parsed_class.inner().impl_interfaces.iter() {
            match self.type_map.get(interface_name.as_str()) {
                None => panic!("interface {} that implemented by class {} is not found",
                               interface_name, class.inner().name),
                Some(label) => {
                    if label.tp != MetaType::Interface {
                        panic!("interface {} that implemented by class {} is in fact not an interface but a {}",
                               interface_name, class.inner().name, label.tp)
                    }
                    if label.is_public || label.file_index == *file_index {
                        let interface = self.status.interfaces.get(label.index as usize).unwrap().clone();
                        class.inner_mut().add_impl(interface);
                    } else {
                        panic!("interface {} that implemented by class {} is not public",
                               interface_name, class.inner().name)
                    }
                }
            }
        }
    }

    fn fill_enums(&mut self) {
        let mut index = 0;
        for enum_class in self.status.enums.iter() {
            self.fill_enum(enum_class.clone(), index);
            index += 1;
        }
    }
    fn fill_enum(&self, enum_class: RefCount<GloomEnumClass>, index: usize) {
        let (parsed_enum, file_index) = self.parsed_enums.get(index).unwrap();
        let parsed_enum = parsed_enum.clone();
        for (name, parsed_type) in parsed_enum.inner().values.iter() {
            let related_type: Option<DataType> = match parsed_type {
                None => Option::None,
                Some(parsed_type) => Some(self.get_type(parsed_type, *file_index))
            };
            enum_class.inner_mut().add_enum_value(name.deref().clone(), related_type);
        }
        for (func_name, is_pub, func) in parsed_enum.inner_mut().funcs.iter_mut() {
            let mut params = Vec::with_capacity(func.params.len());
            for (name, parsed_type) in func.params.iter() {
                params.push(Param::new(
                    name.clone(),
                    self.get_type(parsed_type, *file_index),
                ));
            }
            let return_type = match func.return_type.borrow() {
                None => ReturnType::Void,
                Some(parsed_type) => ReturnType::Have(self.get_type(parsed_type, *file_index))
            };
            let body = std::mem::replace(&mut func.body, Vec::with_capacity(0));
            enum_class.inner_mut().add_func(func_name.clone(), *is_pub, params, return_type, body);
        }
        enum_class.inner_mut().handle_instance_func(enum_class.clone())
    }

    fn analysis_interfaces(&mut self) {
        let mut index = 0;
        for interface in self.status.interfaces.iter() {
            self.analysis_interface(interface.clone(), index);
            index += 1;
        }
    }
    fn analysis_interface(&self, interface: RefCount<Interface>, index: usize) {
        let (parsed_interface, file_index) = self.parsed_interfaces.get(index).unwrap();
        for parent_name in parsed_interface.parents.iter() {
            match self.type_map.get(parent_name.as_str()) {
                None => panic!("Parent interface {} or interface {} is not found", parent_name, interface.inner().name),
                Some(label) => {
                    if label.tp != MetaType::Interface {
                        panic!("declared parent interface {} of {} is not interface", parent_name, interface.inner().name)
                    }
                    let parent_interface = self.status.interfaces.get(label.index as usize).unwrap();
                    if parent_interface.inner().len() == 0 {
                        self.analysis_interface(parent_interface.clone(), label.index as usize);
                    }
                    interface.inner_mut().add_parent(&interface, parent_interface);
                }
            }
        }
        for (name, param_types, return_type) in parsed_interface.funcs.iter() {
            let mut param_data_types = Vec::with_capacity(param_types.len());
            let mut have_self = false;
            for (idx, parsed_type) in param_types.iter().enumerate() {
                param_data_types.push(if let ParsedType::MySelf = parsed_type {
                    if idx == 0 {
                        have_self = true;
                        DataType::Ref(RefType::Interface(interface.clone()))
                    } else {
                        panic!("wrong {}st parameter 'self' occurs in function {} of Interface {}",
                               idx, name, interface.inner().name)
                    }
                } else {
                    self.get_type(parsed_type, *file_index)
                });
            }
            let return_type = match return_type {
                None => ReturnType::Void,
                Some(parsed_type) => ReturnType::Have(self.get_type(parsed_type, *file_index))
            };
            let index = interface.inner().funcs.len() as u16;
            match interface.inner_mut().map.borrow_mut().entry(name.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(index);
                }
                Entry::Occupied(_) => panic!("function name {} already occupied in interface {}",
                                             name, interface.inner().name)
            }
            interface.inner_mut().add_func(AbstractFunc {
                name: name.clone(),
                param_types: param_data_types,
                return_type,
                have_self,
            });
        }
    }

    fn load_decl(&mut self, script: &mut ParsedFile) {
        let file_index = self.file_count;
        script.index = file_index;
        self.file_count += 1;
        // load empty interface
        for (parsed_inter, is_public) in script.interfaces.iter() {
            let index = self.status.interfaces.len();
            match self.type_map.entry(parsed_inter.name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(TypeIndex::from(index as u16, *is_public, file_index, MetaType::Interface));
                }
                Entry::Occupied(_) => panic!("Type name {} already occupied", parsed_inter.name)
            };
            self.status.interfaces.push(RefCount::new(Interface::new(parsed_inter.name.clone())));
        }
        // load empty class
        for (class, is_pub) in script.classes.iter() {
            let index = self.status.classes.len();
            match self.type_map.entry(class.name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(TypeIndex::from(index as u16, *is_pub, file_index, MetaType::Class));
                }
                Entry::Occupied(_) => panic!("Type name {} already occupied", class.name)
            };
            self.status.classes.push(RefCount::new(GloomClass::new(class.name.clone(), file_index, index as u16)));
        }
        // load empty enum
        for (enum_class, is_pub) in script.enums.iter() {
            let index = self.status.enums.len();
            match self.type_map.entry(enum_class.name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(TypeIndex::from(index as u16, *is_pub, file_index, MetaType::Enum));
                }
                Entry::Occupied(_) => panic!("Type name {} already occupied", enum_class.name)
            };
            self.status.enums.push(RefCount::new(GloomEnumClass::new(enum_class.name.clone(), file_index)));
        }
        for parsed_file in script.imports.iter_mut() {
            self.load_decl(parsed_file);
        }
    }
    fn load(&mut self, script: ParsedFile) {
        let index = script.index;
        self.paths.push(script.path);
        for (class, _) in script.classes.into_iter() {
            self.parsed_classes.push((RefCount::new(class), index));
        }
        for (interface, _) in script.interfaces.into_iter() {
            self.parsed_interfaces.push((interface, index));
        }
        for (enum_class, _) in script.enums.into_iter() {
            self.parsed_enums.push((RefCount::new(enum_class), index));
        }
        for (name, func, is_pub) in script.funcs.into_iter() {
            let index = self.status.funcs.len() as u16;
            let mut params = Vec::with_capacity(func.params.len());
            for (name, parsed_type) in func.params.into_iter() {
                params.push(Param::new(
                    name,
                    self.get_type(&parsed_type, script.index),
                ));
            }
            let return_type = match func.return_type {
                None => ReturnType::Void,
                Some(parsed_type) => ReturnType::Have(self.get_type(&parsed_type, script.index))
            };
            match self.func_map.entry(name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert((index, false, is_pub, script.index));
                }
                Entry::Occupied(_) => panic!("func name {} already occupied", name)
            }
            self.status.funcs.push(RefCount::new(
                GloomFunc::new(
                    name.clone(),
                    index,
                    params,
                    return_type,
                    func.body)
            ));
        }
        // statements
        self.status.script_bodies.push(RefCount::new(ScriptBody::new(
            GloomFunc::new(
                Rc::new(String::from("script body")),
                index,
                Vec::with_capacity(0),
                ReturnType::Void,
                script.statements,
            ), index)));
        for file in script.imports.into_iter() {
            self.load(file);
        }
    }

    // ParsedType -> DataType
    fn get_type(&self, origin_type: &ParsedType, file_index: u16) -> DataType {
        match origin_type {
            ParsedType::Single(single_type) => {
                self.analysis_single_type(single_type, file_index)
            }
            ParsedType::Tuple(tuple) => {
                let mut vec = Vec::with_capacity(tuple.vec.len());
                for parsed_type in tuple.vec.iter() {
                    vec.push(self.get_type(parsed_type, file_index));
                }
                DataType::Ref(RefType::Tuple(Box::new(vec)))
            }
            ParsedType::MySelf => {
                DataType::Ref(RefType::MySelf)
            }
        }
    }
    #[inline]
    fn analysis_single_type(&self, single_type: &SingleType, file_index: u16) -> DataType {
        let generic = match &single_type.generic {
            Some(vec) => {
                let mut types = Vec::with_capacity(vec.len());
                for parsed_type in vec.iter() {
                    types.push(self.get_type(parsed_type, file_index));
                }
                Option::Some(types)
            }
            None => Option::None
        };
        match single_type.name.as_str() {
            "int" => return DataType::Int,
            "num" => return DataType::Num,
            "char" => return DataType::Char,
            "bool" => return DataType::Bool,
            _ => {}
        }
        match self.type_map.get(single_type.name.as_str()) {
            Some(label) => {
                if label.is_public || label.file_index == file_index {
                    match label.tp {
                        MetaType::Interface => DataType::Ref(RefType::Interface(
                            self.status.interfaces.get(label.index as usize).unwrap().clone()
                        )),
                        MetaType::Class => DataType::Ref(RefType::Class(
                            self.status.classes.get(label.index as usize).unwrap().clone()
                        )),
                        MetaType::Enum => DataType::Ref(RefType::Enum(
                            self.status.enums.get(label.index as usize).unwrap().clone()
                        )),
                        MetaType::Builtin => DataType::Ref(self.status.builtin_classes
                            .get(label.index as usize).unwrap()
                            .inner().get_ref_type(generic).unwrap())
                    }
                } else {
                    panic!("{} is not public", single_type.name)
                }
            }
            None => {
                panic!("type '{}' not found", single_type.name)
            }
        }
    }

    pub fn result(self) -> (GloomStatus, StaticTable) {
        let mut indexer = self.static_indexer.inner_mut();
        let static_len = indexer.size();
        let static_drop_vec = indexer.basic_drop_vec();
        let static_table = StaticTable::new(static_len, static_drop_vec);
        (self.status, static_table)
    }

    pub fn new() -> Analyzer {
        Analyzer {
            file_count: 0,
            parsed_interfaces: Vec::new(),
            parsed_classes: Vec::new(),
            parsed_enums: Vec::new(),
            type_map: BuiltinClass::class_map(),
            func_map: BuiltInFuncs::func_map(),
            status: GloomStatus::new(),
            static_map: RefCount::new(HashMap::new()),
            builtin_map: BuiltinClass::builtin_type_map(),
            static_indexer: RefCount::new(SlotIndexer::new()),
            paths: Vec::new()
        }
    }
}

pub struct AnalyzeContext<'a> {
    pub func_name: Rc<String>,
    pub symbol_table: HashMap<String, (u16, u8, IsLocal)>,
    pub file_index: u16,
    pub file_name: &'a str,
    pub out_context: Option<&'a AnalyzeContext<'a>>,
    pub captures: Vec<Capture>,
    pub belonged_type: DeclaredType,
    pub func_return_type: ReturnType,
    pub expr_stack: Vec<(SyntaxType, u16)>,
    pub break_stack: Vec<BreakType>,
    pub indexer: SlotIndexer,
    pub block_stack: Vec<BlockType>,
}

impl<'a> AnalyzeContext<'a> {
    pub fn new(func_name: Rc<String>,
               belonged_type: DeclaredType,
               func_return_type: ReturnType,
               file_index: u16,
               file_path : &'a str,
               out_context: Option<&'a AnalyzeContext>) -> AnalyzeContext<'a> {
        AnalyzeContext {
            func_name,
            file_index,
            belonged_type,
            func_return_type,
            out_context,
            symbol_table: HashMap::new(),
            captures: Vec::new(),
            expr_stack: Vec::new(),
            break_stack: Vec::new(),
            file_name: file_path,
            indexer: SlotIndexer::new(),
            block_stack: Vec::new(),
        }
    }

    pub fn info(&self) -> String {
        // type => func => expr > expr > expr
        let mut info = format!(" {} => ", self.file_name);
        match &self.belonged_type {
            DeclaredType::Class(class) => {
                info = info.add("class ");
                info = info.add(class.inner().name.as_str());
                info = info.add(" => ");
            }
            DeclaredType::Enum(enum_class) => {
                info = info.add("enum ");
                info = info.add(enum_class.inner().name.as_str());
                info = info.add(" => ");
            }
            DeclaredType::Interface(interface) => {
                info = info.add("interface ");
                info = info.add(interface.inner().name.as_str());
                info = info.add(" => ");
            }
            DeclaredType::IsNot => {}
        }
        info = info.add(self.func_name.as_str());
        if self.expr_stack.len() > 0 {
            info = info.add(" > ");
        }

        for (frame_type, line) in self.expr_stack.iter() {
            info = info.add(format!("{:?} line {}", frame_type, line).as_str()).add(" > ");
        }
        info.add("\r\n")
    }
}