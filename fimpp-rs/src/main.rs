mod ast;
mod lexer;
mod token;
mod variable;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: cargo run -- <ruta_al_archivo.fim>");
        return;
    }

    let source = fs::read_to_string(&args[1]).expect("No se pudo leer el archivo .fim");
    let tokens = lexer::parse(&source);

    match ast::create_report(&tokens, &source) {
        Ok(report) => {
            println!("=== Reporte Parseado Exitosamente ===");
            println!("Título: {:?}", report.title);
            println!("Autor:  {:?}", report.author);
            println!("Nodos en el cuerpo: {}", report.body.len());
            println!("{:#?}", report);
        }
        Err(err) => {
            eprintln!("Error de sintaxis: {}", err);
        }
    }
}