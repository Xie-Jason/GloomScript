use std::borrow::{Borrow, BorrowMut};
use std::ops::{Add, Deref, DerefMut};
use std::option::Option::Some;
use std::rc::Rc;
use hashbrown::hash_map::{Entry};
use hashbrown::HashMap;
use crate::{
    exec::static_table::StaticTable,
    builtin::funcs::{BuiltInFuncs, IsBuiltIn},
    builtin::classes::BuiltinClass,
    frontend::{
        ast::*,
        index::SlotIndexer,
        ops::BinOpType,
        script::{ParsedFile, ScriptBody},
        status::{GloomStatus, TypeIndex, MetaType}
    },
    obj::func::{Capture, FuncBody, GloomFunc, Param, ReturnType},
    obj::gloom_class::{GloomClass, IsPub},
    obj::gloom_enum::GloomEnumClass,
    obj::refcount::RefCount,
    obj::interface::{AbstractFunc, Interface},
    obj::types::{BreakType, BuiltinType, DataType, DeclaredType, RefType}
};
use crate::frontend::ops::LeftValueOp;
use crate::obj::func::FuncInfo;

pub struct Analyzer{
    status : GloomStatus,

    file_count : u16,
    // 声明类型的后面的u16是该类型所属的文件索引，用于检查访问权限
    // the u16 after declared type is script file index, used for check the access auth
    parsed_interfaces : Vec<(ParsedInterface,u16)>,
    parsed_classes : Vec<(RefCount<ParsedClass>, u16)>,
    parsed_enums : Vec<(RefCount<ParsedEnum>, u16)>,
    pub func_map : HashMap<String,(u16,IsBuiltIn,IsPub,u16)>, // index is_builtin is_pub file_index
    pub func_file_indexes: Vec<u16>,
    pub type_map : HashMap<String, TypeIndex>,
    pub static_map : RefCount<HashMap<String,(u16,u8)>>,
    builtin_map : HashMap<BuiltinType,u16>,
    static_indexer : RefCount<SlotIndexer>,
}
// 这些字段被存储到Analyzer而非GloomStatus中，这意味着我想要它们在运行前被丢弃。
// these fields are stored in Analyzer rather than GloomStatus, because I want to discard them before execution
type IsLocal = bool;
impl Analyzer {
    pub fn analysis(&mut self, mut script : ParsedFile, debug : bool){
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
                self.analysis_func(&mut *func_ref,file_index,Option::None,DeclaredType::Class(class.clone()));
            }
        }
        // function in enums
        for enum_class in self.status.enums.iter() {
            let enum_class = enum_class.clone();
            let file_index = enum_class.inner().file_index;
            for func in enum_class.inner().funcs.iter(){
                let func = func.clone();
                let mut func_ref = func.inner_mut();
                self.analysis_func(&mut *func_ref,file_index,Option::None,DeclaredType::Enum(enum_class.clone()));
            }
        }
        // functions that declared directly
        for func in self.status.funcs.iter() {
            let func = func.clone();
            let mut func_ref = func.inner_mut();
            let file_index = func_ref.info.file_index;
            self.analysis_func(&mut *func_ref,file_index,Option::None,DeclaredType::IsNot);
        }
        // script executable body
        for script_body in self.status.script_bodies.iter() {
            let file_index = script_body.inner().file_index;
            let script_body_rc = script_body.clone();
            let mut script_body_ref = script_body_rc.inner_mut();
            self.analysis_func(&mut script_body_ref.func,file_index,Option::None,DeclaredType::IsNot);
        }
        if debug {
            println!("{:?}",self.status)
        }
    }

    fn analysis_func(&self, func : &mut GloomFunc, file_index : u16, out_env : Option<&AnalyzeContext>, belonged_type : DeclaredType) {
        let params = &mut func.info.params;
        let func_return_type = &func.info.return_type;
        let mut context = AnalyzeContext::new(func.info.name.clone(),belonged_type,func_return_type.clone(),file_index,out_env);
        // load param into symbol table and allocate local slot for parameters
        for param in params.iter_mut() {
            let param_name = &param.name;
            let param_type = &param.data_type;
            let (slot_idx,sub_idx) = context.indexer.put(param_type.clone());
            param.index = (slot_idx,sub_idx);
            match context.symbol_table.entry(param_name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert((slot_idx,sub_idx,true));
                }
                Entry::Occupied(_) => panic!("{} variable name {} already occupied",context.info(),param_name)
            };
        }
        // analysis per statement
        let body = &mut func.body;
        if let FuncBody::Gloom(body) = body {
            let stat_max_index = if body.len() > 0 { body.len() - 1 } else { 0 };
            let mut temp_expr = Expression::None;
            let mut temp_line = 0;
            let mut expr_to_return = false;
            for (stat_index,statement) in body.iter_mut().enumerate() {
                match statement {
                    // 声明局部变量 declare local variable
                    Statement::Let(let_tuple) => {
                        let (var,marked_type,expr,line) = let_tuple.deref_mut();
                        context.expr_stack.push((SyntaxType::Let,*line));
                        match marked_type {
                            None => {
                                // 未标记变量类型 without type mark
                                let deduced_type = self.deduce_type(expr,&mut context);
                                let basic_type = deduced_type.as_basic();
                                // 检查变量名是否重复 check if the variable name occupied
                                let (slot_idx,sub_idx) = context.indexer.put(deduced_type);
                                match context.symbol_table.entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx,true)),
                                    Entry::Occupied(_) => panic!("{} line {}, variable name {} occupied",context.info(),line,var.name().deref()),
                                };
                                *var = Var::new_local(slot_idx, sub_idx, basic_type);
                            }
                            Some(data_type) => {
                                // 已标记变量类型 with type mark
                                let data_type = self.get_type(data_type, file_index);
                                if data_type.is_queue() && expr.is_array_literal() {
                                    if let Expression::Array(array) = expr {
                                        let (_,_,is_queue) = array.deref_mut();
                                        *is_queue = true;
                                    }
                                }
                                let basic_type = data_type.as_basic();
                                if ! self.deduce_type(expr,&mut context).belong_to(&data_type){
                                    panic!()
                                }
                                let (slot_idx,sub_idx) = context.indexer.put(data_type);
                                // 检查变量名是否重复 check if the variable name occupied
                                match context.symbol_table.entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx,true)),
                                    Entry::Occupied(_) => panic!("{} line {}, variable name {} occupied",context.info(),line,var.name().deref()),
                                };
                                *var = Var::new_local(slot_idx, sub_idx, basic_type);
                            }
                        }
                        context.expr_stack.pop();
                    }
                    Statement::Static(static_tuple) => {
                        let (var,parsed_type,expr) = static_tuple.deref_mut();
                        match parsed_type {
                            Some(parsed_type) => {
                                let marked_type = self.get_type(parsed_type, file_index);
                                let basic_type = marked_type.as_basic();
                                let expr_type = self.deduce_type(expr,&mut context);
                                if ! expr_type.belong_to(&marked_type) {
                                    panic!("{} the expression's type {} do not belongs to marked type {} when comes to static variable {} declare",
                                           context.info(),expr_type,marked_type,var.name().deref())
                                }
                                let (slot_idx,sub_idx)  = self.static_indexer.inner_mut().put(marked_type);
                                match context.symbol_table.entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx,false)),
                                    Entry::Occupied(_) => panic!("{} variable name {} occupied",context.info(),var.name().deref())
                                };
                                *var = Var::new_static(slot_idx, sub_idx, basic_type);
                            }
                            None => {
                                let expr_type = self.deduce_type(expr,&mut context);
                                let basic_type = expr_type.as_basic();
                                let (slot_idx,sub_idx) = self.static_indexer.inner_mut().put(expr_type);
                                match context.symbol_table.entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx,false)),
                                    Entry::Occupied(_) => panic!("{} variable name {} occupied",context.info(),var.name().deref())
                                };
                                *var = Var::new_static(slot_idx, sub_idx, basic_type);
                            }
                        };
                    }
                    Statement::PubStatic(static_tuple) => {
                        let (var,parsed_type,expr) = static_tuple.deref_mut();
                        let pub_static_symbol_table = self.static_map.clone();
                        match parsed_type {
                            Some(parsed_type) => {
                                let marked_type = self.get_type(parsed_type, file_index);
                                let basic_type = marked_type.as_basic();
                                let expr_type = self.deduce_type(expr,&mut context);
                                if ! expr_type.belong_to(&marked_type) {
                                    panic!("{} the expression's type {} do not belongs to marked type {} when comes to static variable {} declare",
                                           context.info(),expr_type,marked_type,var.name().deref())
                                }
                                let (slot_idx,sub_idx)  = self.static_indexer.inner_mut().put(marked_type);
                                match pub_static_symbol_table.inner_mut().entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx)),
                                    Entry::Occupied(_) => panic!("{} variable name {} occupied",context.info(),var.name().deref())
                                };
                                *var = Var::new_static(slot_idx, sub_idx, basic_type);
                            }
                            None => {
                                let expr_type = self.deduce_type(expr,&mut context);
                                let basic_type = expr_type.as_basic();
                                let (slot_idx,sub_idx)  = self.static_indexer.inner_mut().put(expr_type);
                                match pub_static_symbol_table.inner_mut().entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx)),
                                    Entry::Occupied(_) => panic!("{} variable name {} occupied",context.info(),var.name().deref())
                                };
                                *var = Var::new_static(slot_idx, sub_idx, basic_type);
                            }
                        };
                    }
                    Statement::LeftValueOp(left_val_tuple) => {
                        self.handle_left_value_op(&mut context,left_val_tuple);
                    }
                    Statement::Discard(expr,line) => {
                        context.expr_stack.push((SyntaxType::Discard,*line));
                        self.deduce_type(expr,&mut context);
                        context.expr_stack.pop();
                    }
                    Statement::Expr(expr,line) => {
                        context.expr_stack.push((SyntaxType::Expr,*line));
                        let expr_type = self.deduce_type(expr, &mut context);
                        if stat_index == stat_max_index {
                            if let ReturnType::Have(data_type) = &*func_return_type {
                                // need to be returned expression, need assert its type as return type
                                if ! data_type.belong_to(&expr_type) {
                                    panic!("{} line {}, mismatched return type, expect {} found {}",
                                           context.info(),line,data_type,expr_type)
                                }
                            }else{
                                // void
                                if ! expr_type.is_none() {
                                    panic!("{} line {}, mismatched return type, expect void found {}",
                                           context.info(),line,expr_type)
                                }
                            }
                            expr_to_return = true;
                            temp_line = *line;
                            temp_expr = std::mem::replace(
                                expr,
                                Expression::None
                            );
                        }
                        context.expr_stack.pop();
                    }
                    Statement::Return(expr,line) => {
                        context.expr_stack.push((SyntaxType::Return,*line));
                        match &*func_return_type {
                            ReturnType::Void => panic!("{} function have no return value, line {}",context.info(),line),
                            ReturnType::Have(data_type) => {
                                let expr_type = self.deduce_type(expr, &mut context);
                                if ! expr_type.belong_to(data_type) {
                                    panic!("{} mismatched return type, expect {} found {}, line {}",
                                           context.info(),data_type,expr_type,line)
                                }
                            }
                        }
                        context.expr_stack.pop();
                    }

                    Statement::Continue(line) => panic!("{} unexpected continue statement in function body, line {}",context.info(),line),
                    Statement::Break(_,line) => panic!("{} unexpected break statement in function body, line {}",context.info(),line),
                    _ => {}
                }
                if expr_to_return {
                    *statement = Statement::Return(
                        std::mem::replace(&mut temp_expr,Expression::None),
                        temp_line
                    );
                }
                expr_to_return = false;
            }
        }
        func.info.captures = context.captures;
        func.info.local_size = context.indexer.size();
        func.info.drop_slots = context.indexer.basic_drop_vec();
    }

    #[inline]
    fn handle_left_value_op(&self, context : &mut AnalyzeContext, left_val_tuple: &mut Box<(LeftValue, LeftValueOp)>) -> DataType{
        let (left_val, left_val_op) = left_val_tuple.deref_mut();
        let left_val_type = match left_val {
            LeftValue::Var(var) => {
                let var_name_ref = var.name().clone();
                match context.symbol_table.get(var_name_ref.as_str()) {
                    Some((slot_idx, sub_idx, is_local)) => {
                        if *is_local {
                            let data_type = context.indexer.get_type(*slot_idx).clone();
                            *var = Var::new_local(*slot_idx,*sub_idx,data_type.as_basic());
                            data_type
                        }else{
                            let static_indexer = self.static_indexer.inner();
                            let data_type = static_indexer.get_type(*slot_idx).clone();
                            *var = Var::new_static(*slot_idx,*sub_idx, data_type.as_basic());
                            data_type
                        }
                    }
                    None => panic!("{} unknown variable {}", context.info(), var_name_ref)
                }
            }
            LeftValue::Chain(_,_) => {
                DataType::Int
            }
        };
        match left_val_op {
            LeftValueOp::Assign(expr) => {
                let expr_type = self.deduce_type(expr,context);
                if ! expr_type.belong_to(&left_val_type) {
                    panic!("{} mismatched type of assign, expression type {} do not belongs to left value type {}",
                           context.info(),expr_type,left_val_type)
                }
                expr_type
            }
            LeftValueOp::PlusEq(expr) => {
                if ! left_val_type.is_int_or_num(){
                    panic!("{} the left of operator '+=' should be a int or num, found {}",
                           context.info(),left_val_type)
                }
                let expr_type = self.deduce_type(expr,context);
                if ! expr_type.is_int_or_num() {
                    panic!("{} the right of operator '+=' should be a int or num, found {}",
                           context.info(),expr_type)
                }
                left_val_type
            }
            LeftValueOp::SubEq(expr) => {
                if ! left_val_type.is_int_or_num(){
                    panic!("{} the left of operator '-=' should be a int or num, found {}",
                           context.info(),left_val_type)
                }
                let expr_type = self.deduce_type(expr,context);
                if ! expr_type.is_int_or_num() {
                    panic!("{} the right of operator '-=' should be a int or num, found {}",
                           context.info(),expr_type)
                }
                left_val_type
            }
            LeftValueOp::PlusOne => {
                if ! left_val_type.is_int_or_num(){
                    panic!("{} the left of operator '++' should be a int or num, found {}",
                           context.info(),left_val_type)
                }
                left_val_type
            }
            LeftValueOp::SubOne => {
                if ! left_val_type.is_int_or_num(){
                    panic!("{} the left of operator '--' should be a int or num, found {}",
                           context.info(),left_val_type)
                }
                left_val_type
            }
        }
    }

    #[inline]
    fn handle_chains(&self, context : &mut AnalyzeContext, chains: &mut Box<(Expression, Vec<Chain>)>) -> DataType{
        let (expr,chain_vec) = chains.deref_mut();
        let mut expr_type = self.deduce_type(expr,context);
        let mut new_type = DataType::Ref(RefType::None);
        let chains_len = chain_vec.len();
        for (chain_idx,chain) in chain_vec.iter_mut().enumerate() {
            match chain {
                Chain::Access(field,basic_type) => {
                    let field_name = field.name();
                    match &expr_type {
                        // find field
                        DataType::Ref(RefType::Class(class)) => {
                            match class.inner().map.get(field_name.as_str()) {
                                Some((slot_idx,sub_idx,is_pub,is_mem_func)) => {
                                    if ! *is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                        *field = VarId::Index(*slot_idx,*sub_idx);
                                        new_type = class.inner().field_indexer.get_type(*slot_idx).clone();
                                        *basic_type = new_type.as_basic();
                                    }else{
                                        panic!("{} the '{}' of class {}  is not a field or not public ",
                                               context.info(),field_name,class.inner().name)
                                    }
                                }
                                None => panic!("{} class {} have no field {}",context.info(),class.inner().name,field_name)
                            };
                        }
                        // find function
                        DataType::Ref(RefType::MetaClass(class)) => {
                            match class.inner().map.get(field_name.as_str()) {
                                Some((index,_,is_pub,is_mem_func)) => {
                                    if *is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                        *field = VarId::Index(*index,0);
                                        new_type = class.inner().funcs.get(*index as usize).unwrap().inner().get_type();
                                    }else{
                                        panic!("{} the '{}' of class {} is not a function or not public ",
                                               context.info(),field_name,class.inner().name)
                                    }
                                }
                                None => panic!("{} class {} have no member function {}",context.info(),class.inner().name,field_name)
                            }
                        }
                        DataType::Ref(RefType::MetaEnum(class)) => {
                            match class.inner().func_map.get(field_name.as_str()) {
                                Some((index,is_pub)) => {
                                    if *is_pub || context.belonged_type.equal_enum(class) {
                                        *field = VarId::Index(*index,0);
                                        new_type = class.inner().funcs.get(*index as usize).unwrap().inner().get_type();
                                    }else{
                                        panic!("{} the function '{}' of enum {} is not public ",
                                               context.info(),field_name,class.inner().name)
                                    }
                                }
                                None => panic!("{} enum {} have no member function {}",context.info(),class.inner().name,field_name)
                            }
                        }
                        DataType::Ref(RefType::MetaInterface(class)) => {
                            match class.inner().map.get(field_name.deref()) {
                                Some(index) => {
                                    *field = VarId::Index(*index,0);
                                    new_type = class.inner().funcs.get(*index as usize).unwrap().func_type();
                                }
                                None => panic!("{} interface {} have no function {}",
                                               context.info(),class.inner().name,field_name)
                            }
                        }
                        DataType::Ref(RefType::MataBuiltinType(builtin_type)) => {
                            match self.builtin_map.get(builtin_type) {
                                Some(index) => {
                                    *field = VarId::Index(*index,0);
                                    new_type = DataType::Ref(self.status.builtin_classes
                                        .get(*index as usize).unwrap().inner().get_ref_type(Option::None).unwrap());
                                }
                                None => panic!("you may forgot import builtin type {:?} in std library",builtin_type)
                            }
                        }
                        other_type => {
                            panic!("{} can't find any field in '{}' type", context.info(),other_type)
                        }
                    }
                }
                Chain::Call(args) => {
                    match &expr_type {
                        DataType::Ref(RefType::Func(func_type)) => {
                            let (param_types,return_type,_) = func_type.deref();
                            let in_fact_len = args.len();
                            for (arg_idx,arg) in args.iter_mut().enumerate() {
                                let arg_type = self.deduce_type(arg, context);
                                let param_type = param_types.get(arg_idx)
                                    .expect(format!("{} mismatch arguments num of function {} call, expect {} arguments, found {}",
                                                    context.info(),expr_type,param_types.len(),in_fact_len).as_str());
                                if ! arg_type.belong_to(param_type){
                                    panic!("{} mismatch argument type of {}st argument when call function {}, expect {} found {}",
                                           context.info(), arg_idx +1,expr_type,param_type, arg_type)
                                }
                            }
                            match return_type {
                                ReturnType::Void => {
                                    if chain_idx != chains_len-1 {
                                        panic!("{} function {} call return void but some chained operation are followed behind",
                                               context.info(),expr_type)
                                    }
                                }
                                ReturnType::Have(data_type) => {
                                    new_type = data_type.clone();
                                }
                            }
                        }
                        _ => panic!("{} the {} type is not a func type",context.info(),expr_type)
                    }
                }
                Chain::FnCall {
                    func,
                    need_self,
                    args
                } => {
                    let func_name = func.name();
                    let function : RefCount<GloomFunc>;
                    match &expr_type {
                        DataType::Ref(ref_type) => {
                            match ref_type {
                                // caller is object, call member function
                                RefType::Class(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.map.get(func_name.as_str()) {
                                        Some((index,sub_idx,is_pub,is_mem_func)) => {
                                            if *is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                if ! target_func.info.need_self {
                                                    panic!("{} function {} of class {} is not a non-static function, which don't have 'self' as first parameter",
                                                           context.info(),func_name,class_ref.name)
                                                }
                                                *need_self = true;
                                                *func = VarId::Index(*index,*sub_idx);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len-1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(),func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            }else{
                                                panic!("{} the '{}' of class {}  is not a function or not public ",
                                                       context.info(),func_name,class_ref.name)
                                            }
                                        }
                                        None => panic!("{} class {} have no function {}",context.info(),class_ref.name,func_name)
                                    };
                                }
                                RefType::Enum(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.func_map.get(func_name.as_str()) {
                                        Some((index,is_pub)) => {
                                            if *is_pub || context.belonged_type.equal_enum(class) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                if ! target_func.info.need_self {
                                                    panic!("{} function {} of enum {} is not a non-static function, which don't have 'self' as first parameter",
                                                           context.info(),func_name,class_ref.name)
                                                }
                                                *need_self = true;
                                                *func = VarId::Index(*index,0);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len-1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(),func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            }else{
                                                panic!("{} the function '{}' of enum {} is not public ",
                                                       context.info(),func_name,class_ref.name)
                                            }
                                        }
                                        None => panic!("{} enum {} have no function {}",
                                                       context.info(),class_ref.name,func_name)
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
                                                params.push(Param::new(empty_name.clone(),param_type.clone()));
                                            }
                                            function = RefCount::new(GloomFunc{
                                                info: FuncInfo{
                                                    name: empty_name,
                                                    params,
                                                    return_type: ReturnType::Void,
                                                    captures: Vec::with_capacity(0),
                                                    drop_slots: Vec::with_capacity(0),
                                                    local_size: 0,
                                                    need_self: false,
                                                    file_index: 0
                                                },
                                                body: FuncBody::None
                                            });
                                            if ! target_func.have_self {
                                                panic!("{} function {} of interface {} is not a non-static function, which don't have 'self' as first parameter",
                                                       context.info(),func_name,class_ref.name)
                                            }
                                            *need_self = true;
                                            *func = VarId::Index(*index,0);
                                            match &target_func.return_type {
                                                ReturnType::Void => {
                                                    if chain_idx != chains_len-1 {
                                                        panic!("{} function {} call return void but some chained operation are followed behind",
                                                               context.info(),func_name)
                                                    }
                                                }
                                                ReturnType::Have(return_type) => {
                                                    new_type = return_type.clone();
                                                }
                                            }
                                        }
                                        None => panic!("{} interface {} have no function {}",
                                                       context.info(),class.inner().name,func_name)
                                    }
                                }
                                // caller is type, call member function
                                RefType::MetaClass(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.map.get(func_name.as_str()) {
                                        Some((index,_,is_pub,is_mem_func)) => {
                                            if *is_mem_func && (*is_pub || context.belonged_type.equal_class(class)) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                *func = VarId::Index(*index,0);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len-1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(),func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            }else{
                                                panic!("{} the '{}' of class {}  is not a function or not public ",
                                                       context.info(),func_name,class_ref.name)
                                            }
                                        }
                                        None => panic!("{} class {} have no function {}",context.info(),class_ref.name,func_name)
                                    };
                                }
                                RefType::MetaEnum(class) => {
                                    let class_ref = class.inner();
                                    match class_ref.func_map.get(func_name.as_str()) {
                                        Some((index,is_pub)) => {
                                            if *is_pub || context.belonged_type.equal_enum(class) {
                                                function = class_ref.funcs.get(*index as usize).unwrap().clone();
                                                let target_func = function.inner();
                                                *func = VarId::Index(*index,0);
                                                match &target_func.info.return_type {
                                                    ReturnType::Void => {
                                                        if chain_idx != chains_len-1 {
                                                            panic!("{} function {} call return void but some chained operation are followed behind",
                                                                   context.info(),func_name)
                                                        }
                                                    }
                                                    ReturnType::Have(return_type) => {
                                                        new_type = return_type.clone();
                                                    }
                                                }
                                            }else{
                                                panic!("{} the function '{}' of enum {} is not public ",
                                                       context.info(),func_name,class_ref.name)
                                            }
                                        }
                                        None => panic!("{} enum {} have no function {}",
                                                       context.info(),class_ref.name,func_name)
                                    }
                                }
                                RefType::MetaInterface(class) => {
                                    panic!("could not access the member function of Interface {}",class.inner())
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
                                                    *func = VarId::Index(*index,0);
                                                    match &target_func.info.return_type {
                                                        ReturnType::Void => {
                                                            if chain_idx != chains_len-1 {
                                                                panic!("{} function {} call return void but some chained operation are followed behind",
                                                                       context.info(),func_name)
                                                            }
                                                        }
                                                        ReturnType::Have(return_type) => {
                                                            new_type = return_type.clone();
                                                        }
                                                    }
                                                }
                                                None => {
                                                    panic!("{} not found function {} in builtin type {}",
                                                           context.info(),func_name,builtin_type.to_str())
                                                }
                                            }
                                        }
                                        None => {
                                            panic!("you may forgot import builtin type {:?} in std library",builtin_type)
                                        }
                                    }
                                }

                                RefType::Any | RefType::None | RefType::MySelf  => {
                                    panic!()
                                }
                                // non-static function
                                builtin_type => {
                                    let builtin_type = builtin_type.as_built_type();
                                    match self.builtin_map.get(&builtin_type) {
                                        Some(index) => {
                                            let class = self.status.builtin_classes.get(*index as usize).unwrap();
                                            let class = class.inner();
                                            match class.map.get(func_name.as_str()){
                                                Some(index) => {
                                                    function = class.funcs.get(*index as usize).unwrap().clone();
                                                    let target_func = function.inner();
                                                    if ! target_func.info.need_self {
                                                        panic!("{} function {} of builtin type {} is not a non-static function, which don't have 'self' as first parameter",
                                                               context.info(),func_name,builtin_type.to_str())
                                                    }
                                                    *func = VarId::Index(*index,0);
                                                    *need_self = true;
                                                    match &target_func.info.return_type {
                                                        ReturnType::Void => {
                                                            if chain_idx != chains_len-1 {
                                                                panic!("{} function {} call return void but some chained operation are followed behind",
                                                                       context.info(),func_name)
                                                            }
                                                        }
                                                        ReturnType::Have(return_type) => {
                                                            new_type = return_type.clone();
                                                        }
                                                    }
                                                }
                                                None => {
                                                    panic!("{} not found function {} in builtin type {}",
                                                           context.info(),func_name,builtin_type.to_str())
                                                }
                                            }
                                        }
                                        None => {
                                            panic!("you may forgot import builtin type {:?} in std library",builtin_type)
                                        }
                                    }
                                }
                            }
                        }
                        // call non-static function
                        basic_type => {
                            panic!("basic data type value can't be caller of member function, found {} value as caller call function '{}'",
                                   basic_type,func_name)
                        }
                    }
                    let function = function.inner();
                    let mut param_iter = function.info.params.iter();
                    if *need_self {
                        let self_type = match param_iter.next() {
                            Some(param) => &param.data_type,
                            None => panic!("function {:?} have no parameter but need self argument",function),
                        };
                        if ! expr_type.belong_to(&self_type) {
                            panic!("{} mismatched argument type in first argument 'self' of function {:?} call, expect {}, found {}",
                                   context.info(),function,self_type,expr_type)
                        }
                    }
                    for (idx,(arg_expr,param)) in args.iter_mut().zip(param_iter).enumerate() {
                        let arg_type = self.deduce_type(arg_expr, context);
                        if ! arg_type.belong_to(&param.data_type) {
                            panic!("{} mismatched argument type in {}st argument of function {:?} call, expect {}, found {}",
                                   context.info(),idx,function,param.data_type,arg_type)
                        }
                    }
                }
            };
            expr_type = std::mem::replace(&mut new_type,DataType::Ref(RefType::None));
        }
        expr_type
    }

    fn deduce_type(& self, expr : &mut Expression, context : &mut AnalyzeContext ) -> DataType{
        match expr {
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
                    Some((slot_idx,sub_idx,is_local)) => {
                        if *is_local {
                            // non-static local variable
                            let data_type = context.indexer.get_type(*slot_idx).clone();
                            *var_ref = Var::new_local(*slot_idx,*sub_idx,data_type.as_basic());
                            data_type
                        }else{
                            // local variable
                            let data_type = self.static_indexer.inner().get_type(*slot_idx).clone();
                            *var_ref = Var::new_static(*slot_idx,*sub_idx,data_type.as_basic());
                            data_type
                        }
                    }
                    None => match self.static_map.inner().get(var_name.as_str()) {
                        Some((slot_idx,sub_idx)) => {
                            // public static variable
                            let data_type = self.static_indexer.inner().get_type(*slot_idx).clone();
                            *var_ref = Var::new_static(*slot_idx,*sub_idx,data_type.as_basic());
                            data_type
                        }
                        None => match context.out_context {
                            Some(out_context) => {
                                if let Some((out_slot_idx, out_sub_idx,is_local)) = out_context.symbol_table.get(var_name.as_str()) {
                                    if *is_local {
                                        // 捕获非静态的局部变量 captured non-static local variable
                                        // 记录捕获 插入符号表 record capture, insert into symbol table
                                        let captured_type = out_context.indexer.get_type(*out_slot_idx).clone();
                                        let cap_basic_type = captured_type.as_basic();
                                        let (slot_idx,sub_idx) = context.indexer.put(captured_type.clone());
                                        // 已经尝试通过该名称获取，所以不需要entry api。 try find this name before, so there are not same name variable here
                                        context.symbol_table.insert(var_name.deref().clone(),(slot_idx,sub_idx,true));
                                        context.captures.push(Capture::new(
                                            *out_slot_idx,
                                            *out_sub_idx,
                                            slot_idx,
                                            sub_idx,
                                            cap_basic_type
                                        ));
                                        *var_ref = Var::new_local(slot_idx,sub_idx,cap_basic_type);
                                        captured_type
                                    }else{
                                        // captured static variable
                                        let data_type = self.static_indexer.inner().get_type(*out_slot_idx).clone();
                                        *var_ref = Var::new_static(*out_slot_idx,*out_sub_idx,data_type.as_basic());
                                        data_type
                                    }
                                }else{
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
                            }else{
                                panic!("{} Type {} is not public",context.info(),var_name)
                            }
                        }
                        // function
                        None => {
                            match self.func_map.get(var_name.as_str()) {
                                Some((index,_,is_pub,file_index)) => {
                                    if *is_pub || *file_index == context.file_index {
                                        *var_ref = Var::DirectFn(*index);
                                        let func_type = self.status.funcs.get(*index as usize).unwrap().inner().get_ref_type();
                                        result_type = DataType::Ref(func_type);
                                    }else{
                                        panic!("{} unknown type {}",context.info(),var_name)
                                    }
                                }
                                None => {
                                    panic!("{} unknown type {}",context.info(),var_name)
                                }
                            }
                        }
                    }
                }
                result_type
            }
            Expression::Chain(chains) => self.handle_chains(context, chains),
            Expression::Tuple(tuple) => {
                let vec = tuple.deref_mut();
                let mut tuple_types = Vec::with_capacity(vec.len());
                for expr in vec.iter_mut() {
                    tuple_types.push(self.deduce_type(expr,context));
                }
                DataType::Ref(RefType::Tuple(Box::new(tuple_types)))
            }
            Expression::Array(array) => {
                let (array,basic_type,_) = array.deref_mut();
                if array.len() == 0 {
                    // without any array item
                    DataType::Ref(RefType::Array(Box::new(DataType::Ref(RefType::Any))))
                }else{
                    // array with generic type
                    let mut iter = array.iter_mut();
                    let first_elem = iter.next().unwrap();
                    let mut data_type = self.deduce_type(first_elem, context);
                    if data_type.is_none() {
                        panic!("{} expect a type, found void, in first expression of array literal : {:?}",context.info(),first_elem)
                    }
                    for (idx,expr) in iter.enumerate() {
                        let temp_type = self.deduce_type(expr,context);
                        if temp_type.is_none() {
                            panic!("{} expect a value or object, found void, in {}st expression of array literal : {:?}",context.info(),idx+2,expr)
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
                    _ => panic!("{} the object construction expect a Class as type mark, found {}",
                                context.info(),class_type)
                };
                let class = class_rc.inner();
                if class.field_count as usize != construction.fields.len() {
                    panic!("{} the fields of class have {}, found {} in the construction list",
                           context.info(),class.field_count,construction.fields.len())
                }
                for (var, expr) in construction.fields.iter_mut() {
                    let field_name = var.name();
                    match class.map.get(field_name.as_str()) {
                        Some((slot_idx,sub_idx,is_pub,is_fn)) => {
                            if *is_pub || context.belonged_type.equal_class(class_rc) {
                                if *is_fn {
                                    panic!("{} {} in class {} is a function rather than field",
                                           context.info(),field_name,class_type)
                                }else{
                                    let expr_type = self.deduce_type(expr, context);
                                    let field_type = class.field_indexer.get_type(*slot_idx);
                                    if expr_type.belong_to(field_type) {
                                        *var = VarId::Index(*slot_idx,*sub_idx);
                                    }else{
                                        panic!("{} the field {} of {} need a value/object with {} type, found {}",
                                               context.info(),field_name,class_type,field_type,expr_type)
                                    }
                                }
                            }else{
                                panic!("{} field {} is not public, so you can't construct the object of {} except in class member function",
                                       context.info(),field_name,class_type)
                            }
                        }
                        None => panic!("{} unknown field '{}' in class_rc {}",
                                        context.info(), field_name, class_type)
                    }
                };
                class_type.clone()
            }
            Expression::NegOp(expr) => {
                self.deduce_type(expr.deref_mut(),context)
            }
            Expression::NotOp(expr) => {
                self.deduce_type(expr.deref_mut(),context)
            }
            Expression::Cast(cast) => {
                let (expr,parsed_type, data_type) = cast.deref_mut();
                let cast_type = self.get_type(parsed_type, context.file_index);
                let real_type = self.deduce_type(expr, context);
                *data_type = cast_type.clone();
                if ( cast_type.is_num_liked() && real_type.is_num_liked() )
                    || cast_type.belong_to(&real_type)
                    || real_type.belong_to(&cast_type) {
                    cast_type
                }else {
                    panic!("object type cast error, from {} to {}",real_type,cast_type)
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
                                self.get_type(parsed_type, context.file_index)
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
                self.analysis_func(&mut func,context.file_index ,Option::Some(context),context.belonged_type.clone());
                let func_type = func.get_type();
                let is_parsed = func_expr.is_parsed();
                if is_parsed {
                    *func_expr = FuncExpr::Analysed(RefCount::new(func));
                };
                func_type
            }
            Expression::BinaryOp(bin_op) => {
                let bin_op = bin_op.deref_mut();
                let mut left_type = self.deduce_type( &mut bin_op.left,context);
                for (op,expr) in bin_op.vec.iter_mut() {
                    match op.to_type() {
                        BinOpType::Calculate => {
                            // number to number
                            if left_type.is_int_or_num() {
                                let right_type = self.deduce_type(expr,context);
                                if right_type.is_int_or_num() {
                                    if left_type.is_int() && right_type.is_int() {
                                        left_type = DataType::Int;
                                    }else{
                                        // 两操作数中有一或两个num类型  one or two of two operand are num type
                                        left_type = DataType::Num;
                                    }
                                }else{
                                    panic!("{} binary operator '{}' have wrong right operand type {}",context.info(),op,right_type)
                                }
                            }else{
                                panic!("{} binary operator '{}' have wrong left operand type {}",context.info(),op,left_type)
                            }
                        }
                        BinOpType::Compare => {
                            // number or char to bool
                            if left_type.is_num_liked() {
                                let right_type = self.deduce_type(expr,context);
                                if right_type.is_num_liked() {
                                    left_type = DataType::Bool;
                                }else{
                                    panic!("{} binary operator '{}' have wrong right operand type {}",context.info(),op,right_type)
                                }
                            }else{
                                panic!("{} binary operator '{}' have wrong left operand type {}",context.info(),op,left_type)
                            }
                        }
                        BinOpType::Equal => {
                            let right_type = self.deduce_type(expr,context);
                            if right_type.belong_to(&left_type) || left_type.belong_to(&right_type) {
                                left_type = DataType::Bool;
                            }else{
                                panic!("{} binary operator '{}' have wrong operand type, left : {} , right : {}",context.info(),op,left_type,right_type)
                            }
                        }
                        BinOpType::Logic => {
                            if left_type.is_bool() {
                                let right_type = self.deduce_type(expr,context);
                                if right_type.is_bool() {
                                    left_type = DataType::Bool;
                                }else{
                                    panic!("{} binary operator '{}' have wrong right operand type, expect bool found {}",context.info(),op,right_type)
                                }
                            }else{
                                panic!("{} binary operator '{}' have wrong left operand type, expect bool found {}",context.info(),op,left_type)
                            }
                        }
                    }
                }
                left_type
            }
            Expression::While(while_loop) => {
                match self.analysis_while(while_loop.deref_mut(), context) {
                    ReturnType::Void => DataType::Ref(RefType::None),
                    ReturnType::Have(data_type) => data_type
                }
            }
            Expression::IfElse(if_else) => {
                match self.analysis_if_else(if_else.deref_mut(), context) {
                    ReturnType::Have(data_type) => data_type,
                    ReturnType::Void => DataType::Ref(RefType::None)
                }
            }

            expr => panic!("unsupported expression {:?} now",expr)
        }
    }

    fn analysis_while(& self, while_loop : &mut WhileLoop, context : &mut AnalyzeContext) -> ReturnType {
        let line = while_loop.line;
        // check condition expression type
        let cond_expr = &mut while_loop.condition;
        let cond_type = self.deduce_type(cond_expr, context);
        if ! cond_type.is_bool() {
            panic!("{} line {}, the loop condition expression is not bool type but {}",context.info(),line,cond_type)
        };
        let mut temp_var_table = Vec::new();
        let statements = &mut while_loop.statements;
        let max_idx = statements.len() - 1;
        context.expr_stack.push((SyntaxType::While,line));
        context.break_stack.push(BreakType::Uninit);
        context.indexer.enter_sub_block();

        let mut temp_expr = Expression::None;
        let mut temp_line = 0;
        let mut expr_to_break = false;

        for (idx,statement) in statements.iter_mut().enumerate() {
            match statement {
                Statement::Let(let_tuple) => {
                    let (var,marked_type,expr,line) = let_tuple.deref_mut();
                    match marked_type {
                        None => {
                            // 未标记变量类型 without type mark
                            let deduced_type = self.deduce_type(expr,context);
                            let basic_type = deduced_type.as_basic();
                            // 检查变量名是否重复 check if the variable name occupied
                            let (slot_idx,sub_idx) = context.indexer.put(deduced_type);
                            match context.symbol_table.entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx,true)),
                                Entry::Occupied(_) => panic!("{} line {} variable name {} occupied",context.info(),line,var.name().deref()),
                            };
                            temp_var_table.push(var.name().deref().clone());
                            *var = Var::new_local(slot_idx,sub_idx,basic_type);
                        }
                        Some(data_type) => {
                            // 已标记变量类型 with type mark
                            let data_type = self.get_type(data_type, context.file_index);
                            let basic_type = data_type.as_basic();
                            if ! self.deduce_type(expr,context).belong_to(&data_type){
                                panic!()
                            }
                            let (slot_idx,sub_idx) = context.indexer.put(data_type);
                            // 检查变量名是否重复 check if the variable name occupied
                            match context.symbol_table.entry(var.name().deref().clone()) {
                                Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx, true)),
                                Entry::Occupied(_) => panic!("{} line {}, variable name {} occupied",context.info(),line,var.name().deref()),
                            };
                            temp_var_table.push(var.name().deref().clone());
                            *var = Var::new_local(slot_idx,sub_idx,basic_type);
                        }
                    }
                }
                Statement::LeftValueOp(left) => {
                    self.handle_left_value_op(context,left);
                }
                Statement::Expr(expr,line) => {
                    context.expr_stack.push((SyntaxType::Expr,*line));
                    let expr_type = self.deduce_type(expr,context);
                    if max_idx == idx {
                        // 默认返回最后一个表达式的结果  default return the result of last expression
                        let break_type = context.break_stack.last_mut().unwrap();
                        let mut alert = Option::None;
                        match break_type {
                            BreakType::Type(break_data_type) => {
                                if expr_type.belong_to(break_data_type) {
                                    // best situation, do nothing
                                }else if break_data_type.belong_to(&expr_type) {
                                    *break_data_type = expr_type;
                                }else{
                                    alert = Option::Some(format!("mismatched break type of loop, expect {} found {}",break_data_type,expr_type));
                                }
                            }
                            BreakType::Uninit => {
                                *break_type = if expr_type.is_none() {
                                    BreakType::Void
                                }else{
                                    BreakType::Type(expr_type)
                                }
                            }
                            BreakType::Void => {
                                if ! expr_type.is_none() {
                                    alert = Option::Some(format!("mismatch break type of while loop, expect void found {}",expr_type))
                                }
                            }
                        };
                        if let Some(alert) = alert {
                            panic!("{} {}",context.info(),alert)
                        }
                        expr_to_break = true;
                        temp_line = *line;
                        temp_expr = std::mem::replace(expr,Expression::None);
                    }
                    context.expr_stack.pop();
                }
                Statement::Discard(expr,line) => {
                    context.expr_stack.push((SyntaxType::Discard,*line));
                    self.deduce_type(expr,context);
                    if max_idx == idx {
                        // 表示这个while循环没有返回类型 means this while loop return void
                        let break_type = context.break_stack.last_mut().unwrap();
                        let mut alert = Option::None;
                        match break_type {
                            BreakType::Type(break_data_type) => {
                                alert = Option::Some(format!("mismatched break type of loop, expect void found {}",break_data_type));
                            }
                            BreakType::Uninit => {
                                *break_type = BreakType::Void;
                            }
                            BreakType::Void => {} // matched
                        };
                        if let Some(alert) = alert {
                            panic!("{}{}",context.info(),alert)
                        }
                    }
                    context.expr_stack.pop();
                }
                Statement::Break(expr,line) => {
                    context.expr_stack.push((SyntaxType::Break,*line));
                    let expr_type = self.deduce_type(expr,context);
                    let mut alert = Option::None;
                    let break_type = context.break_stack.last_mut().unwrap();
                    match break_type {
                        BreakType::Type(break_data_type) => {
                            if expr_type.belong_to(break_data_type) {
                                // best situation, do nothing
                            }else if break_data_type.belong_to(&expr_type) {
                                *break_data_type = expr_type;
                            }else{
                                alert = Option::Some(format!("line {}, mismatched break type of loop, expect {} found {}",line,break_data_type,expr_type));
                            }
                        }
                        BreakType::Uninit => {
                            *break_type = if expr_type.is_none() {
                                BreakType::Void
                            }else{
                                BreakType::Type(expr_type)
                            }
                        }
                        BreakType::Void => {
                            if ! expr_type.is_none() {
                                alert = Option::Some(format!("{} mismatch break type of while loop, expect void found {}",context.info(),expr_type));
                            }
                        }
                    };
                    if let Some(alert) = alert {
                        panic!("{}{}",context.info(),alert)
                    }
                    context.expr_stack.pop();
                }
                Statement::Return(expr,line) => {
                    context.expr_stack.push((SyntaxType::Return,*line));
                    let data_type = self.deduce_type(expr, context);
                    match &context.func_return_type {
                        ReturnType::Have(return_type) => {
                            if ! data_type.belong_to(return_type) {
                                panic!("{} line {}, expect return type if {}, found return type is {}",line,context.info(),return_type,data_type)
                            }
                        }
                        ReturnType::Void => {
                            if ! data_type.is_none() {
                                panic!("{} expect return void, found return {} type",context.info(),data_type)
                            }
                        }
                    }
                    context.expr_stack.pop();
                }

                Statement::Continue(_) => {}
                Statement::Static(_) => panic!("{} you can't declare static variable in while loop body",context.info()),
                Statement::PubStatic(_) => panic!("{} you can't declare public static variable in while loop body",context.info()),
                _ => {}
            }
            if expr_to_break {
                *statement = Statement::Break(
                    std::mem::replace(&mut temp_expr,Expression::None),
                    temp_line
                );
            }
        }
        while_loop.drop_vec = context.indexer.level_sub_block();
        ReturnType::Void
    }
    fn analysis_if_else(& self, if_else : &mut IfElse, context : &mut AnalyzeContext ) -> ReturnType {
        let mut result_type = BreakType::Uninit;
        let mut temp_vars = Vec::new();
        for (branch_idx,branch) in if_else.branches.iter_mut().enumerate() {
            // 处理每个分支  handle every branch
            let statements = &mut branch.statements;
            let cond_type = self.deduce_type(&mut branch.condition,context);
            if ! cond_type.is_bool() {
                panic!("{} the condition of {}st if-else branch have non-bool type {}", context.info(), branch_idx, cond_type);
            }
            context.expr_stack.push((SyntaxType::IfElseBranch,branch.line));
            context.indexer.enter_sub_block();
            let max_idx = statements.len() - 1;

            let mut expr_to_result = false;
            let mut temp_expr = Expression::None;
            let mut temp_line = 0;

            for (idx,statement) in statements.iter_mut().enumerate() {
                match statement {
                    Statement::Let(let_tuple) => {
                        let (var,marked_type,expr,line) = let_tuple.deref_mut();
                        match marked_type {
                            None => {
                                // 未标记变量类型 without type mark
                                let deduced_type = self.deduce_type(expr,context);
                                let basic_type = deduced_type.as_basic();
                                // 检查变量名是否重复 check if the variable name occupied
                                let (slot_idx,sub_idx) = context.indexer.put(deduced_type);
                                match context.symbol_table.entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx,true)),
                                    Entry::Occupied(_) => panic!("{} line {}, variable name {} occupied",context.info(),line,var.name().deref()),
                                };
                                temp_vars.push(var.name().deref().clone());
                                *var = Var::new_local(slot_idx,sub_idx,basic_type);
                            }
                            Some(data_type) => {
                                // 已标记变量类型 with type mark
                                let data_type = self.get_type(data_type, context.file_index);
                                let basic_type = data_type.as_basic();
                                if ! self.deduce_type(expr,context).belong_to(&data_type){
                                    panic!()
                                }
                                let (slot_idx,sub_idx) = context.indexer.put(data_type);
                                // 检查变量名是否重复 check if the variable name occupied
                                match context.symbol_table.entry(var.name().deref().clone()) {
                                    Entry::Vacant(entry) => entry.insert((slot_idx,sub_idx,true)),
                                    Entry::Occupied(_) => panic!("{} line {}, variable name {} occupied",context.info(),line,var.name().deref()),
                                };
                                *var = Var::new_local(slot_idx,sub_idx,basic_type);
                            }
                        }
                    }
                    Statement::LeftValueOp(left) => {
                        self.handle_left_value_op(context,left);
                    }
                    Statement::Expr(expr,line) => {
                        context.expr_stack.push((SyntaxType::Expr,*line));
                        let expr_type = self.deduce_type(expr,context);
                        if idx == max_idx {
                            match &mut result_type {
                                BreakType::Type(data_type) => {
                                    if expr_type.belong_to(data_type) {
                                        expr_to_result = true;
                                        temp_expr = std::mem::replace(expr,Expression::None);
                                        temp_line = *line;
                                    }else if data_type.belong_to(&expr_type) {
                                        *data_type = expr_type;
                                    }else {
                                        panic!("{} mismatched type, expect {} found {}",context.info(),data_type,expr_type)
                                    }
                                }
                                BreakType::Uninit => {
                                    result_type = if expr_type.is_none() {
                                        BreakType::Void
                                    }else{
                                        expr_to_result = true;
                                        temp_expr = std::mem::replace(expr,Expression::None);
                                        temp_line = *line;
                                        BreakType::Type(expr_type)
                                    }
                                }
                                BreakType::Void => {
                                    if ! expr_type.is_none() {
                                        panic!("{} expect the if-else return void but found {}",context.info(),expr_type)
                                    }
                                }
                            }

                        }
                        context.expr_stack.pop();
                    }
                    Statement::Discard(expr,line) => {
                        context.expr_stack.push((SyntaxType::Discard,*line));
                        self.deduce_type(expr,context);
                        if idx == max_idx {
                            match &mut result_type {
                                BreakType::Type(data_type) => {
                                    panic!("{} mismatched result type of if-else, expect {} found void",context.info(),data_type)
                                }
                                BreakType::Uninit => {
                                    result_type = BreakType::Void
                                }
                                BreakType::Void => {}
                            }
                        }
                        context.expr_stack.pop();
                    }

                    Statement::Break(expr,line) => {
                        context.expr_stack.push((SyntaxType::Break,*line));
                        let expr_type = self.deduce_type(expr,context);
                        let break_type = match context.break_stack.last_mut() {
                            Some(break_type) => break_type,
                            None => continue,
                        };
                        let mut alert = Option::None;
                        match break_type {
                            BreakType::Type(break_data_type) => {
                                if expr_type.belong_to(break_data_type) {
                                    // best situation, do nothing
                                }else if break_data_type.belong_to(&expr_type) {
                                    *break_data_type = expr_type;
                                }else{
                                    alert = Option::Some(format!("line {}, mismatched break type of loop, expect {} found {}",
                                                                 line,break_data_type,expr_type));
                                }
                            }
                            BreakType::Uninit => {
                                *break_type = if expr_type.is_none() {
                                    BreakType::Void
                                }else{
                                    BreakType::Type(expr_type)
                                }
                            }
                            BreakType::Void => {
                                if ! expr_type.is_none() {
                                    alert = Option::Some(format!("line {}, mismatch break type of while loop, expect void found {}",
                                                                 line,expr_type))
                                }
                            }
                        };
                        if let Some(alert) = alert {
                            panic!("{}{}",context.info(),alert)
                        }
                        context.expr_stack.pop();
                    }
                    Statement::Return(expr,line) => {
                        context.expr_stack.push((SyntaxType::Return,*line));
                        let data_type = if expr.is_none() {
                            DataType::Ref(RefType::None)
                        } else {
                            self.deduce_type(expr, context)
                        };
                        match &context.func_return_type {
                            ReturnType::Have(return_type) => {
                                if ! data_type.belong_to(return_type) {
                                    panic!("{} line {}, expect return type if {}, found return type is {}",
                                           context.info(),line,return_type,data_type)
                                }
                            }
                            ReturnType::Void => {
                                if ! data_type.is_none() {
                                    panic!("{} line {}, expect return void, found return {} type",
                                           context.info(),line,data_type)
                                }
                            }
                        }
                        context.expr_stack.pop();
                    }
                    Statement::Continue(_) => {}

                    Statement::Static(_) => panic!("{} you can't declare static variable in if-else statement",context.info()),
                    Statement::PubStatic(_) => panic!("{} you can't declare public static variable in if-else statement",context.info()),
                    _ => {}
                }
                if expr_to_result {
                    *statement = Statement::IfResult(
                        std::mem::replace(&mut temp_expr,Expression::None),
                        temp_line
                    )
                }
            }
            // 处理完一个分支的全部语句 handle all the statements of one branch
            // 清理分支内声明的变量的信息 clear the info of the variables declared in branch
            branch.drop_vec = context.indexer.level_sub_block();
            for temp_var_name in temp_vars.iter() {
                context.symbol_table.remove(temp_var_name.as_str());
            }
            temp_vars.clear();
            context.expr_stack.pop();
        }
        match result_type {
            BreakType::Type(data_type) => ReturnType::Have(data_type),
            BreakType::Void => ReturnType::Void,
            BreakType::Uninit => ReturnType::Void,
        }
    }

    fn fill_classes(&mut self){
        let mut index = 0;
        for class in self.status.classes.iter() {
            self.fill_class(class.clone(),index);
            index += 1;
        }
    }
    fn fill_class(&self, class : RefCount<GloomClass>, index : usize){
        let (parsed_class,file_index) = self.parsed_classes.get(index).unwrap();
        let parsed_class = parsed_class.clone();
        // handle parent class
        if let Option::Some(parent_name) = &parsed_class.inner().parent {
            match self.type_map.get(parent_name.as_str()) {
                None => { panic!("the parent class {} of {} not found",parent_name,parsed_class.inner().name) }
                Some(label) => {
                    if label.tp != MetaType::Class {
                        panic!("declared parent class {} of {} is not a class",parent_name,class.inner().name)
                    }
                    if label.is_public || label.file_index == *file_index {
                        let parent_class = self.status.classes.get(label.index as usize).unwrap().clone();
                        if parent_class.inner().len() == 0 {
                            // means parent class is not uninitialized, fill it recursively
                            self.fill_class(parent_class.clone(), label.index as usize);
                        }
                        class.inner_mut().set_parent(parent_class);
                    }else {
                        panic!("the parent class {} of {} is not public",parent_name,parsed_class.inner().name)
                    }
                }
            }
        }
        // fill fields
        for (is_pub,parsed_type,name) in parsed_class.inner().fields.iter() {
            class.inner_mut().add_field(
                *is_pub,
                name.deref().clone(),
                self.get_type(parsed_type, *file_index)
            );
        }
        // fill funcs
        for (is_pub, name, func) in parsed_class.inner_mut().funcs.iter_mut() {
            let mut params = Vec::with_capacity(func.params.len());
            for (param_name, parsed_type) in func.params.iter() {
                params.push(Param::new(
                    param_name.clone(),
                    self.get_type(parsed_type, *file_index)
                ));
            }
            let return_type : ReturnType = match &func.return_type {
                None => ReturnType::Void,
                Some(parsed_type) => ReturnType::Have(self.get_type(parsed_type, *file_index)),
            };
            // 在不需要move ParsedFunc 的情况下，仅使用ParsedFunc的可变引用将函数体的Vec<Statement> move至status中的GloomClass中
            let body : Vec<Statement> = std::mem::replace(&mut func.body, Vec::with_capacity(0));
            class.inner_mut().add_func(*is_pub,name.clone(),params,return_type,body);
        }
        // handle instance funcs
        class.inner().handle_instance_func(class.clone());
        // handle implemented interface
        for interface_name in parsed_class.inner().impl_interfaces.iter() {
            match self.type_map.get(interface_name.as_str()){
                None => panic!("interface {} that implemented by class {} is not found",
                               interface_name,class.inner().name),
                Some(label) => {
                    if label.tp != MetaType::Interface {
                        panic!("interface {} that implemented by class {} is in fact not an interface but a {}",
                               interface_name, class.inner().name, label.tp)
                    }
                    if label.is_public || label.file_index == *file_index {
                        let interface = self.status.interfaces.get(label.index as usize).unwrap().clone();
                        class.inner_mut().add_impl(interface);
                    }else {
                        panic!("interface {} that implemented by class {} is not public",
                               interface_name,class.inner().name)
                    }
                }
            }
        }
    }

    fn fill_enums(&mut self){
        let mut index = 0;
        for enum_class in self.status.enums.iter() {
            self.fill_enum(enum_class.clone(),index);
            index += 1;
        }
    }
    fn fill_enum(&self, enum_class : RefCount<GloomEnumClass>, index : usize){
        let (parsed_enum,file_index) = self.parsed_enums.get(index).unwrap();
        let parsed_enum = parsed_enum.clone();
        for (name,parsed_type) in parsed_enum.inner().values.iter() {
            let related_type: Option<DataType> = match parsed_type {
                None => Option::None,
                Some(parsed_type) => Some(self.get_type(parsed_type, *file_index))
            };
            enum_class.inner_mut().add_enum_value(name.deref().clone(),related_type);
        }
        for (func_name, is_pub , func) in parsed_enum.inner_mut().funcs.iter_mut() {
            let mut params = Vec::with_capacity(func.params.len());
            for (name, parsed_type) in func.params.iter() {
                params.push(Param::new(
                    name.clone(),
                    self.get_type(parsed_type, *file_index)
                ));
            }
            let return_type = match func.return_type.borrow() {
                None => ReturnType::Void,
                Some(parsed_type) => ReturnType::Have(self.get_type(parsed_type, *file_index))
            };
            let body = std::mem::replace(&mut func.body,Vec::with_capacity(0));
            enum_class.inner_mut().add_func(func_name.clone(),*is_pub,params,return_type,body);
        }
        enum_class.inner_mut().handle_instance_func(enum_class.clone())
    }

    fn analysis_interfaces(&mut self){
        let mut index = 0;
        for interface in self.status.interfaces.iter() {
            self.analysis_interface(interface.clone(),index);
            index += 1;
        }
    }
    fn analysis_interface(&self, interface : RefCount<Interface>, index : usize){
        let (parsed_interface,file_index) = self.parsed_interfaces.get(index).unwrap();
        for parent_name in parsed_interface.parents.iter() {
            match self.type_map.get(parent_name.as_str()) {
                None => panic!("Parent interface {} or interface {} is not found",parent_name,interface.inner().name),
                Some(label) => {
                    if label.tp != MetaType::Interface {
                        panic!("declared parent interface {} of {} is not interface",parent_name,interface.inner().name)
                    }
                    let parent_interface = self.status.interfaces.get(label.index as usize).unwrap();
                    if parent_interface.inner().len() == 0 {
                        self.analysis_interface(parent_interface.clone(),label.index as usize);
                    }
                    interface.inner_mut().add_parent(&interface,parent_interface);
                }
            }
        }
        for (name, param_types, return_type) in parsed_interface.funcs.iter() {
            let mut param_data_types = Vec::with_capacity(param_types.len());
            let mut have_self = false;
            for (idx,parsed_type) in param_types.iter().enumerate() {
                param_data_types.push(if let ParsedType::MySelf = parsed_type {
                    if idx == 0 {
                        have_self = true;
                        DataType::Ref(RefType::Interface(interface.clone()))
                    }else{
                        panic!("wrong {}st parameter 'self' occurs in function {} of Interface {}",
                               idx,name,interface.inner().name)
                    }
                }else{
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
                                             name,interface.inner().name)
            }
            interface.inner_mut().add_func(AbstractFunc{
                name: name.clone(),
                param_types: param_data_types,
                return_type,
                have_self,
            });
        }
    }

    fn load_decl(&mut self, script : &mut ParsedFile){
        let file_index = self.file_count;
        script.index = file_index;
        self.file_count += 1;
        // load empty interface
        for (parsed_inter,is_public) in script.interfaces.iter() {
            let index = self.status.interfaces.len();
            match self.type_map.entry(parsed_inter.name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(TypeIndex::from(index as u16, *is_public, file_index, MetaType::Interface));
                }
                Entry::Occupied(_) => panic!("Type name {} already occupied",parsed_inter.name)
            };
            self.status.interfaces.push(RefCount::new(Interface::new(parsed_inter.name.clone())));
        }
        // load empty class
        for (class,is_pub) in script.classes.iter() {
            let index = self.status.classes.len();
            match self.type_map.entry(class.name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(TypeIndex::from(index as u16, *is_pub, file_index, MetaType::Class));
                }
                Entry::Occupied(_) => panic!("Type name {} already occupied",class.name)
            };
            self.status.classes.push(RefCount::new(GloomClass::new(class.name.clone(), file_index)));
        }
        // load empty enum
        for (enum_class, is_pub) in script.enums.iter() {
            let index = self.status.enums.len();
            match self.type_map.entry(enum_class.name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(TypeIndex::from(index as u16, *is_pub, file_index, MetaType::Enum));
                }
                Entry::Occupied(_) => panic!("Type name {} already occupied",enum_class.name)
            };
            self.status.enums.push(RefCount::new(GloomEnumClass::new(enum_class.name.clone(), file_index)));
        }
        for parsed_file in script.imports.iter_mut() {
            self.load_decl(parsed_file);
        }
    }
    fn load(&mut self, script : ParsedFile){
        let index = script.index;
        for (class,_) in script.classes.into_iter() {
            self.parsed_classes.push((RefCount::new(class), index));
        }
        for (interface,_) in script.interfaces.into_iter() {
            self.parsed_interfaces.push((interface,index));
        }
        for (enum_class, _) in script.enums.into_iter() {
            self.parsed_enums.push((RefCount::new(enum_class), index));
        }
        for (name,func,is_pub) in script.funcs.into_iter() {
            let index = self.status.funcs.len() as u16;
            let mut params = Vec::with_capacity(func.params.len());
            for (name, parsed_type) in func.params.into_iter() {
                params.push(Param::new(
                    name,
                    self.get_type(&parsed_type, script.index)
                ));
            }
            let return_type = match func.return_type {
                None => ReturnType::Void,
                Some(parsed_type) => ReturnType::Have(self.get_type(&parsed_type, script.index))
            };
            match self.func_map.entry(name.deref().clone()) {
                Entry::Vacant(entry) => {
                    entry.insert((index,false,is_pub,script.index));
                }
                Entry::Occupied(_) => panic!("func name {} already occupied",name)
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
            script.statements
        ),index)));
        for file in script.imports.into_iter() {
            self.load(file);
        }
    }

    // ParsedType -> DataType
    fn get_type(& self, origin_type : &ParsedType, file_index : u16) -> DataType{
        match origin_type {
            ParsedType::Single(single_type) => {
                self.analysis_single_type(single_type,file_index)
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
    fn analysis_single_type(& self, single_type : &SingleType, file_index : u16) -> DataType {
        let generic = match &single_type.generic {
            Some(vec) => {
                let mut types = Vec::with_capacity(vec.len());
                for parsed_type in vec.iter() {
                    types.push(self.get_type(parsed_type,file_index));
                }
                Option::Some(types)
            }
            None => Option::None
        };
        match single_type.name.as_str() {
            "int" => return DataType::Int,
            "num" => return  DataType::Num,
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
                }else {
                    panic!("{} is not public",single_type.name)
                }
            }
            None => {
                /*match BuiltinType::try_from_str(single_type.name.as_str()) {
                    // 导入内置类型 import builtin type
                    Some(builtin_type) => {
                        let builtin_classes = self.status.builtin_classes.clone();
                        let index = builtin_classes.inner().len() as u16;
                        self.type_map.insert(
                            single_type.name.clone(),
                            Label::from(index,true,0,MetaType::Builtin)
                        );
                        self.builtin_map.clone().inner_mut().insert(builtin_type,index);

                        let class = BuiltinClass::from_builtin_type(builtin_type);
                        // return data type
                        let ref_type = class.get_ref_type(generic);
                        builtin_classes.inner_mut().push(class);
                        DataType::Ref(ref_type)
                    }
                    None => panic!("type '{}' not found", single_type.name)
                }*/
                panic!("{:?}",single_type)
            }
        }
    }

    pub fn result(self) -> (GloomStatus,StaticTable){
        let mut indexer = self.static_indexer.inner_mut();
        let static_len = indexer.size();
        let static_drop_vec = indexer.basic_drop_vec();
        let static_table = StaticTable::new(static_len,static_drop_vec);
        (self.status,static_table)
    }

    pub fn new() -> Analyzer{
        Analyzer {
            file_count: 0,
            parsed_interfaces: Vec::new(),
            parsed_classes: Vec::new(),
            parsed_enums : Vec::new(),
            type_map: BuiltinClass::class_map(),
            func_map: BuiltInFuncs::func_map(),
            status : GloomStatus::new(),
            static_map: RefCount::new(HashMap::new()),
            builtin_map: BuiltinClass::builtin_type_map(),
            func_file_indexes: Vec::new(),
            static_indexer: RefCount::new(SlotIndexer::new())
        }
    }
}

