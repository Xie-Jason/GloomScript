use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use hashbrown::HashMap;
use crate::frontend::ops::{BinOp, LeftValueOp};
use crate::obj::func::GloomFunc;
use crate::obj::gloom_class::{IsPub};
use crate::obj::refcount::RefCount;
use crate::obj::types::{BasicType, DataType};

pub struct ParsedClass{
    pub name : Rc<String>,
    pub parent : Option<Rc<String>>,
    pub impl_interfaces : Vec<Rc<String>>,
    pub fields : Vec<(bool, ParsedType, Rc<String>)>,
    pub funcs : Vec<(bool, Rc<String>, ParsedFunc)>
}

impl Debug for ParsedClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Class {}:{:?} impl{:?} {:?} {:?}",self.name,self.parent,self.impl_interfaces,self.fields,self.funcs)
    }
}

pub struct ParsedInterface{
    pub name : Rc<String>,
    pub parents : Vec<Rc<String>>,
    pub funcs : Vec<(Rc<String>, Vec<ParsedType>, Option<ParsedType>)>
}

impl Debug for ParsedInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Interface {}:{:?} {:?}",self.name,self.parents,self.funcs)
    }
}

pub struct ParsedEnum{
    pub name : Rc<String>,
    pub values : Vec<(Rc<String>, Option<ParsedType>)>,
    pub funcs : Vec<(Rc<String>, IsPub, ParsedFunc)>
}

impl Debug for ParsedEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Enum {} {:?} {:?}",self.name,self.values,self.funcs)
    }
}

pub type Line = u16;

// 16byte
#[derive(Debug)]
pub enum Statement{
    Let(Box<(Var, Option<ParsedType>, Expression, Line)>),
    Static(Box<(Var, Option<ParsedType>, Expression)>),
    PubStatic(Box<(Var, Option<ParsedType>, Expression)>),

    LeftValueOp(Box<(LeftValue,LeftValueOp)>),

    Expr(Expression,u16),
    Discard(Expression,u16),

    Continue(u16),
    Break(Expression,u16),
    Return(Expression,u16),
    IfResult(Expression,u16),
}

// 16byte
#[derive(Debug)]
pub enum Expression{
    None,
    // 字面常量 literal constant value
    Int(i64),
    Num(f64),
    Char(char),
    Bool(bool),
    Str(Rc<String>),

    Var(Box<Var>),
    Tuple(Box<Vec<Expression>>),
    Array(Box<(Vec<Expression>,BasicType,bool)>), // is array to queue
    Construct(Box<Construction>),

    // 二元操作 binary operation
    BinaryOp(Box<BinOpVec>),

    // 一元操作 unary operation
    Cast(Box<(Expression,ParsedType,DataType)>),
    NegOp(Box<Expression>),
    NotOp(Box<Expression>),

    // 条件控制 condition control
    IfElse(Box<IfElse>),

    // 循环控制 loop control
    While(Box<WhileLoop>),
    For(RefCount<ForLoop>),

    // 模式匹配 pattern match
    Match(RefCount<(Expression, Vec<(Expression, Vec<Statement>)>)>),

    // 链式的成员变量访问和函数调用 field access and function call
    Chain(Box<(Expression,Vec<Chain>)>),

    // 类函数定义 func-like define
    Func(Box<FuncExpr>)
}

impl Expression {
    #[inline]
    pub fn is_none(&self) -> bool{
        match self {
            Expression::None => true,
            _ => false
        }
    }
    #[inline]
    pub fn is_array_literal(&self) -> bool{
        if let Expression::Array(_) = self{
            true
        }else{
            false
        }
    }
}

#[derive(Debug,Clone)]
pub enum Var{
    Name(Rc<String>),

    LocalInt(u16,u8),
    LocalNum(u16,u8),
    LocalChar(u16,u8),
    LocalBool(u16,u8),
    LocalRef(u16),

    StaticInt(u16,u8),
    StaticNum(u16,u8),
    StaticChar(u16,u8),
    StaticBool(u16,u8),
    StaticRef(u16),

    Class(u16),
    Enum(u16),
    Interface(u16),
    BuiltinType(u16),

    DirectFn(u16)
}

impl Var {
    pub fn name(&self) -> Rc<String> {
        match self{
            Var::Name(name) => name.clone(),
            var => panic!("{:?}",var)
        }
    }
    #[inline]
    pub fn new_local(slot_idx : u16, sub_idx : u8, basic_type : BasicType) -> Var {
        match basic_type {
            BasicType::Int => Var::LocalInt(slot_idx,sub_idx),
            BasicType::Num => Var::LocalNum(slot_idx,sub_idx),
            BasicType::Char => Var::LocalChar(slot_idx,sub_idx),
            BasicType::Bool => Var::LocalBool(slot_idx,sub_idx),
            BasicType::Ref => Var::LocalRef(slot_idx),
        }
    }
    #[inline]
    pub fn new_static(slot_idx : u16, sub_idx : u8, basic_type : BasicType) -> Var {
        match basic_type {
            BasicType::Int => Var::StaticInt(slot_idx,sub_idx),
            BasicType::Num => Var::StaticNum(slot_idx,sub_idx),
            BasicType::Char => Var::StaticChar(slot_idx,sub_idx),
            BasicType::Bool => Var::StaticBool(slot_idx,sub_idx),
            BasicType::Ref => Var::StaticRef(slot_idx),
        }
    }
}

