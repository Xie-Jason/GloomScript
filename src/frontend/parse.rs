use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::frontend::ast::{
    BinOpVec, Chain, Construction, ExprType, Expression, ForIter, ForLoop, FuncExpr, IfBranch,
    IfElse, LeftValue, ParsedClass, ParsedEnum, ParsedFunc, ParsedInterface, ParsedType,
    SingleType, Statement, TypeTuple, Var, VarId, WhileLoop,
};
use crate::frontend::import::Importer;
use crate::frontend::ops::{BinOp, LeftValueOp};
use crate::frontend::script::ParsedFile;
use crate::frontend::token::Token;
use crate::obj::refcount::RefCount;
use crate::obj::types::{BasicType, DataType, RefType};

pub struct Parser {
    tokens: Vec<Token>,
    curr: usize,
    classes: Vec<(ParsedClass, bool)>,
    interfaces: Vec<(ParsedInterface, bool)>,
    enums: Vec<(ParsedEnum, bool)>,
    funcs: Vec<(Rc<String>, ParsedFunc, bool)>,
    imports: Vec<ParsedFile>,
    importer: RefCount<Importer>,
    path: String,
    pub lines: Vec<u16>,
}

impl Parser {
    pub fn parse(mut self) -> Result<ParsedFile, ParseError> {
        let vec = self.statements()?;
        Result::Ok(ParsedFile {
            imports: self.imports,
            classes: self.classes,
            interfaces: self.interfaces,
            funcs: self.funcs,
            enums: self.enums,
            statements: vec,
            path: self.path,
            index: 0,
        })
    }

