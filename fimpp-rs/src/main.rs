mod ast;
mod errors;
mod interpreter;
mod lexer;
mod token;
mod variable;

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("fim (Rust port) {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let mut print_ast = false;
    let mut print_tokens = false;
    let mut file: Option<&String> = None;

    for arg in &args[1..] {
        match arg.as_str() {
            "--ast" => print_ast = true,
            "--tokens" => print_tokens = true,
            "--pretty" => {}
            other if !other.starts_with('-') => file = Some(arg),
            _ => {}
        }
    }

    let Some(path) = file else {
        eprintln!("Uso: cargo run -- <ruta_al_archivo.fim> [--ast] [--tokens] [--pretty] [--version]");
        return ExitCode::FAILURE;
    };

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("No se pudo leer el archivo '{}': {}", path, err);
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
            eprintln!("Spike notó algo inusual en tu reporte...");
            eprintln!("{}", err);
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
            eprintln!("La Princesa Celestia notó algo inusual en tu reporte...");
            eprintln!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    if args.iter().any(|a| a == "--pretty") {
        println!("┌─ fim {}", env!("CARGO_PKG_VERSION"));
        println!("├─ Report Name: {}", interpreter.report_title());
        println!("└─ Report Author: {}", interpreter.report_author());
    }

    if let Err(err) = interpreter.run() {
        eprintln!("¡La Princesa Celestia atrapó algo inusual en tu reporte!");
        eprintln!("{}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Port de `luna/aprint` restringido a la tabla de tokens de `main.go`.
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

    // Delimitadores por columna y alineación: 0 y 2 a la derecha (menos el
    // ancho de la columna); el resto a la izquierda.
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
