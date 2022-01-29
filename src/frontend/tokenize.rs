use std::panic::panic_any;
use std::rc::Rc;
use std::str::FromStr;
use crate::frontend::token::Token;

pub struct Tokenizer{
    src  : Vec<u8>,
    curr : usize,
    line : u16,
}

impl Tokenizer{
    pub fn tokenize(&mut self) -> (Vec<Token>, Vec<u16>) {
        let mut tokens : Vec<Token> = Vec::with_capacity(self.src.len() / 2);
        let mut lines : Vec<u16> = Vec::with_capacity(tokens.len());
        while self.curr < self.src.len() {
            let byte = *self.src.get(self.curr).unwrap();
            match byte {
                b'\n' => {
                    self.line += 1;
                }
                b'(' => {
                    tokens.push(Token::LParen);
                    lines.push(self.line);
                },
                b')' => {
                    tokens.push(Token::RParen);
                    lines.push(self.line);
                }
                b'*' => {
                    tokens.push(Token::Mul);
                    lines.push(self.line);
                },
                b';' => {
                    tokens.push(Token::Semi);
                    lines.push(self.line);
                }
                b'{' => {
                    tokens.push(Token::LBrace);
                    lines.push(self.line);
                }
                b'}' => {
                    tokens.push(Token::RBrace);
                    lines.push(self.line);
                }
                b',' => {
                    tokens.push(Token::Comma);
                    lines.push(self.line);
                }
                b'.' => {
                    tokens.push(Token::Dot);
                    lines.push(self.line);
                }
                b'[' => {
                    tokens.push(Token::LBracket);
                    lines.push(self.line);
                }
                b']' => {
                    tokens.push(Token::RBracket);
                    lines.push(self.line);
                }
                b':' => {
                    tokens.push(Token::Colon);
                    lines.push(self.line);
                }
                b'+' => {
                    tokens.push(match self.peek_u8() {
                        b'+' => {
                            self.curr += 1;
                            Token::PlusPlus
                        },
                        b'=' => {
                            self.curr += 1;
                            Token::PlusEq
                        }
                        _ => Token::Plus
                    });
                    lines.push(self.line);
                }
                b'-' => {
                    tokens.push(match self.peek_u8() {
                        b'-' => {
                            self.curr += 1;
                            Token::SubSub
                        },
                        b'=' => {
                            self.curr += 1;
                            Token::SubEq
                        }
                        _ => Token::Sub
                    });
                    lines.push(self.line);
                }
                b'!' => {
                    tokens.push(if self.peek_u8() == b'='{
                        self.curr += 1;
                        Token::NotEq
                    }else {
                        Token::Not
                    });
                    lines.push(self.line);
                }
                b'>' => {
                    if self.peek_u8() == b'=' {
                        self.curr += 1;
                        tokens.push(Token::GtEq)
                    }else {
                        tokens.push(Token::Gt)
                    };
                    lines.push(self.line);
                },
                b'<' => {
                    if self.peek_u8() == b'=' {
                        self.curr += 1;
                        tokens.push(Token::LtEq)
                    }else {
                        tokens.push(Token::Lt)
                    };
                    lines.push(self.line);
                },
                b'&' => {
                    if self.peek_u8() == b'&' {
                        self.curr += 1;
                    }
                    tokens.push(Token::And);
                    lines.push(self.line);
                },
                b'|' => {
                    if self.peek_u8() == b'|' {
                        self.curr += 1;
                    }
                    tokens.push(Token::Or);
                    lines.push(self.line);
                },
                b'=' => {
                    if self.peek_u8() == b'=' {
                        self.curr += 1;
                        tokens.push(Token::Eqs);
                    }else if self.peek_u8() == b'>' {
                        self.curr += 1;
                        tokens.push(Token::Arrow);
                    } else {
                        tokens.push(Token::Eq);
                    }
                    lines.push(self.line);
                },
                // 除 或 注释
                b'/' => {
                    match self.peek_u8() {
                        b'/' => {
                            self.curr += 1;
                            self.skip_annotation_line()
                        }
                        b'*' => {
                            self.curr += 1;
                            self.skip_annotation_block()
                        }
                        _ => {
                            tokens.push(Token::Div);
                            lines.push(self.line);
                        }
                    }
                }
                // 字面量字符串
                b'"' => {
                    tokens.push(self.parse_str());
                    lines.push(self.line);
                },
                // 数字
                byte if ( byte >= b'0' && byte <= b'9' )|| byte == b'-'  => {
                    tokens.push(self.parse_num());
                    lines.push(self.line);
                    // 此时self.curr已经指向下一个u8了，不应当+1
                    continue
                }
                b'\'' => {
                    tokens.push(self.parse_char());
                    lines.push(self.line);
                }
                // 标识符
                byte if Self::is_valid_header(byte) => {
                    tokens.push(self.parse_identifier());
                    lines.push(self.line);
                    // 同上
                    continue
                }
                byte if byte <= b' ' => {}
                _ => println!("{}",byte as char),
            }
            self.curr += 1;
        }
        lines.push(self.line+1);
        (tokens,lines)
    }
    fn parse_num(&mut self) -> Token{
        let mut vec: Vec<u8> = Vec::new();
        let mut is_float : bool = false;
        while self.curr < self.src.len(){
            let byte = *self.src.get(self.curr).unwrap();
            if (byte >= b'0' && byte <= b'9') || byte == b'-'  {
                vec.push(byte);
            } else if byte == b'.' && ! is_float {
                vec.push(byte);
                is_float = true;
            } else{
                break;
            }
            self.curr += 1;
        }
        let string = String::from_utf8(vec).unwrap();
        if is_float {
            return Token::Num(f64::from_str(string.as_str()).unwrap())
        }
        Token::Int(i64::from_str(string.as_str()).unwrap())
    }
    fn parse_identifier(&mut self) -> Token{
        let mut vec: Vec<u8> = Vec::new();
        vec.push(*self.src.get(self.curr).unwrap());
        self.curr += 1;
        while self.curr < self.src.len(){
            let byte = *self.src.get(self.curr).unwrap();
            if Self::is_valid_letter(byte) {
                vec.push(byte);
            }else {
                break
            }
            self.curr += 1;
        }
        let id = String::from_utf8(vec).unwrap();
        match id.as_str() {
            "let" => Token::Let,
            "func" => Token::Func,
            "return" => Token::Return,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "class" => Token::Class,
            "interface" => Token::Interface,
            "import" => Token::Import,
            "impl" => Token::Impl,
            "in" => Token::In,
            "pub" => Token::Pub,
            "static" => Token::Static,
            "match" => Token::Match,
            "enum" => Token::Enum,
            "as" => Token::As,
            "_" => Token::Underline,
            _ => Token::Id(Rc::new(id))
        }
    }
    fn parse_str(&mut self) -> Token {
        let mut vec: Vec<u8> = Vec::new();
        while self.curr < self.src.len(){
            self.curr += 1;
            let byte = *self.src.get(self.curr).unwrap();
            if byte == b'"' {
                break
            }else {
                vec.push(byte)
            }
        }
        Token::Str(Rc::new(String::from_utf8(vec).unwrap()))
    }

