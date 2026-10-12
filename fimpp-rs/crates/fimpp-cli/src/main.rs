use chumsky::Parser;
use fimpp_lexer::Token;
use logos::Logos;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: fimpp-cli <command> <file>");
        eprintln!("Commands:");
        eprintln!("  run <file.fimpp>      Compile and execute a FiM++ source file");
        eprintln!("  compile <file.fimpp>  Compile a FiM++ source file to bytecode");
        eprintln!("  exec <file.bin>       Execute a compiled bytecode file");
        std::process::exit(1);
    }

    let command = &args[1];
    let file_path = &args[2];

    match command.as_str() {
        "run" => {
            let source = fs::read_to_string(file_path).expect("Failed to read source file");
            let bytecode = compile_source(&source);
            let mut vm = fimpp_vm::VM::new();
            if let Err(e) = vm.execute(&bytecode) {
                eprintln!("Runtime error: {}", e);
                std::process::exit(1);
            }
        }
        "compile" => {
            let source = fs::read_to_string(file_path).expect("Failed to read source file");
            let bytecode = compile_source(&source);
            let bin = fimpp_codegen::serialize(&bytecode);
            let out_path = format!("{}.bin", file_path);
            fs::write(&out_path, bin).expect("Failed to write bytecode file");
            println!("Compiled to {}", out_path);
        }
        "exec" => {
            let bin = fs::read(file_path).expect("Failed to read bytecode file");
            let bytecode = fimpp_codegen::deserialize(&bin).expect("Failed to parse bytecode");
            let mut vm = fimpp_vm::VM::new();
            if let Err(e) = vm.execute(&bytecode) {
                eprintln!("Runtime error: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}

fn compile_source(source: &str) -> fimpp_codegen::Bytecode {
    let tokens: Result<Vec<Token>, _> = Token::lexer(source).collect();
    let tokens = match tokens {
        Ok(t) => t,
        Err(_) => {
            eprintln!("Lexer error");
            std::process::exit(1);
        }
    };

    let ast = match fimpp_parser::parser().parse(tokens) {
        Ok(program) => program,
        Err(errs) => {
            eprintln!("Parser error:");
            for e in errs {
                eprintln!("{:?}", e);
            }
            std::process::exit(1);
        }
    };

    match fimpp_codegen::generate(&ast) {
        Ok(bytecode) => bytecode,
        Err(e) => {
            eprintln!("Codegen error: {:?}", e);
            std::process::exit(1);
        }
    }
}
