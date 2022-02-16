use crate::bytecode::gen::CodeGenerator;
use crate::frontend::analysis::Analyzer;
use crate::frontend::import::Importer;
use crate::frontend::status::GloomStatus;
use crate::obj::refcount::RefCount;
use crate::vm::machine::GloomVM;
use crate::vm::static_table::StaticTable;
use clap::{App, Arg};

mod builtin;
mod bytecode;
mod frontend;
mod jit;
mod obj;
mod stdlib;
mod vm;

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
                )
                .arg(
                    Arg::new("debug")
                        .short('d')
                        .long("debug")
                        .help("Enable debug mode"),
                ),
        )
        .subcommand(
            App::new("check")
                .about("Check a script")
                .arg(
                    Arg::new("FILE")
                        .help("Check the script")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("debug")
                        .short('d')
                        .long("debug")
                        .help("Enable debug mode"),
                ),
        );

    let matches = app.clone().get_matches();
    let mut status = true;

    // gloom run <FILE>
    if status {
        matches.subcommand_matches("run").map(|m| {
            status = false;
            let debug = m.is_present("debug");
            let path = m.value_of("FILE").unwrap();
            run_script(path.to_string(), debug)
        });
    }

    // gloom check <FILE>
    if status {
        matches.subcommand_matches("check").map(|m| {
            status = false;
            let debug = m.is_present("debug");
            let path = m.value_of("FILE").unwrap();
            parse_file(path.to_string(), debug)
        });
    }

    if status {
        app.print_help().map_err(|e| e.to_string())?
    }

    Ok(())
}

fn run_script(path: String, debug: bool) {
    // check file
    let (mut status, static_table) = parse_file(path, debug);

    // code generation
    let constant_pool = CodeGenerator::new().generate(&mut status);

    // run
    GloomVM::new(static_table, constant_pool, status).run();
}

fn parse_file(path: String, debug: bool) -> (GloomStatus, StaticTable) {
    // lexer and parse
    let importer = RefCount::new(Importer::new());
    let parsed_file = Importer::import_file(path, importer).unwrap().unwrap();
    // analyse
    let mut analyzer = Analyzer::new();
    analyzer.analysis(parsed_file, debug).map_err(|err| {
        err.to_string()
    }).unwrap();
    analyzer.result()
}