    fn statements(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        while self.has_next() && !self.test_next(Token::RBrace) {
            let token = self.next();
            let statement = match token {
                Token::Continue => {
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    Statement::Continue(self.line())
                }
                Token::Return => {
                    let index = self.curr;
                    let line = self.line();
                    let result_statement = match self.expr() {
                        Err(_) => {
                            self.rollback(index);
                            Statement::Return(Expression::None, line)
                        }
                        Ok(expr) => Statement::Return(expr, line),
                    };
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    result_statement
                }
                Token::Break => {
                    let line = self.line();
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    Statement::Break(line)
                }
                // 局部变量声明 Let
                Token::Let => {
                    let line = self.line();
                    let var_name = self.identifier()?;
                    let mut parsed_type: Option<ParsedType> = None;
                    if self.test_next(Token::Eq) {
                        self.forward();
                    } else if self.test_next(Token::Colon) {
                        self.forward();
                        parsed_type = Some(self.parse_type()?);
                        self.assert_next(Token::Eq)
                            .map_err(|e| e.msg("expect a '=' after 'let' and variable name"))?;
                    } else {
                        parsed_type = Some(self.parse_type()?);
                        self.assert_next(Token::Eq).map_err(|e| {
                            e.msg("expect a '=' after variable type in 'let' statement")
                        })?;
                    }
                    let expr = self.expr()?;
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    Statement::Let(Box::new((Var::Name(var_name), parsed_type, expr, line)))
                }
                Token::Static => {
                    let var_name = self.identifier()?;
                    let mut parsed_type: Option<ParsedType> = None;
                    if self.test_next(Token::Eq) {
                        self.forward();
                    } else if self.test_next(Token::Colon) {
                        self.forward();
                        parsed_type = Some(self.parse_type()?);
                        self.assert_next(Token::Eq)
                            .map_err(|e| e.msg("expect a '=' after 'static' and variable name"))?;
                    } else {
                        parsed_type = Some(self.parse_type()?);
                        self.assert_next(Token::Eq).map_err(|e| {
                            e.msg("expect a '=' after variable type in 'static' statement")
                        })?;
                    }
                    let expr = self.expr()?;
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    Statement::Static(Box::new((Var::Name(var_name), parsed_type, expr)))
                }
                Token::RBrace => {
                    self.backward();
                    break;
                }
                Token::Comma => {
                    self.backward();
                    break;
                }
                Token::Semi => {
                    continue;
                }
                Token::Import => {
                    let importer = self.importer.clone();
                    match self.next().clone() {
                        Token::Id(lib) => {
                            Importer::import_std_lib(lib.as_str(), importer).unwrap();
                        }
                        Token::Str(path) => {
                            let mut path = path.to_string();
                            let start_with_point = path.starts_with('.');
                            if start_with_point {
                                path.remove(0);
                                let mut new_path = self.path.clone();
                                let deli_location = if let Some(idx) = new_path.rfind('\\') {
                                    idx
                                }else if let Some(idx) = new_path.rfind('/') {
                                    idx
                                }else {
                                    return Result::Err(ParseError::new(
                                        self.line(),
                                        format!("imported file path {:?} not formatted path",path)
                                    ))
                                };
                                let len = new_path.len();
                                for _ in deli_location..len {
                                    new_path.remove(new_path.len() - 1);
                                }
                                new_path.push_str(path.as_str());
                                path = new_path;
                            }
                            if let Some(parsed_file) =
                                Importer::import_file(path, importer).unwrap()
                            {
                                self.imports.push(parsed_file);
                            }
                        }
                        token => return Result::Err(ParseError::new(
                            self.line(),
                            format!(
                                "expect a string literal or a identifier after 'import', found {}",
                                token
                            ),
                        )),
                    }
                    continue;
                }
                // public declaration
                Token::Pub => match self.next().clone() {
                    Token::Class => {
                        let class = self.parse_class()?;
                        self.classes.push((class, true));
                        continue;
                    }
                    Token::Interface => {
                        let interface = self.parse_interface()?;
                        self.interfaces.push((interface, true));
                        continue;
                    }
                    Token::Enum => {
                        let parsed_enum = self.parse_enum()?;
                        self.enums.push((parsed_enum, true));
                        continue;
                    }
                    Token::Func => {
                        let func_name = self.identifier()?;
                        let func = self.parse_func(false)?;
                        self.funcs.push((func_name, func, true));
                        continue;
                    }
                    Token::Static => {
                        let var_name = self.identifier()?;
                        let mut parsed_type: Option<ParsedType> = None;
                        if self.test_next(Token::Eq) {
                            self.forward();
                        } else if self.test_next(Token::Colon) {
                            self.forward();
                            parsed_type = Some(self.parse_type()?);
                            self.assert_next(Token::Eq).map_err(|e| {
                                e.msg("expect a '=' after 'pub static' and variable name")
                            })?;
                        } else {
                            parsed_type = Some(self.parse_type()?);
                            self.assert_next(Token::Eq).map_err(|e| {
                                e.msg("expect a '=' after variable type in 'pub static' statement")
                            })?;
                        }
                        let expr = self.expr()?;
                        if self.has_next() && self.test_next(Token::Semi) {
                            self.forward();
                        }
                        Statement::PubStatic(Box::new((Var::Name(var_name), parsed_type, expr)))
                    }
                    token => {
                        let line = *self.lines.get(self.curr).unwrap();
                        return Result::Err(ParseError::new(
                            line,
                            format!("expect class interface enum or func, found {:?}", token),
                        ));
                    }
                },
                // private declaration
                Token::Class => {
                    let class = self.parse_class()?;
                    self.classes.push((class, false));
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    continue;
                }
                Token::Interface => {
                    let interface = self.parse_interface()?;
                    self.interfaces.push((interface, false));
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    continue;
                }
                Token::Enum => {
                    let parsed_enum = self.parse_enum()?;
                    self.enums.push((parsed_enum, false));
                    if self.has_next() && self.test_next(Token::Semi) {
                        self.forward();
                    }
                    continue;
                }
                Token::Func => {
                    if self.test_next(Token::LParen) {
                        // nameless func / closure / lambda expression
                        // let curr pointer to Token::Func
                        self.backward();
                        let line = self.line();
                        let expr = self.expr()?;
                        if self.has_next() && self.test_next(Token::Semi) {
                            self.forward();
                            Statement::Discard(expr, line)
                        } else {
                            Statement::Expr(expr, line)
                        }
                    } else {
                        // function declaration
                        let func_name = self.identifier()?;
                        let func = self.parse_func(false)?;
                        self.funcs.push((func_name, func, false));
                        continue;
                    }
                }
                // while循环 while-loop
                Token::While => {
                    let line = self.line();
                    let condition = self.expr()?;
                    self.assert_next(Token::LBrace)?;
                    let statements = self.statements()?;
                    self.assert_next(Token::RBrace)?;
                    Statement::While(Box::new(WhileLoop {
                        condition,
                        statements,
                        drop_slots: Vec::with_capacity(0),
                        line,
                        return_void: false,
                    }))
                }
                // for-循环 for-loop
                Token::For => {
                    let var_name = self.identifier()?;
                    let line = self.line();
                    self.assert_next(Token::In)?;
                    let check_point = self.curr;
                    let expr = match self.expr() {
                        Ok(expr) => expr,
                        Err(_) => {
                            self.rollback(check_point);
                            Expression::Var(Box::new(Var::Name(self.identifier()?)))
                        }
                    };
                    let for_iter: ForIter = if let Expression::Tuple(mut tuple) = expr {
                        let vec = tuple.deref_mut();
                        if vec.len() == 2 {
                            let end = vec.pop().unwrap();
                            let start = vec.pop().unwrap();
                            ForIter::Range(start, end, Expression::Int(1))
                        } else if vec.len() == 3 {
                            let step = vec.pop().unwrap();
                            let end = vec.pop().unwrap();
                            let start = vec.pop().unwrap();
                            ForIter::Range(start, end, step)
                        } else {
                            ForIter::Iter(Expression::Tuple(std::mem::replace(
                                &mut tuple,
                                Box::new(Vec::with_capacity(0)),
                            )))
                        }
                    } else {
                        ForIter::Iter(expr)
                    };
                    self.assert_next(Token::LBrace)?;
                    let statements = self.statements()?;
                    self.assert_next(Token::RBrace)?;
                    Statement::For(Box::new(ForLoop {
                        var: Var::Name(var_name),
                        for_iter,
                        statements,
                        drop_slots: Vec::new(),
                        line,
                        return_void: false,
                    }))
                }
                _ => {
                    self.backward();
                    let line = self.line();
                    let expr = self.expr()?;
                    if self.has_next() {
                        let token = self.next();
                        match token {
                            Token::Semi => Statement::Discard(expr, line),
                            Token::Eq
                            | Token::SubEq
                            | Token::PlusEq
                            | Token::SubSub
                            | Token::PlusPlus => {
                                let left_value = match expr {
                                    Expression::Var(var) => LeftValue::Var(*var),
                                    Expression::Chain(chain_box) => {
                                        let (_, chains) = chain_box.deref();
                                        if let Chain::Access(_, _) = chains.last().unwrap() {
                                        } else {
                                            panic!()
                                        }
                                        let (expr, chains) = *chain_box;
                                        LeftValue::Chain(expr, chains)
                                    }
                                    _ => panic!(),
                                };
                                let left_value_op = match token {
                                    Token::Eq => LeftValueOp::Assign(self.expr()?),
                                    Token::SubEq => LeftValueOp::SubEq(self.expr()?),
                                    Token::PlusEq => LeftValueOp::PlusEq(self.expr()?),
                                    Token::SubSub => LeftValueOp::SubOne,
                                    Token::PlusPlus => LeftValueOp::PlusOne,
                                    _ => panic!(),
                                };
                                if self.has_next() && self.test_next(Token::Semi) {
                                    // handle ';'
                                    self.forward();
                                }
                                Statement::LeftValueOp(Box::new((left_value, left_value_op)))
                            }
                            _ => {
                                self.backward();
                                Statement::Expr(expr, line)
                            }
                        }
                    } else {
                        Statement::Expr(expr, line)
                    }
                }
            };
            statements.push(statement)
        }
        Result::Ok(statements)
    }

