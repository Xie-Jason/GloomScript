use std::fs::File;
use std::io::Read;

use hashbrown::HashSet;

use crate::frontend::parse::Parser;
use crate::frontend::script::ParsedFile;
use crate::frontend::tokenize::Tokenizer;
use crate::obj::refcount::RefCount;

pub struct Importer {
    set: HashSet<String>,
}

impl Importer {
    pub fn contains(&self, name: &str) -> bool {
        self.set.contains(name)
    }
    pub fn import_file(&mut self, name: &str, importer: RefCount<Importer>) -> ParsedFile {
        self.set.insert(String::from(name));
        let path = name;
        let mut file = File::open(path).unwrap();
        let mut src: Vec<u8> = Vec::with_capacity(256);
        file.read_to_end(&mut src).unwrap();
        let mut tokenizer = Tokenizer::new(src);
        let (tokens, lines) = tokenizer.tokenize();
        let parser: Parser = Parser::new(tokens, lines, importer);
        parser.parse()
    }
    pub fn new() -> Importer {
        Importer {
            set: HashSet::new(),
        }
    }
}
