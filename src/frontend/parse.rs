use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use hashbrown::HashMap;
use crate::frontend::ast::{Expression, IfElse, Statement, ParsedType, Var, BinOpVec, VarId, TypeTuple, SingleType, ParsedFunc, WhileLoop, ForLoop, ParsedClass, ParsedInterface, ParsedEnum, Construction, ExprType, ForIter, FuncExpr, Chain, IfBranch, LeftValue};
use crate::frontend::import::Importer;
use crate::frontend::ops::{BinOp, LeftValueOp};
use crate::frontend::script::ParsedFile;
use crate::frontend::token::Token;
use crate::obj::refcount::RefCount;
use crate::obj::types::{BasicType, DataType, RefType};

pub struct Parser{
    tokens : Vec<Token>,
    curr : usize,
    classes : Vec<(ParsedClass,bool)>,
    interfaces : Vec<(ParsedInterface,bool)>,
    enums : Vec<(ParsedEnum,bool)>,
    funcs : Vec<(Rc<String>, ParsedFunc, bool)>,
    imports : Vec<ParsedFile>,
    importer : RefCount<Importer>,
    pub lines: Vec<u16>,
}

impl Parser {
    pub fn parse(mut self) -> ParsedFile{
        let vec = self.statements();
        ParsedFile{
            imports: self.imports,
            classes: self.classes,
            interfaces: self.interfaces,
            funcs: self.funcs,
            enums : self.enums,
            statements: vec,
            index : 0
        }
    }

