use std::fmt::{Display, Formatter};
use crate::frontend::ast::Expression;

#[derive(Debug)]
pub enum LeftValueOp{
    Assign(Expression),
    PlusEq(Expression),
    SubEq(Expression),
    PlusOne,
    SubOne,
}

#[derive(Debug,Copy,Clone)]
pub enum BinOp {
    // calc : num to num
    Plus,   // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    // compare : num to bool
    Gt,     // >
    Lt,     // <
    GtEq,   // >=
    LtEq,   // <=
    // eqs : basic/ref to bool
    Eqs,    // ==
    NotEq,  // !=
    // logic : bool to bool
    And,    // && &
    Or,     // || |
}

pub enum BinOpType{
    Calculate,
    Compare,
    Equal,
    Logic
}

impl BinOp {
    #[inline]
    pub fn to_type(&self) -> BinOpType{
        match self {
            BinOp::Plus => BinOpType::Calculate,
            BinOp::Sub => BinOpType::Calculate,
            BinOp::Mul => BinOpType::Calculate,
            BinOp::Div => BinOpType::Calculate,
            BinOp::Gt => BinOpType::Compare,
            BinOp::Lt => BinOpType::Compare,
            BinOp::GtEq => BinOpType::Compare,
            BinOp::LtEq => BinOpType::Compare,
            BinOp::Eqs => BinOpType::Equal,
            BinOp::NotEq => BinOpType::Equal,
            BinOp::And => BinOpType::Logic,
            BinOp::Or => BinOpType::Logic
        }
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",match self {
            BinOp::Plus => "'+'",
            BinOp::Sub => "'-'",
            BinOp::Mul => "'*'",
            BinOp::Div => "'/'",
            BinOp::Gt => "'>'",
            BinOp::Lt => "'<'",
            BinOp::GtEq => "'>='",
            BinOp::LtEq => "'<='",
            BinOp::Eqs => "'=='",
            BinOp::NotEq => "'!='",
            BinOp::And => "'&&'",
            BinOp::Or => "'||'",
        })
    }
}