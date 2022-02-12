use std::convert::TryFrom;
use std::fs::File;
use std::io::{Error, Read};

use hashbrown::HashSet;

use crate::frontend::parse::Parser;
use crate::frontend::script::ParsedFile;
use crate::frontend::tokenize::Tokenizer;
use crate::obj::refcount::RefCount;
use crate::stdlib::StdLibKind;

pub struct Importer {
    file_set : HashSet<String>,
    std_set : HashSet<StdLibKind>
}

impl Importer {
    pub fn import_file(name: String, importer: RefCount<Importer>) -> Result<Option<ParsedFile>, Error> {
        println!("import {}",name);
        {
            let mut importer_mut = importer.inner_mut();
            let contains = importer_mut.file_set.contains(name.as_str());
            if contains {
                return Result::Ok(Option::None)
            }
            importer_mut.file_set.insert(name.clone());
        }
        let path = name.as_str();
        let mut file = File::open(path)?;
        let mut src: Vec<u8> = Vec::with_capacity(256);
        file.read_to_end(&mut src).unwrap();
        let mut tokenizer = Tokenizer::new(src);
        let (tokens, lines) = tokenizer.tokenize();
        let parser: Parser = Parser::new(tokens, lines, importer.clone(),name);
        Result::Ok(Option::Some(parser.parse()))
    }
    pub fn import_std_lib(name : &str, importer : RefCount<Importer>) -> Result<(),String>{
        match StdLibKind::try_from(name) {
            Ok(kind) => {
                let mut importer = importer.inner_mut();
                let already_exists = importer.std_set.contains(&kind);
                if ! already_exists {
                    importer.std_set.insert(kind);
                }
                Result::Ok(())
            }
            Err(err) => Result::Err(err),
        }
    }
    pub fn new() -> Importer {
        Importer {
            file_set: HashSet::new(),
            std_set: HashSet::new(),
        }
    }
}