    fn statements(&mut self) -> Vec<Statement>{
        let mut statements = Vec::new();
        while self.has_next() && ! self.test_next(Token::RBrace) {
            let statement = match self.next() {
                Token::Continue => {
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    Statement::Continue(self.line())
                },
                Token::Return => {
                    let index = self.curr;
                    let line = self.line();
                    let result_statement = match self.expr() {
                        Err(_) => {
                            self.rollback(index);
                            Statement::Return(Expression::None,line)
                        }
                        Ok(expr) => {
                            Statement::Return(expr,line)
                        }
                    };
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    result_statement
                }
                Token::Break => {
                    let index = self.curr;
                    let line = self.line();
                    let result_statement = match self.expr() {
                        Err(_) => {
                            self.rollback(index);
                            Statement::Break(Expression::None,line)
                        }
                        Ok(expr) => {
                            Statement::Break(expr,line)
                        }
                    };
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    result_statement
                }
                // 局部变量声明 Let
                Token::Let => {
                    let line = self.line();
                    let var_name = self.identifier();
                    let mut parsed_type: Option<ParsedType> = None;
                    if self.test_next(Token::Eq) {
                        self.forward();
                    } else {
                        parsed_type = Some(self.parse_type());
                        self.assert_next(Token::Eq);
                    }
                    let expr = self.expr().unwrap();
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    Statement::Let(Box::new((Var::Name(var_name), parsed_type, expr, line)))
                }
                Token::Static => {
                    let var_name = self.identifier();
                    let mut parsed_type: Option<ParsedType> = None;
                    if self.test_next(Token::Eq) {
                        self.forward();
                    } else {
                        parsed_type = Some(self.parse_type());
                        self.assert_next(Token::Eq);
                    }
                    let expr = self.expr().unwrap();
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    Statement::Static(Box::new((Var::Name(var_name), parsed_type, expr)))
                }
                Token::RBrace => {
                    self.backward();
                    break
                }
                Token::Comma => {
                    self.backward();
                    break
                }
                Token::Semi => {
                    continue
                }
                Token::Import => {
                    let import_path = self.identifier();
                    let file = self.importer.inner_mut().import_file(import_path.as_str(), self.importer.clone());
                    self.imports.push(file);
                    continue
                }
                // public declaration
                Token::Pub => {
                    match self.next() {
                        Token::Class => {
                            let class = self.parse_class();
                            self.classes.push((class,true));
                            continue
                        }
                        Token::Interface => {
                            let interface = self.parse_interface();
                            self.interfaces.push((interface,true));
                            continue
                        }
                        Token::Enum => {
                            let parsed_enum = self.parse_enum();
                            self.enums.push((parsed_enum,true));
                            continue
                        }
                        Token::Func => {
                            let func_name = self.identifier();
                            let func = self.parse_func(false);
                            self.funcs.push((func_name,func,true));
                            continue
                        }
                        Token::Static => {
                            let var_name = self.identifier();
                            let mut parsed_type: Option<ParsedType> = None;
                            if self.test_next(Token::Eq) {
                                self.forward();
                            } else {
                                parsed_type = Some(self.parse_type());
                                self.assert_next(Token::Eq);
                            }
                            let expr = self.expr().unwrap();
                            if self.has_next() && self.test_next(Token::Semi) {
                                self.forward();
                            }
                            Statement::PubStatic(Box::new((Var::Name(var_name), parsed_type, expr)))
                        }
                        token => panic!("expect class interface enum or func, found {:?}",token),
                    }
                }
                // private declaration
                Token::Class => {
                    let class = self.parse_class();
                    self.classes.push((class,false));
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    continue
                }
                Token::Interface => {
                    let interface = self.parse_interface();
                    self.interfaces.push((interface,false));
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    continue
                }
                Token::Enum => {
                    let parsed_enum = self.parse_enum();
                    self.enums.push((parsed_enum,false));
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    continue
                }
                Token::Func => {
                    if self.test_next(Token::LParen) {
                        // nameless func / closure / lambda expression
                        // let curr pointer to Token::Func
                        self.backward();
                        let line = self.line();
                        let expr = self.expr().expect(format!("{:?}",statements).as_str());
                        if self.has_next() && self.test_next(Token::Semi) {
                            self.forward();
                            Statement::Discard(expr,line)
                        } else {
                            Statement::Expr(expr,line)
                        }
                    }else {
                        // function declaration
                        let func_name = self.identifier();
                        let func = self.parse_func(false);
                        self.funcs.push((func_name,func,false));
                        continue
                    }
                }
                _ => {
                    self.backward();
                    let line = self.line();
                    let expr = self.expr().unwrap();
                    if self.has_next() {
                        let token = self.next();
                        match token {
                            Token::Semi => Statement::Discard(expr,line),
                            Token::Eq | Token::SubEq | Token::PlusEq | Token::SubSub | Token::PlusPlus => {
                                let left_value = match expr {
                                    Expression::Var(var) => {
                                        LeftValue::Var(*var)
                                    }
                                    Expression::Chain(chain_box) => {
                                        let (_, chains) = chain_box.deref();
                                        if let Chain::Access(_,_) = chains.last().unwrap() {} else {
                                            panic!()
                                        }
                                        let (expr, chains) = *chain_box;
                                        LeftValue::Chain(expr, chains)
                                    }
                                    _ => panic!()
                                };
                                let left_value_op = match token {
                                    Token::Eq => LeftValueOp::Assign(self.expr().unwrap()),
                                    Token::SubEq => LeftValueOp::SubEq(self.expr().unwrap()),
                                    Token::PlusEq => LeftValueOp::PlusEq(self.expr().unwrap()),
                                    Token::SubSub => LeftValueOp::SubOne,
                                    Token::PlusPlus => LeftValueOp::PlusOne,
                                    _ => panic!()
                                };
                                if self.has_next() && self.test_next(Token::Semi) {
                                    // handle ';'
                                    self.forward();
                                }
                                Statement::LeftValueOp(Box::new((left_value,left_value_op)))
                            }
                            _ => {
                                self.backward();
                                Statement::Expr(expr,line)
                            }
                        }
                    }else {
                        Statement::Expr(expr,line)
                    }
                }
            };
            statements.push(statement)
        }
        statements
    }

