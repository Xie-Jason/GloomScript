use std::fs::File;
use std::io::Read;
use std::mem::{MaybeUninit, size_of};
use std::time::Instant;
use crate::exec::executor::Executor;
use crate::frontend::analysis::Analyzer;
use crate::frontend::import::Importer;
use crate::frontend::parse::Parser;
use crate::frontend::tokenize::Tokenizer;
use crate::obj::refcount::RefCount;
use crate::obj::slot::Slot;

mod obj;
mod frontend;
mod builtin;
mod exec;

fn main() {
    println!("MaybeUninit<Slot>{} Slot{}",size_of::<MaybeUninit<Slot>>(),size_of::<Slot>());
    let debug = false;
    let mut file = File::open(&String::from(r"D:\Rust\projects\gloomscript\gloom\GloomTest.gs")).unwrap();
    let mut src : Vec<u8> = Vec::with_capacity(256);
    let read = file.read_to_end(&mut src).unwrap();
    if debug {
        println!("read bytes:{}",read);
    }
    let mut tokenizer = Tokenizer::new(src);
    let (tokens,lines) = tokenizer.tokenize();
    let importer = RefCount::new(Importer::new());
    let parse_start_time = Instant::now();
    let parser : Parser = Parser::new(tokens,lines,importer);
    let parsed_file = parser.parse();
    let parse_end_time = Instant::now();
    println!("Parse {}us",parse_end_time.duration_since(parse_start_time).as_micros());
    let mut analyzer = Analyzer::new();
    analyzer.analysis(parsed_file,debug);
    let analysis_end_time = Instant::now();
    println!("Analysis {}us",analysis_end_time.duration_since(parse_end_time).as_micros());
    let (status,static_table) = analyzer.result();
    let executor = Executor::new(status,static_table);
    Executor::exec(executor);
    println!("Execute {}us",Instant::now().duration_since(analysis_end_time).as_micros());
}
