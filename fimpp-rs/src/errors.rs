// Port de luna/errors/errors.go
// Formatea errores indicando la línea/columna del fallo, igual que el original.

pub struct ErrorOrigin {
    pub line: usize,
    pub column: usize,
    line_content: String,
}

pub fn get_error_origin(source: &str, index: usize) -> ErrorOrigin {
    let end = std::cmp::min(index + 1, source.len());
    // Evita cortar un carácter UTF-8 a la mitad (Go trabaja a nivel de bytes).
    let content = source.get(..end).unwrap_or(source);
    let lines: Vec<&str> = content.split('\n').collect();

    let line = lines.len() + 1;
    let column = lines[lines.len() - 1].len() + 1;

    let source_lines: Vec<&str> = source.split('\n').collect();
    let line_content = source_lines[lines.len() - 1].replace('\t', " ");

    ErrorOrigin {
        line,
        column,
        line_content,
    }
}

pub fn parse_error(message: &str, source: &str, index: usize) -> String {
    let origin = get_error_origin(source, index);

    let trimmed = origin.line_content.trim_start_matches(['\t', ' ']);
    let indent = origin.line_content.len() - trimmed.len();

    let pad = origin.column as isize - 1 - indent as isize;
    let pad = if pad > 0 { " ".repeat(pad as usize) } else { String::new() };

    format!(
        "[line {}:{}] {}\n{}\n{}^",
        origin.line, origin.column, message, trimmed, pad
    )
}