    fn expr(&mut self) -> Result<Expression,ParseError>{
        let expr = self.medium_expr()?;
        let mut op_vec : Option<Vec<(BinOp,Expression)>> = Option::None;
        while self.has_next() {
            let bin_op_expr : Option<(BinOp,Expression)>;
            match self.next() {
                Token::Plus => bin_op_expr = Some((BinOp::Plus, self.medium_expr()?)),
                Token::Sub => bin_op_expr = Some((BinOp::Sub, self.medium_expr()?)),
                Token::And => bin_op_expr = Some((BinOp::And, self.medium_expr()?)),
                Token::Or => bin_op_expr = Some((BinOp::Or, self.medium_expr()?)),
                _ => {
                    self.backward();
                    break
                }
            }
            match bin_op_expr {
                Some(op_tuple) => {
                    match &mut op_vec {
                        None => {
                            let mut vec = Vec::new();
                            vec.push(op_tuple);
                            op_vec = Some(vec);
                        },
                        Some(vec) => vec.push(op_tuple),
                    }
                }
                None => break,
            }
        }
        match op_vec {
            Some(vec) => {
                Result::Ok(Expression::BinaryOp(Box::new(BinOpVec{
                    left: expr,
                    vec
                })))
            }
            None => Result::Ok(expr),
        }
    }

