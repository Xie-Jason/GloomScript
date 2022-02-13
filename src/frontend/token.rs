use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(PartialEq)]
pub enum Token {
    Int(i64),
    Num(f64),
    Id(Rc<String>),
    Str(Rc<String>),
    Char(char),
    Bool(bool),

    Plus,
    // +
    Sub,
    // -
    Mul,
    // *
    Div, // /

    Eq,
    // =
    Eqs,
    // ==
    Gt,
    // >
    Lt,
    // >
    GtEq,
    // >=
    LtEq,
    // <=
    NotEq, // !=

    PlusEq,
    // +=
    SubEq,
    // -=
    PlusPlus,
    // ++
    SubSub, // --

    LParen,
    // (
    RParen, // )

    LBrace,
    // {
    RBrace, // }

    LBracket,
    // [
    RBracket, // ]

    Semi,
    // ;
    Comma,
    // ,
    Not,
    // !
    Dot,
    // .
    Colon, // :

    // key words
    Let,
    Return,
    Func,
    If,
    Else,
    While,
    For,
    And,
    Or,
    Break,
    Continue,
    Class,
    Interface,
    Import,
    Match,
    Arrow,
    Underline,
    Impl,
    In,
    Pub,
    Static,
    Enum,
    As,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String;
        write!(
            f,
            "{}",
            match self {
                Token::Plus => "'+'",
                Token::Sub => "'-'",
                Token::Mul => "'*'",
                Token::Div => "'/'",
                Token::Eq => "'='",
                Token::Eqs => "'=='",
                Token::Gt => "'>'",
                Token::Lt => "'<'",
                Token::GtEq => "'>='",
                Token::LtEq => "'<='",
                Token::NotEq => "'!='",
                Token::PlusEq => "'+='",
                Token::SubEq => "'-='",
                Token::PlusPlus => "'++'",
                Token::SubSub => "'--'",
                Token::LParen => "'('",
                Token::RParen => "')'",
                Token::LBrace => "'{'",
                Token::RBrace => "'}'",
                Token::LBracket => "'['",
                Token::RBracket => "']'",
                Token::Semi => "';'",
                Token::Comma => "','",
                Token::Not => "'!'",
                Token::Dot => "'.'",
                Token::Colon => "':'",
                Token::Let => "'let'",
                Token::Return => "'return'",
                Token::Func => "'func'",
                Token::If => "'if'",
                Token::Else => "'else'",
                Token::While => "'while'",
                Token::For => "'for'",
                Token::And => "'&&'",
                Token::Or => "'||'",
                Token::Break => "'break'",
                Token::Continue => "'continue'",
                Token::Class => "'class'",
                Token::Interface => "'interface'",
                Token::Import => "'import'",
                Token::Match => "'match'",
                Token::Arrow => "'=>'",
                Token::Underline => "'_'",
                Token::Impl => "'impl'",
                Token::In => "'in'",
                Token::Pub => "'pub'",
                Token::Static => "'static'",
                Token::Enum => "'enum'",
                Token::As => "'as'",

                Token::Int(n) => {
                    s = format!("'{}'", n);
                    s.as_str()
                }
                Token::Num(n) => {
                    s = format!("'{}'", n);
                    s.as_str()
                }
                Token::Id(n) => {
                    s = format!("'{}'", n);
                    s.as_str()
                }
                Token::Str(n) => {
                    s = format!("\"{}\"", n);
                    s.as_str()
                }
                Token::Char(n) => {
                    s = format!("'{}'", n);
                    s.as_str()
                }
                Token::Bool(n) => {
                    s = format!("'{}'", n);
                    s.as_str()
                }
            }
        )
    }
}