pub struct IfElse{
    pub branches: Vec<IfBranch>,
    // else 分支的条件为 Expr::Bool(true)
    // the condition of else branch is Expr::Bool(true)
}

impl Debug for IfElse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.branches)
    }
}

#[derive(Debug)]
pub struct IfBranch{
    pub condition : Expression,
    pub statements : Vec<Statement>,
    pub drop_vec : Vec<u16>,
    pub line : Line,
}

pub struct BinOpVec{
    pub left : Expression,
    pub vec  : Vec<(BinOp,Expression)>
}

impl Debug for BinOpVec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?} {:?}",self.left,self.vec)
    }
}

#[derive(Debug)]
pub enum VarId {
    Index(u16,u8),
    Name(Rc<String>)
}

impl VarId {
    #[inline]
    pub fn name(&self) -> Rc<String>{
        match self {
            VarId::Index(_,_) => panic!(),
            VarId::Name(name) => name.clone()
        }
    }
    #[inline]
    pub fn index(&self) -> (u16,u8){
        match self {
            VarId::Index(i1, i2) => (*i1,*i2),
            VarId::Name(_) => panic!()
        }
    }
}

pub enum ParsedType{
    Single(SingleType),
    Tuple(TypeTuple),
    MySelf
}

impl Debug for ParsedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsedType::Single(tp) => write!(f,"{:?}",tp),
            ParsedType::Tuple(tp) => write!(f,"{:?}",tp),
            ParsedType::MySelf => { write!(f,"Self") }
        }
    }
}

pub struct SingleType{
    pub name : Rc<String>,
    pub generic : Option<Vec<ParsedType>>
}
impl Debug for SingleType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}<{:?}>",self.name,self.generic)
    }
}

pub struct TypeTuple{
    pub vec : Vec<ParsedType>
}

impl Debug for TypeTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.vec)
    }
}

pub struct WhileLoop {
    pub condition : Expression,
    pub statements : Vec<Statement>,
    pub drop_vec : Vec<u16>,
    pub line : Line,
    // 如果循环中有局部变量声明 那么它们应当在每次循环结束时被析构
    // if some local variables declare in loop, them should be dropped after every loop
}

impl Debug for WhileLoop {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"while {:?} {:?}",self.condition,self.statements)
    }
}

pub struct ForLoop{
    pub var : Var,
    pub for_iter : ForIter,
    pub statements : Vec<Statement>
}

#[derive(Debug)]
pub enum ForIter{
    Num(Expression,Expression,Expression),
    Iter(Expression)
}

impl Debug for ForLoop {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"for {:?} in {:?} {:?}",self.var,self.for_iter,self.statements)
    }
}

#[derive(Debug)]
pub enum FuncExpr{
    Parsed(ParsedFunc),
    Analysed(RefCount<GloomFunc>)
}

impl FuncExpr {
    pub fn is_parsed(&self) -> bool {
        match self {
            FuncExpr::Parsed(_) => true,
            FuncExpr::Analysed(_) => false,
        }
    }
}

pub struct ParsedFunc {
    pub params : Vec<(Rc<String>, ParsedType)>,
    pub body : Vec<Statement>,
    pub return_type : Option<ParsedType>
}

impl Debug for ParsedFunc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"func{:?}->{:?} {:?}",self.params,self.return_type,self.body)
    }
}

#[derive(Debug)]
pub struct MatchDef{
    pub matched : Expression,
    pub branches : Vec<(Expression,Vec<Statement>)>
}

#[derive(Debug)]
pub enum ExprType{
    Parsed(ParsedType),
    Analyzed(DataType)
}



#[derive(Debug)]
pub struct Construction{
    pub class_type : ExprType,
    pub map : HashMap<String,Expression>
}

pub enum Chain{
    // field of object, func of meta type
    Access(VarId,BasicType),

    // static func, caller : meta type
    // non-static func, caller : object
    FnCall{
        func : VarId,
        need_self : bool,
        args : Vec<Expression>
    },

    // expr is func type
    Call(Vec<Expression>)
}

impl Debug for Chain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Access(i,t) => {
                write!(f,"->{:?}<{:?}>",i.index(),t)
            }
            Chain::FnCall { func, need_self, args } => {
                write!(f,"func[{:?}] {:?}",func,args)
            }
            Chain::Call(call) => {
                write!(f,"call({:?})",call)
            }
        }

    }
}

#[derive(Debug)]
pub enum LeftValue{
    Var(Var),
    Chain(Expression,Vec<Chain>)
}

// 用来报错时打印详细信息 used for print details when error occurs
pub enum SyntaxType{
    Let,
    Static,
    PubStatic,
    Expr,
    Discard,
    Assign,
    While,
    IfElseBranch,
    Break,
    Return,
    Match,
    Chain
}

impl Debug for SyntaxType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",match self {
            SyntaxType::Let => "let",
            SyntaxType::Static => "static",
            SyntaxType::PubStatic => "pub static",
            SyntaxType::Expr => "expression",
            SyntaxType::Discard => "discarded expr",
            SyntaxType::Assign => "assign",
            SyntaxType::While => "while",
            SyntaxType::IfElseBranch => "if-else branch",
            SyntaxType::Break => "break",
            SyntaxType::Return => "return",
            SyntaxType::Match => "match",
            SyntaxType::Chain => "chain operation"
        })
    }
}