    fn medium_expr(&mut self) -> Result<Expression,ParseError>{
        let mut expr = self.primary_expr()?;
        if self.has_next() && self.test_next(Token::As) {
            self.forward();
            let parsed_type = self.parse_type();
            expr = Expression::Cast(Box::new((
                expr,
                parsed_type,
                DataType::Ref(RefType::Any)
            )));
        }
        let mut op_vec : Option<Vec<(BinOp,Expression)>> = Option::None;
        while self.has_next() {
            let bin_op_expr : Option<(BinOp,Expression)>;
            match self.next() {
                Token::Mul => bin_op_expr = Some((BinOp::Mul, self.primary_expr()?)),
                Token::Div => bin_op_expr = Some((BinOp::Div, self.primary_expr()?)),
                Token::Gt => bin_op_expr = Some((BinOp::Gt, self.primary_expr()?)),
                Token::Lt => bin_op_expr = Some((BinOp::Lt, self.primary_expr()?)),
                Token::Eqs => bin_op_expr = Some((BinOp::Eqs, self.primary_expr()?)),
                Token::GtEq => bin_op_expr = Some((BinOp::GtEq, self.primary_expr()?)),
                Token::LtEq => bin_op_expr = Some((BinOp::LtEq, self.primary_expr()?)),
                Token::NotEq => bin_op_expr = Some((BinOp::NotEq, self.primary_expr()?)),
                _ => {
                    self.backward();
                    break
                }
            }
            match bin_op_expr {
                Some(op_tuple) => match &mut op_vec {
                    None => {
                        let mut vec = Vec::new();
                        vec.push(op_tuple);
                        op_vec = Some(vec);
                    },
                    Some(vec) => {
                        vec.push(op_tuple);
                    },
                },
                None => break,
            }
        }
        match op_vec {
            Some(vec) => {
                Result::Ok(Expression::BinaryOp(Box::new(BinOpVec{
                    left: expr,
                    vec
                })))
            }
            None => Result::Ok(expr),
        }
    }
    fn primary_expr(&mut self) -> Result<Expression,ParseError> {
        let line = self.line();
        let mut expr = match self.next() {
            // 字面量 literal value : int num char bool str
            Token::Int(int) => Expression::Int(*int),
            Token::Num(num) => Expression::Num(*num),
            Token::Char(ch) => Expression::Char(*ch),
            Token::Bool(bl) => Expression::Bool(*bl),
            Token::Str(str) => Expression::Str(str.clone()),
            // 变量、函数调用或成员变量访问 variable、func call or member field access
            Token::Id(id) => {
                let var_name = id.clone();
                // object construct
                if self.has_next() && self.test_next(Token::LBrace) {
                    self.forward();
                    let class_name = var_name.clone();
                    let mut fields = Vec::new();
                    while self.has_next() {
                        match self.next() {
                            Token::RBrace => break,
                            Token::Comma => continue,
                            _ => {
                                self.backward();
                                let field_name = self.identifier();
                                self.assert_next(Token::Colon);
                                let expr = self.expr()?;
                                fields.push((VarId::Name(field_name),expr));
                            }
                        }
                    }
                    Expression::Construct(Box::new(Construction {
                        class_type: ExprType::Parsed(ParsedType::Single(SingleType {
                            name: class_name,
                            generic: None
                        })),
                        fields
                    }))
                }else{
                    Expression::Var(Box::new(Var::Name(var_name)))
                }
            },
            Token::If => {
                let mut branches = Vec::new();
                while self.has_next() {
                    let condition = self.expr()?;
                    self.assert_next(Token::LBrace);
                    let branch = self.statements();
                    self.assert_next(Token::RBrace);
                    let line = self.line();
                    branches.push(IfBranch{
                        condition,
                        statements: branch,
                        drop_vec: Vec::with_capacity(0),
                        line
                    });
                    if !self.has_next() {
                        break
                    }
                    if !self.test_next(Token::Else) {
                        break
                    }
                    self.forward();
                    if self.test_next(Token::If) {
                        // 继续 if(){}else if(){}
                        self.forward();
                        continue
                    } else if self.test_next(Token::LBrace) {
                        // if(){}else{}
                        self.forward();
                        let line = self.line();
                        branches.push(IfBranch{
                            condition : Expression::Bool(true),
                            statements: self.statements(),
                            drop_vec: Vec::with_capacity(0),
                            line
                        });
                        self.assert_next(Token::RBrace);
                        break
                    } else {
                        println!("unexpected token near if-else {:?}", self.peek());
                        break
                    }
                }
                Expression::IfElse(Box::new(IfElse {
                    branches,
                }))
            }
            // 元组 或 (计算表达式) tuple or (calculation expression)
            Token::LParen => {
                let mut vec: Vec<Expression> = Vec::new();
                while self.has_next() {
                    vec.push(self.expr()?);
                    if self.test_next(Token::RParen) {
                        self.forward();
                        break
                    } else if self.test_next(Token::Comma) {
                        self.forward();
                    } else {
                        println!("unexpected token {:?} near tuple parse, line {}", self.peek(),self.line())
                    }
                }
                if vec.len() == 1 {
                    vec.pop().unwrap()
                } else {
                    Expression::Tuple(Box::new(vec))
                }
            }
            // 数组/队列 Array Queue
            Token::LBracket => {
                let mut vec: Vec<Expression> = Vec::new();
                while self.has_next() {
                    vec.push(self.expr()?);
                    let line = self.line();
                    match self.next() {
                        Token::RBracket => {
                            break
                        }
                        Token::Comma => {}
                        token => {
                            return Err(ParseError::new(line,format!("unexpect token when parse array, expect ']' or ',', found {:?}",token)))
                        }
                    }
                }
                Expression::Array(Box::new((vec,BasicType::Ref,false)))
            }
            // 匿名函数 Anonymous Function
            // 立即执行函数 immediate exec func
            Token::Func => {
                let func = self.parse_func(false);
                Expression::Func(Box::new(FuncExpr::Parsed(func)))
            }
            // while循环 while-loop
            Token::While => {
                let line = self.line();
                let condition = self.expr()?;
                self.assert_next(Token::LBrace);
                let statements = self.statements();
                self.assert_next(Token::RBrace);
                Expression::While(Box::new(WhileLoop {
                    condition,
                    statements,
                    drop_vec: Vec::with_capacity(0),
                    line
                }))
            }
            // for-循环 for-loop
            Token::For => {
                let var_name = self.identifier();
                self.assert_next(Token::In);
                let expr = self.expr()?;
                let for_iter : ForIter = if let Expression::Tuple(mut tuple) = expr {
                    let vec= tuple.deref_mut();
                    if vec.len() == 2 {
                        let end = vec.pop().unwrap();
                        let start = vec.pop().unwrap();
                        ForIter::Num(start, end, Expression::Int(1))
                    } else if vec.len() == 3 {
                        let step = vec.pop().unwrap();
                        let end = vec.pop().unwrap();
                        let start = vec.pop().unwrap();
                        ForIter::Num(start, end, step)
                    } else {
                        panic!("'for <Var> in <Tuple>' is wrong syntax")
                    }
                } else {
                    ForIter::Iter(expr)
                };
                self.assert_next(Token::LBrace);
                let statements = self.statements();
                self.assert_next(Token::RBrace);
                Expression::For(RefCount::new(ForLoop {
                    var: Var::Name(var_name),
                    for_iter,
                    statements
                }))
            }
            // 匹配 match
            Token::Match => {
                self.assert_next(Token::LParen);
                let expr = self.expr()?;
                self.assert_next(Token::RParen);
                self.assert_next(Token::LBrace);
                let mut branches = Vec::new();
                while self.has_next() {
                    match self.next() {
                        Token::RBrace => break,
                        _ => {
                            self.backward();
                            let try_matched = self.expr()?;
                            self.assert_next(Token::Arrow);
                            if self.test_next(Token::LBrace) {
                                self.forward();
                                let statements = self.statements();
                                branches.push((try_matched, statements));
                                self.assert_next(Token::RBrace);
                            } else {
                                let statements = self.statements();
                                branches.push((try_matched, statements));
                                self.assert_next(Token::Comma);
                            }
                        }
                    }
                }
                Expression::Match(RefCount::new((expr, branches)))
            }
            // 一元操作 Unary operation
            Token::Not => Expression::NotOp(Box::new(self.expr()?)),
            Token::Sub => Expression::NegOp(Box::new(self.expr()?)),
            token => {
                return Result::Err(ParseError::new(line,format!("unexpected token {:?} when parse primary expression",token)));
            }
        };
        if self.has_next() && (self.test_next(Token::Dot) || self.test_next(Token::LParen)) {
            let mut chains = Vec::new();
            let line = self.line();
            while self.has_next() {
                match self.next() {
                    Token::Dot => {
                        // field access
                        let field_name = self.identifier();
                        if self.has_next() && self.test_next(Token::LParen) {
                            self.forward();
                            let mut args = Vec::new();
                            while self.has_next() {
                                let arg = self.expr()?;
                                args.push(arg);
                                match self.next() {
                                    Token::Comma => continue,
                                    Token::RParen => break,
                                    token => {
                                        return Result::Err(ParseError::new(
                                            line, format!("expect ',' or ')' in function call arguments, found {}",token)));
                                    }
                                }
                            }
                            chains.push(Chain::FnCall{
                                func: VarId::Name(field_name),
                                need_self : false,
                                args
                            })
                        }else{
                            chains.push(Chain::Access(
                                VarId::Name(field_name),
                                BasicType::Ref
                            ));
                        }
                    }
                    Token::LParen => {
                        // function call
                        let mut args = Vec::new();
                        while self.has_next() {
                            match self.next() {
                                Token::Comma => continue,
                                Token::RParen => break,
                                _ => {
                                    self.backward();
                                    let arg = self.expr()?;
                                    args.push(arg);
                                }
                            }
                        }
                        chains.push(Chain::Call(args))
                    }
                    _ => {
                        self.backward();
                        break
                    }
                }
            }// while loop end
            expr = Expression::Chain(Box::new((expr,chains)))
        }
        Result::Ok(expr)
    }

