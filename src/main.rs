use std::fs::File;
use std::io::Read;

use clap::{App, Arg};

use crate::bytecode::gen::CodeGenerator;
use crate::frontend::analysis::Analyzer;
use crate::frontend::import::Importer;
use crate::frontend::parse::Parser;
use crate::frontend::status::GloomStatus;
use crate::frontend::tokenize::Tokenizer;
use crate::obj::refcount::RefCount;
use crate::vm::machine::GloomVM;
use crate::vm::static_table::StaticTable;

mod obj;
mod frontend;
mod builtin;
mod bytecode;
mod vm;
mod jit;

fn main() -> Result<(), String> {
    let mut app = App::new("Gloom Script")
        .version(env!("CARGO_PKG_VERSION"))
        .about("GloomScript language interpreter implemented in Rust.")
        .author("Xie Jason")
        .subcommand(
            App::new("run")
                .about("Run a script")
                .arg(
                    Arg::new("FILE")
                        .help("Sets the script to run")
                        .required(true)
                        .index(1),
                ).arg(
                Arg::new("debug")
                    .short('d')
                    .long("debug")
                    .help("Enable debug mode"),
            )
        ).subcommand(
        App::new("check")
            .about("Check a script")
            .arg(
                Arg::new("FILE")
                    .help("Check the script")
                    .required(true)
                    .index(1)
            )
            .arg(
                Arg::new("debug")
                    .short('d')
                    .long("debug")
                    .help("Enable debug mode")
            )
    );

    let matches = app.clone().get_matches();
    let mut status = true;

    // gloom run <FILE>
    if status {
        matches.subcommand_matches("run").map(|m| {
            status = false;
            let debug = m.is_present("debug");
            let path = m.value_of("FILE").unwrap();
            run_file(read_file(path.to_string()), debug)
        });
    }

    // gloom check <FILE>
    if status {
        matches.subcommand_matches("check").map(|m| {
            status = false;
            let debug = m.is_present("debug");
            let path = m.value_of("FILE").unwrap();
            check_file(read_file(path.to_string()), debug)
        });
    }

    if status {
        app.print_help().map_err(|e| e.to_string())?
    }

    Ok(())
}

fn read_file(path: String) -> Vec<u8> {
    let mut file = File::open(&path).unwrap();
    let mut src: Vec<u8> = Vec::with_capacity(256);
    file.read_to_end(&mut src).unwrap();
    src
}

fn run_file(src: Vec<u8>, debug: bool) {
    // check file
    let (mut status, static_table) = check_file(src, debug);

    // code generation
    let constant_pool = CodeGenerator::new().generate(&mut status);

    // run
    GloomVM::new(static_table, constant_pool, status).run();
}

fn check_file(src: Vec<u8>, debug: bool) -> (GloomStatus, StaticTable) {
    // lexer and parse
    let mut tokenizer = Tokenizer::new(src);
    let (tokens, lines) = tokenizer.tokenize();
    let parser: Parser = Parser::new(tokens, lines, RefCount::new(Importer::new()));
    let parsed_file = parser.parse();

    // analyse
    let mut analyzer = Analyzer::new();
    analyzer.analysis(parsed_file, debug);
    analyzer.result()
}