    fn parse_char(&mut self) -> Token {
        let mut vec: Vec<u8> = Vec::new();
        while self.curr < self.src.len(){
            self.curr += 1;
            let byte = *self.src.get(self.curr).unwrap();
            if byte == b'\'' {
                break
            }else {
                vec.push(byte)
            }
        }
        const INCORRECT_CHAR: &'static str = "incorrect char";
        if vec.len() > 4 {
            panic_any(INCORRECT_CHAR)
        }
        match String::from_utf8(vec) {
            Ok(str) => {
                let chars : Vec<char>= str.chars().collect();
                Token::Char(*chars.get(0).unwrap())
            }
            Err(_) => {
                panic!("{}",INCORRECT_CHAR)
            }
        }
    }

    fn skip_annotation_line(&mut self){
        while self.curr < self.src.len() {
            let byte = *self.src.get(self.curr).unwrap();
            if byte == b'\n' {
                break
            }
            self.curr += 1;
        }
        self.line+=1;
    }
    fn skip_annotation_block(&mut self){
        while self.curr < self.src.len() {
            let byte = *self.src.get(self.curr).unwrap();
            match byte {
                b'\n' => {
                    self.line += 1;
                }
                b'*' if self.peek_u8() == b'/' => {
                    self.curr += 1;
                    break
                }
                _ => {}
            }
            self.curr += 1;
        }
    }
    fn peek_u8(&self) -> u8{
        let index = self.curr + 1;
        if index < self.src.len() {
            *self.src.get(index).unwrap()
        } else {
            0
        }
    }
    fn is_valid_letter(byte : u8) -> bool{
        (byte >= b'a' && byte <= b'z')
            || (byte >= b'0' && byte <= b'9')
            || (byte >= b'A' && byte <= b'Z')
            || byte == b'_'
    }
    fn is_valid_header(byte : u8) -> bool{
        (byte >= b'a' && byte <= b'z')
            || (byte >= b'A' && byte <= b'Z')
            || byte == b'_'
    }

    pub(crate) fn new(src : Vec<u8>) -> Tokenizer {
        Tokenizer{
            src,
            curr : 0,
            line : 1
        }
    }


}