    fn parse_func(&mut self, is_mem_func : bool) -> ParsedFunc{
        self.assert_next(Token::LParen);
        let mut param_vec = Vec::new();
        while self.has_next() {
            match self.next() {
                Token::RParen => break,
                Token::Comma => continue,
                _ => {
                    self.backward();
                    if let Token::Id(name) = self.peek(){
                        if is_mem_func && name.as_str().eq("self") {
                            param_vec.push((Rc::new(String::from("self")), ParsedType::MySelf));
                            self.forward();
                            continue
                        }
                    }
                    let param_type = self.parse_type();
                    let param_name = self.identifier();
                    param_vec.push((param_name,param_type));
                }
            }
        }
        let mut return_type = Option::None;
        if ! self.test_next(Token::LBrace) {
            return_type = Some(self.parse_type())
        }
        self.assert_next(Token::LBrace);
        let statements = self.statements();
        self.assert_next(Token::RBrace);
        ParsedFunc{
            params: param_vec,
            body: statements,
            return_type
        }
    }
    fn parse_class(&mut self) -> ParsedClass {
        let name = self.identifier();
        let mut parent_class = None;
        let mut impl_vec = Vec::with_capacity(0);
        let mut field_vec = Vec::new();
        let mut func_vec = Vec::new();
        // parse inherit
        if self.test_next(Token::Colon) {
            self.forward();
            parent_class = Some(self.identifier());
        }
        // parse implementation
        if self.test_next(Token::Impl) {
            self.forward();
            while self.has_next() {
                impl_vec.push(self.identifier());
                if self.test_next(Token::Comma) {
                    self.forward();
                }
                if self.test_next(Token::LBrace) {
                    break
                }else {
                    panic!()
                }
            }
        }
        self.assert_next(Token::LBrace);
        // parse fields and funcs
        let mut is_public;
        while self.has_next() {
            match self.next() {
                Token::RBrace => break,
                Token::Pub => {
                    // public
                    is_public = true;
                }
                _ => {
                    // private
                    self.backward();
                    is_public = false;
                }
            }
            if self.test_next(Token::Func){ // function
                self.forward();
                let func_name = self.identifier();
                let parsed_func = self.parse_func(true);
                func_vec.push((is_public,func_name,parsed_func));
            }else{ // field
                let parsed_type = self.parse_type();
                let name = self.identifier();
                field_vec.push((is_public,parsed_type,name));
            }
        }
        ParsedClass{
            name,
            parent: parent_class,
            impl_interfaces: impl_vec,
            fields : field_vec,
            funcs : func_vec
        }
    }
    fn parse_interface(&mut self) -> ParsedInterface {
        let name = self.identifier();
        let mut parents = Vec::new();
        if self.test_next(Token::Colon) {
            self.forward();
            while self.has_next() {
                match self.next() {
                    Token::LBrace => break,
                    Token::Comma => continue,
                    Token::Id(id)=> parents.push(id.clone()),
                    token => {
                        panic!("unexpect token {:?} when parse interface parent define, expect identifier as parent interface name",token)
                    }
                }
            }
        }else {
            self.assert_next(Token::LBrace);
        }
        let mut funcs  = Vec::new();
        while self.has_next() {
            match self.next() {
                Token::RBrace => {
                    break
                }
                Token::Func => {
                    let func_name = self.identifier();
                    let mut params = Vec::new();
                    self.assert_next(Token::LParen);
                    // gloom self
                    if let Token::Id(name) = self.peek(){
                        if name.as_str().eq("self") {
                            params.push(ParsedType::Single(SingleType{
                                name : name.clone(),
                                generic : None
                            }));
                            self.forward();
                        }
                    }
                    // parse params
                    while self.has_next() {
                        match self.next() {
                            Token::RParen => break,
                            Token::Comma => continue,
                            _ => {
                                self.backward();
                                let parsed_type = self.parse_type();
                                self.identifier();
                                params.push(parsed_type);
                            }
                        }
                    }
                    // parse return type
                    let mut return_type = Option::None;
                    if ! self.test_next(Token::Func) && ! self.test_next(Token::RBrace) {
                        return_type = Option::Some(self.parse_type());
                    }
                    funcs.push((func_name,params,return_type));
                }
                token => {
                    panic!("expect token 'func', found {:?}",token);
                }
            }
        }
        ParsedInterface{
            name,
            parents,
            funcs
        }
    }
    fn parse_enum(&mut self) -> ParsedEnum {
        let enum_name= self.identifier();
        let mut enum_values = Vec::new();
        let mut funcs = Vec::new();
        self.assert_next(Token::LBrace);
        while self.has_next() {
            match self.next() {
                Token::RBrace => break,
                Token::Id(id) => {
                    let value_name = id.clone();
                    if self.test_next(Token::LParen) {
                        self.assert_next(Token::LParen);
                        let parsed_type = self.parse_type();
                        self.assert_next(Token::RParen);
                        enum_values.push((value_name,Some(parsed_type)));
                    }else {
                        enum_values.push((value_name,None))
                    }
                }
                Token::Pub => {
                    if let Token::Func = self.next() {
                        let name = self.identifier();
                        let func = self.parse_func(true);
                        funcs.push((name,true,func));
                    }else{
                        panic!("expect 'func' after 'pub', found token {:?}",self.peek())
                    }
                }
                Token::Func => {
                    let name = self.identifier();
                    let func = self.parse_func(true);
                    funcs.push((name,false,func));
                }
                token => panic!("expect identifier as enum value, found {:?}",token),
            }
        }
        ParsedEnum{
            name: enum_name,
            values: enum_values,
            funcs
        }
    }