pub struct AnalyzeContext<'a>{
    pub func_name : Rc<String>,
    pub symbol_table : HashMap<String,(u16,u8,IsLocal)>,
    pub file_index : u16,
    pub file_name : Rc<String>,
    pub out_context: Option<&'a AnalyzeContext<'a>>,
    pub captures: Vec<Capture>,
    pub belonged_type : DeclaredType,
    pub func_return_type : ReturnType,
    pub expr_stack : Vec<(SyntaxType,u16)>,
    pub break_stack : Vec<BreakType>,
    pub indexer : SlotIndexer
}

impl<'a> AnalyzeContext<'a>{
    pub fn new(func_name : Rc<String>,
               belonged_type: DeclaredType,
               func_return_type: ReturnType,
               file_index : u16,
               out_context: Option<&'a AnalyzeContext>) -> AnalyzeContext<'a>{
        AnalyzeContext{
            func_name,
            file_index,
            belonged_type,
            func_return_type,
            out_context,
            symbol_table: HashMap::new(),
            captures: Vec::new(),
            expr_stack: Vec::new(),
            break_stack: Vec::new(),
            file_name: Rc::new(String::from("")),
            indexer: SlotIndexer::new()
        }
    }

    pub fn info(&self) -> String{
        // type => func => expr > expr > expr
        let mut info = format!("[ {} => ",self.file_name);
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
        for (frame_type,line) in self.expr_stack.iter() {
            info = info.add(format!("{:?} line {}",frame_type,line).as_str()).add(" > ");
        }
        info.add("] \r\n")
    }

}