    fn expr(&mut self) -> Result<Expression, ParseError> {
        let expr = self.medium_expr()?;
        let mut op_vec: Option<Vec<(BinOp, Expression)>> = Option::None;
        while self.has_next() {
            let bin_op_expr: Option<(BinOp, Expression)>;
            match self.next() {
                Token::Plus => bin_op_expr = Some((BinOp::Plus, self.medium_expr()?)),
                Token::Sub => bin_op_expr = Some((BinOp::Sub, self.medium_expr()?)),
                Token::And => bin_op_expr = Some((BinOp::And, self.medium_expr()?)),
                Token::Or => bin_op_expr = Some((BinOp::Or, self.medium_expr()?)),
                _ => {
                    self.backward();
                    break;
                }
            }
            match bin_op_expr {
                Some(op_tuple) => match &mut op_vec {
                    None => {
                        let mut vec = Vec::new();
                        vec.push(op_tuple);
                        op_vec = Some(vec);
                    }
                    Some(vec) => vec.push(op_tuple),
                },
                None => break,
            }
        }
        match op_vec {
            Some(vec) => Result::Ok(Expression::BinaryOp(Box::new(BinOpVec { left: expr, vec }))),
            None => Result::Ok(expr),
        }
    }

    fn medium_expr(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.primary_expr()?;
        if self.has_next() && self.test_next(Token::As) {
            self.forward();
            let parsed_type = self.parse_type()?;
            expr = Expression::Cast(Box::new((expr, parsed_type, DataType::Ref(RefType::Any))));
        }
        let mut op_vec: Option<Vec<(BinOp, Expression)>> = Option::None;
        while self.has_next() {
            let bin_op_expr: Option<(BinOp, Expression)>;
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
                    break;
                }
            }
            match bin_op_expr {
                Some(op_tuple) => match &mut op_vec {
                    None => {
                        let mut vec = Vec::new();
                        vec.push(op_tuple);
                        op_vec = Some(vec);
                    }
                    Some(vec) => {
                        vec.push(op_tuple);
                    }
                },
                None => break,
            }
        }
        match op_vec {
            Some(vec) => Result::Ok(Expression::BinaryOp(Box::new(BinOpVec { left: expr, vec }))),
            None => Result::Ok(expr),
        }
    }

    fn primary_expr(&mut self) -> Result<Expression, ParseError> {
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
                                let field_name = self.identifier()?;
                                self.assert_next(Token::Colon)?;
                                let expr = self.expr()?;
                                fields.push((VarId::Name(field_name), BasicType::Ref, expr));
                            }
                        }
                    }
                    Expression::Construct(Box::new(Construction {
                        class_type: ExprType::Parsed(ParsedType::Single(SingleType {
                            name: class_name,
                            generic: None,
                        })),
                        fields,
                    }))
                } else {
                    Expression::Var(Box::new(Var::Name(var_name)))
                }
            }
            Token::If => {
                let mut branches = Vec::new();
                while self.has_next() {
                    let line = self.line();
                    let condition = self.expr()?;
                    self.assert_next(Token::LBrace)?;
                    let branch = self.statements()?;
                    self.assert_next(Token::RBrace)?;
                    branches.push(IfBranch {
                        condition,
                        statements: branch,
                        drop_vec: Vec::with_capacity(0),
                        line,
                    });
                    if !self.has_next() {
                        break;
                    }
                    if !self.test_next(Token::Else) {
                        break;
                    }
                    self.forward();
                    if self.test_next(Token::If) {
                        // 继续 if(){}else if(){}
                        self.forward();
                        continue;
                    } else if self.test_next(Token::LBrace) {
                        // if(){}else{}
                        self.forward();
                        let line = self.line();
                        branches.push(IfBranch {
                            condition: Expression::Bool(true),
                            statements: self.statements()?,
                            drop_vec: Vec::with_capacity(0),
                            line,
                        });
                        self.assert_next(Token::RBrace)?;
                        break;
                    } else {
                        println!("unexpected token near if-else {:?}", self.peek());
                        break;
                    }
                }
                Expression::IfElse(Box::new(IfElse {
                    branches,
                    return_void: false,
                }))
            }
            // 元组 或 (计算表达式) tuple or (calculation expression)
            Token::LParen => {
                let mut vec: Vec<Expression> = Vec::new();
                while self.has_next() {
                    vec.push(self.expr()?);
                    if self.test_next(Token::RParen) {
                        self.forward();
                        break;
                    } else if self.test_next(Token::Comma) {
                        self.forward();
                    } else {
                        println!(
                            "unexpected token {:?} near tuple parse, line {}",
                            self.peek(),
                            self.line()
                        )
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
                            break;
                        }
                        Token::Comma => {}
                        token => {
                            return Err(ParseError::new(line, format!("unexpect token when parse array, expect ']' or ',', found {:?}", token)));
                        }
                    }
                }
                Expression::Array(Box::new((vec, BasicType::Ref, false)))
            }
            // 匿名函数 Anonymous Function
            // 立即执行函数 immediate exec func
            Token::Func => {
                let func = self.parse_func(false)?;
                Expression::Func(Box::new(FuncExpr::Parsed(func)))
            }
            // 匹配 match
            Token::Match => {
                self.assert_next(Token::LParen)?;
                let expr = self.expr()?;
                self.assert_next(Token::RParen)?;
                self.assert_next(Token::LBrace)?;
                let mut branches = Vec::new();
                while self.has_next() {
                    match self.next() {
                        Token::RBrace => break,
                        _ => {
                            self.backward();
                            let try_matched = self.expr()?;
                            self.assert_next(Token::DoubleArrow)?;
                            if self.test_next(Token::LBrace) {
                                self.forward();
                                let statements = self.statements()?;
                                branches.push((try_matched, statements));
                                self.assert_next(Token::RBrace)?;
                            } else {
                                let statements = self.statements()?;
                                branches.push((try_matched, statements));
                                self.assert_next(Token::Comma)?;
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
                return Result::Err(ParseError::new(
                    line,
                    format!("unexpected token {:?} when parse primary expression", token),
                ));
            }
        };
        if self.has_next() && (self.test_next(Token::Dot) || self.test_next(Token::LParen)) {
            let mut chains = Vec::new();
            while self.has_next() {
                match self.next() {
                    Token::Dot => {
                        // field access
                        let field_name = self.identifier()?;
                        if self.has_next() && self.test_next(Token::LParen) {
                            self.forward();
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
                            chains.push(Chain::FnCall {
                                func: VarId::Name(field_name),
                                args,
                                need_self: false,
                                is_dyn: false,
                            })
                        } else {
                            chains.push(Chain::Access(VarId::Name(field_name), BasicType::Ref));
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
                        break;
                    }
                }
            } // while loop end
            expr = Expression::Chain(Box::new((expr, chains)))
        }
        Result::Ok(expr)
    }

    fn parse_func(&mut self, is_mem_func: bool) -> Result<ParsedFunc, ParseError> {
        self.assert_next(Token::LParen)?;
        let mut param_vec = Vec::new();
        while self.has_next() {
            match self.next() {
                Token::RParen => break,
                Token::Comma => continue,
                _ => {
                    self.backward();
                    if let Token::Id(name) = self.peek() {
                        if is_mem_func && name.as_str().eq("self") {
                            param_vec.push((Rc::new(String::from("self")), ParsedType::MySelf));
                            self.forward();
                            continue;
                        }
                    }
                    let param_type = self.parse_type()?;
                    let param_name = self.identifier()?;
                    param_vec.push((param_name, param_type));
                }
            }
        }
        if self.test_next(Token::SingleArrow) {
            self.forward();
        }
        let mut return_type = Option::None;
        if !self.test_next(Token::LBrace) {
            return_type = Some(self.parse_type()?)
        }
        self.assert_next(Token::LBrace)?;
        let statements = self.statements()?;
        self.assert_next(Token::RBrace)?;
        Result::Ok(ParsedFunc {
            params: param_vec,
            body: statements,
            return_type,
        })
    }

    fn parse_class(&mut self) -> Result<ParsedClass, ParseError> {
        let name = self.identifier()?;
        let mut parent_class = None;
        let mut impl_vec = Vec::with_capacity(0);
        let mut field_vec = Vec::new();
        let mut func_vec = Vec::new();
        // parse inherit
        if self.test_next(Token::Colon) {
            self.forward();
            parent_class = Some(self.identifier()?);
        }
        // parse implementation
        if self.test_next(Token::Impl) {
            self.forward();
            while self.has_next() {
                impl_vec.push(self.identifier()?);
                if self.test_next(Token::Comma) {
                    self.forward();
                }
                if self.test_next(Token::LBrace) {
                    break;
                } else {
                    panic!()
                }
            }
        }
        self.assert_next(Token::LBrace)?;
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
            if self.test_next(Token::Func) {
                // function
                self.forward();
                let func_name = self.identifier()?;
                let parsed_func = self.parse_func(true)?;
                func_vec.push((is_public, func_name, parsed_func));
            } else {
                // field
                let parsed_type = self
                    .parse_type()
                    .map_err(|e| e.msg("expect a field type"))?;
                let name = self
                    .identifier()
                    .map_err(|e| e.msg("expect a field name"))?;
                field_vec.push((is_public, parsed_type, name));
            }
        }
        Result::Ok(ParsedClass {
            name,
            parent: parent_class,
            impl_interfaces: impl_vec,
            fields: field_vec,
            funcs: func_vec,
        })
    }

    fn parse_interface(&mut self) -> Result<ParsedInterface, ParseError> {
        let name = self.identifier()?;
        let mut parents = Vec::new();
        if self.test_next(Token::Colon) {
            self.forward();
            while self.has_next() {
                match self.next() {
                    Token::LBrace => break,
                    Token::Comma => continue,
                    Token::Id(id) => parents.push(id.clone()),
                    token => {
                        panic!("unexpect token {:?} when parse interface parent define, expect identifier as parent interface name", token)
                    }
                }
            }
        } else {
            self.assert_next(Token::LBrace)?;
        }
        let mut funcs = Vec::new();
        while self.has_next() {
            match self.next().clone() {
                Token::RBrace => {
                    break;
                }
                Token::Func => {
                    let func_name = self.identifier()?;
                    let mut params = Vec::new();
                    self.assert_next(Token::LParen)
                        .map_err(|err| err.msg("need a '(' after 'func'"))?;
                    // gloom self
                    if let Token::Id(name) = self.peek() {
                        if name.as_str().eq("self") {
                            params.push(ParsedType::MySelf);
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
                                let parsed_type = self
                                    .parse_type()
                                    .map_err(|e| e.msg("expect a param type"))?;
                                self.identifier()
                                    .map_err(|e| e.msg("expect a param name"))?;
                                params.push(parsed_type);
                            }
                        }
                    }
                    // parse return type
                    let mut return_type = Option::None;
                    if self.test_next(Token::SingleArrow) {
                        self.forward();
                    }
                    if !self.test_next(Token::Func) && !self.test_next(Token::RBrace) {
                        return_type = Option::Some(self.parse_type()?);
                    }
                    funcs.push((func_name, params, return_type));
                }
                token => {
                    return Result::Err(ParseError::new(
                        self.line(),
                        format!("expect token 'func', found {:?}", token),
                    ))
                }
            }
        }
        Result::Ok(ParsedInterface {
            name,
            parents,
            funcs,
        })
    }

    fn parse_enum(&mut self) -> Result<ParsedEnum, ParseError> {
        let enum_name = self.identifier()?;
        let mut enum_values = Vec::new();
        let mut funcs = Vec::new();
        self.assert_next(Token::LBrace)?;
        while self.has_next() {
            match self.next().clone() {
                Token::RBrace => break,
                Token::Id(id) => {
                    let value_name = id.clone();
                    if self.test_next(Token::LParen) {
                        // TODO 改语法
                        self.assert_next(Token::LParen)
                            .map_err(|err| err.msg("expect a '('"))?;
                        let parsed_type = self.parse_type()?;
                        self.assert_next(Token::RParen)?;
                        enum_values.push((value_name, Some(parsed_type)));
                    } else {
                        enum_values.push((value_name, None))
                    }
                }
                Token::Pub => {
                    if let Token::Func = self.next() {
                        let name = self.identifier()?;
                        let func = self.parse_func(true)?;
                        funcs.push((name, true, func));
                    } else {
                        return Result::Err(ParseError::new(
                            self.line(),
                            format!("expect 'func' after 'pub', found token {:?}", self.peek()),
                        ));
                    }
                }
                Token::Func => {
                    let name = self.identifier()?;
                    let func = self.parse_func(true)?;
                    funcs.push((name, false, func));
                }
                token => {
                    return Result::Err(ParseError::new(
                        self.line(),
                        format!("expect identifier as enum value, found {:?}", token),
                    ))
                }
            }
        }
        Result::Ok(ParsedEnum {
            name: enum_name,
            values: enum_values,
            funcs,
        })
    }

    #[inline]
    fn peek(&self) -> &Token {
        self.tokens.get(self.curr).unwrap()
    }
    #[inline]
    fn next(&mut self) -> &Token {
        let token = self.tokens.get(self.curr).unwrap();
        self.curr += 1;
        token
    }
    #[inline]
    fn assert_next(&mut self, token: Token) -> Result<(), ParseError> {
        let curr = self.tokens.get(self.curr).unwrap();
        if token.eq(curr) {
            self.curr += 1;
            Result::Ok(())
        } else {
            Result::Err(ParseError::new(
                self.line(),
                format!("[assert_next] expect token {:?} in fact {:?}", token, curr),
            ))
        }
    }
    #[inline]
    fn test_next(&self, token: Token) -> bool {
        token.eq(self.tokens.get(self.curr).unwrap())
    }
    #[inline]
    fn has_next(&self) -> bool {
        self.curr + 1 < self.tokens.len()
    }
    #[inline]
    fn forward(&mut self) {
        self.curr += 1;
    }
    #[inline]
    fn backward(&mut self) {
        self.curr -= 1;
    }
    #[inline]
    fn rollback(&mut self, index: usize) {
        self.curr = index;
    }

    #[inline]
    fn identifier(&mut self) -> Result<Rc<String>, ParseError> {
        let curr = self.tokens.get(self.curr).unwrap();
        self.curr += 1;
        if let Token::Id(name) = curr {
            Result::Ok(name.clone())
        } else {
            Result::Err(ParseError::new(
                self.line(),
                format!("expect identifier in fact found token {:?}", curr),
            ))
        }
    }

    #[inline]
    fn line(&self) -> u16 {
        match self.lines.get(self.curr) {
            None => {
                panic!("curr : {} lines len : {}", self.curr, self.lines.len())
            }
            Some(line) => *line,
        }
    }

    fn parse_type(&mut self) -> Result<ParsedType, ParseError> {
        if self.test_next(Token::LParen) {
            self.forward();
            let mut vec = Vec::new();
            while self.has_next() {
                match self.next() {
                    Token::Comma => continue,
                    Token::RParen => break,
                    _ => {
                        self.backward();
                        vec.push(self.parse_type()?)
                    }
                }
            }
            return Result::Ok(ParsedType::Tuple(TypeTuple { vec }));
        }
        let type_name = self
            .identifier()
            .map_err(|err| err.msg("expect a type identifier"))?;
        let mut generic: Option<Vec<ParsedType>> = Option::None;
        if self.has_next() && self.test_next(Token::Lt) {
            let mut vec: Vec<ParsedType> = Vec::new();
            self.forward();
            while self.has_next() {
                vec.push(self.parse_type()?);
                if self.test_next(Token::Gt) {
                    self.forward();
                    break;
                }
                if self.test_next(Token::Comma) {
                    self.forward();
                }
            }
            generic = Some(vec)
        }
        Result::Ok(ParsedType::Single(SingleType {
            name: type_name,
            generic,
        }))
    }

    pub fn new(
        tokens: Vec<Token>,
        lines: Vec<u16>,
        importer: RefCount<Importer>,
        path: String,
    ) -> Parser {
        return Parser {
            tokens,
            lines,
            curr: 0,
            classes: Vec::with_capacity(0),
            interfaces: Vec::with_capacity(0),
            enums: Vec::with_capacity(0),
            funcs: Vec::new(),
            imports: Vec::new(),
            importer,
            path,
        };
    }
}

pub struct ParseError {
    pub line: u16,
    pub reason: String,
}

impl ParseError {
    pub fn new(line: u16, reason: String) -> ParseError {
        ParseError { line, reason }
    }

    pub fn msg(mut self, msg: &'static str) -> ParseError {
        self.reason.push_str(msg);
        ParseError {
            line: self.line,
            reason: self.reason,
        }
    }
}

impl Debug for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error line {} : {}", self.line, self.reason)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error line {} : {}", self.line, self.reason)
    }
}

impl Error for ParseError {}