    #[inline]
    fn peek(&self) -> &Token{
        self.tokens.get(self.curr).unwrap()
    }
    #[inline]
    fn next(&mut self) -> &Token{
        let token = self.tokens.get(self.curr).unwrap();
        self.curr += 1;
        token
    }
    #[inline]
    fn assert_next(&mut self, token : Token){
        let curr = self.tokens.get(self.curr).unwrap();
        if token.eq(curr) {
            self.curr += 1;
        }else {
            panic!("[assert_next] expect token {:?} in fact {:?}",token,curr)
        }
    }
    #[inline]
    fn test_next(&self, token : Token) -> bool {
        token.eq(self.tokens.get(self.curr).unwrap())
    }
    #[inline]
    fn has_next(&self) -> bool{
        self.curr < self.tokens.len() - 1
    }
    #[inline]
    fn forward(&mut self){
        self.curr+= 1;
    }
    #[inline]
    fn backward(&mut self){
        self.curr -= 1;
    }
    #[inline]
    fn rollback(&mut self, index : usize){
        self.curr = index;
    }

    #[inline]
    fn identifier(&mut self) -> Rc<String>{
        let curr = self.tokens.get(self.curr).unwrap();
        self.curr += 1;
        if let Token::Id(name) = curr {
            return name.clone()
        }else {
            panic!("expect identifier in fact found token {:?}",curr)
        }
    }

