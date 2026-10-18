use fimpp_ast::*;
use fimpp_lexer::*;
use fimpp_semantics::*;
use fimpp_vm::*;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    // Reproduce la semantica de `flag` de Go: deja de parsear al primer arg
    // posicional. Los flags pueden ir antes del archivo.
    let mut pretty = false;
    let mut print_tokens = false;
    let mut print_ast = false;
    let mut check = false;
    let mut version = false;
    let mut file: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-version" | "--version" => version = true,
            "-tokens" | "--tokens" => print_tokens = true,
            "-pretty" | "--pretty" => pretty = true,
            "-ast" | "--ast" => print_ast = true,
            "-check" | "--check" => check = true,
            s if s.starts_with('-') => {}
            _ => {
                file = Some(args[i].clone());
                break;
            }
        }
        i += 1;
    }

    if version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let Some(path) = file else {
        return ExitCode::SUCCESS;
    };

    // Go checks that the path is a regular file.
    match fs::metadata(&path) {
        Ok(m) if m.is_file() => {}
        _ => {
            println!("Invalid file '{}'", &path);
            return ExitCode::FAILURE;
        }
    }

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            println!("An error has occured while trying to load file '{}'", &path);
            println!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    let tokens = lexer::parse(&source);

    if print_tokens {
        print_token_table(&tokens);
        return ExitCode::SUCCESS;
    }

    let report = match ast::create_report(&tokens, &source) {
        Ok(report) => report,
        Err(err) => {
            println!("Spike noticed something unusual in your report...");
            println!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    if print_ast {
        println!("=== Reporte Parseado Exitosamente ===");
        println!("Título: {:?}", report.title);
        println!("Autor:  {:?}", report.author);
        println!("Nodos en el cuerpo: {}", report.body.len());
        println!("{:#?}", report);
        return ExitCode::SUCCESS;
    }

    let mut interpreter = match interpreter::Interpreter::new(&report, &source) {
        Ok(interpreter) => interpreter,
        Err(err) => {
            println!("Princess Celestia noticed something unusual in your report...");
            println!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    if check {
        match semantics::analyze(&report, &source) {
            Ok(()) => return ExitCode::SUCCESS,
            Err(err) => {
                println!("{}", err);
                return ExitCode::FAILURE;
            }
        }
    }

    if pretty {
        println!("┌─ fim {}", env!("CARGO_PKG_VERSION"));
        println!("├─ Report Name: {}", interpreter.report_title());
        println!("└─ Report Author: {}", interpreter.report_author());
    }

    if let Err(err) = interpreter.run() {
        println!("Princess Celestia caught something unusual in your report!");
        println!("{}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Port de la tabla de tokens de `main.go` (luna/aprint).
fn print_token_table(tokens: &[token::Token]) {
    const COLUMNS: usize = 4;

    let mut widths = [0usize; COLUMNS];
    let mut rows: Vec<[String; COLUMNS]> = Vec::with_capacity(tokens.len());

    for (idx, token) in tokens.iter().enumerate() {
        let row = [
            format!("{}.", idx + 1),
            format!("{}:{}", token.start + 1, token.start + 1 + token.length),
            token.value.clone(),
            token.token_type.to_string(),
        ];

        for column in 0..COLUMNS {
            if row[column].len() >= widths[column] {
                widths[column] = row[column].len();
            }
        }

        rows.push(row);
    }

    let delimiters = [" ", " ", " -> "];
    let right_aligned = [true, false, true, false];

    let mut output = String::new();
    for (idx, row) in rows.iter().enumerate() {
        for column in 0..COLUMNS {
            if right_aligned[column] {
                output.push_str(&format!("{:>width$}", row[column], width = widths[column]));
            } else {
                output.push_str(&format!("{:<width$}", row[column], width = widths[column]));
            }

            if column + 1 != COLUMNS {
                output.push_str(delimiters[column]);
            }
        }

        if idx + 1 != rows.len() {
            output.push('\n');
        }
    }

    println!("{}", output);
}
