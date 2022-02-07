use std::fs::File;
use std::io::Read;
use std::time::Instant;
use crate::bytecode::gen::CodeGenerator;
use crate::exec::executor::Executor;
use crate::frontend::analysis::Analyzer;
use crate::frontend::import::Importer;
use crate::frontend::parse::Parser;
use crate::frontend::tokenize::Tokenizer;
use crate::obj::refcount::RefCount;
use crate::vm::machine::GloomVM;

mod obj;
mod frontend;
mod builtin;
mod exec;
mod bytecode;
mod vm;

fn main() {
    let debug = false;
    let mut file = File::open(&String::from(r"D:\Rust\projects\gloomscript\gloom\AfterTest.gs")).unwrap();
    let mut src : Vec<u8> = Vec::with_capacity(256);
    file.read_to_end(&mut src).unwrap();
    // lexer and parse
    let mut tokenizer = Tokenizer::new(src);
    let (tokens,lines) = tokenizer.tokenize();
    let parser : Parser = Parser::new(tokens,lines,RefCount::new(Importer::new()));
    let parsed_file = parser.parse();
    // analyse
    let mut analyzer = Analyzer::new();
    analyzer.analysis(parsed_file,debug);
    let (mut status,static_table) = analyzer.result();
    // code generation
    let constant_pool = CodeGenerator::new().generate(&mut status);
    // run
    GloomVM::new(static_table,constant_pool,status).run();
}