    #[inline]
    fn line(&self) -> u16{
        match self.lines.get(self.curr) {
            None => {
                panic!("curr : {} lines len : {}",self.curr,self.lines.len())
            }
            Some(line) => *line
        }
    }

    fn parse_type(&mut self) -> ParsedType{
        if self.test_next(Token::LParen) {
            self.forward();
            let mut vec = Vec::new();
            while self.has_next() {
                match self.next() {
                    Token::Comma => continue,
                    Token::RParen => break,
                    _ => {
                        self.backward();
                        vec.push(self.parse_type())
                    }
                }
            }
            return ParsedType::Tuple(TypeTuple{vec})
        }
        let type_name = self.identifier();
        let mut generic : Option<Vec<ParsedType>> = Option::None;
        if self.has_next() && self.test_next(Token::Lt) {
            let mut vec : Vec<ParsedType> = Vec::new();
            self.forward();
            while self.has_next() {
                vec.push(self.parse_type());
                if self.test_next(Token::Gt) {
                    self.forward();
                    break
                }
                if self.test_next(Token::Comma) {
                    self.forward();
                }
            }
            generic = Some(vec)
        }
        ParsedType::Single(SingleType{
            name: type_name,
            generic
        })
    }

    pub fn new(tokens : Vec<Token>, lines: Vec<u16>, importer : RefCount<Importer>) -> Parser{
        return Parser{
            tokens,
            lines,
            curr : 0,
            classes: Vec::with_capacity(0),
            interfaces: Vec::with_capacity(0),
            enums : Vec::with_capacity(0),
            funcs : Vec::new(),
            imports: Vec::new(),
            importer,
        }
    }
}

pub struct ParseError {
    pub line : u16,
    pub reason : String,
}

impl ParseError {
    pub fn new(line : u16, reason : String) -> ParseError{
        ParseError{
            line,
            reason
        }
    }
}

impl Debug for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Parse error line {} : {}",self.line,self.reason)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Parse error line {} : {}",self.line,self.reason)
    }
}

impl Error for ParseError{}