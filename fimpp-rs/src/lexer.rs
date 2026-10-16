use std::collections::VecDeque;
use crate::token::{Token, TokenType};

const SPLITTABLE_CHARS: [char; 13] = [
    '.', '!', '?', ':', ',', '(', ')', '"', '\'', ' ', '\t', '\\', '\n',
];

const PUNCTUATIONS: [char; 5] = ['.', '!', '?', ':', ','];

pub fn parse(source: &str) -> Vec<Token> {
    let t = create_partial_tokens(source);
    let t = merge_partial_tokens(t);
    let t = create_tokens(t);
    let t = merge_multi_tokens(t);
    let t = smart_identifier_tokens(t);
    let t = merge_identifiers(t);
    let t = clean_tokens(t);

    t.into_iter().collect()
}

// ============================================================================
// UTILIDADES LÉXICAS (luna/utilities + twilight/utilities)
// ============================================================================

pub fn as_boolean_value(s: &str) -> Option<bool> {
    match s {
        "yes" | "true" | "right" | "correct" => Some(true),
        "no" | "false" | "wrong" | "incorrect" => Some(false),
        _ => None,
    }
}

pub fn as_character_value(s: &str) -> Option<String> {
    if s.len() == 1 {
        return Some(s.to_string());
    }
    if s.len() == 2 && s.starts_with('\\') {
        let b = s.as_bytes()[1];
        let res = match b {
            b'0' => "\0".to_string(),
            b'r' => "\r".to_string(),
            b'n' => "\n".to_string(),
            b't' => "\t".to_string(),
            other => (other as char).to_string(),
        };
        return Some(res);
    }
    None
}

fn is_string_number(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let is_negative = value.starts_with('-');
    let mut has_number = false;

    for (idx, r) in value.chars().enumerate() {
        if is_negative && idx == 0 {
            continue;
        }
        if !r.is_ascii_digit() {
            return false;
        }
        has_number = true;
    }
    has_number
}

fn is_string_positive_number(value: &str) -> bool {
    !value.starts_with('-') && is_string_number(value)
}

fn is_indent_string(s: &str) -> bool {
    s == " " || s == "\t"
}

fn merge_tokens(q: &mut VecDeque<Token>, mut amount: usize) -> Option<Token> {
    let mut first = q.pop_front()?;
    amount = amount.saturating_sub(1);

    while amount > 0 {
        if let Some(t) = q.pop_front() {
            first.append(&t);
            amount -= 1;
        } else {
            break;
        }
    }
    Some(first)
}

fn check_token_sequence(tokens: &VecDeque<Token>, sequence: &[&str]) -> bool {
    if tokens.len() < sequence.len() {
        return false;
    }
    for (idx, &val) in sequence.iter().enumerate() {
        if tokens[idx].value != val {
            return false;
        }
    }
    true
}

// ============================================================================
// PARTIAL LEXER (twilight/partial_lexer.go)
// ============================================================================

fn create_partial_tokens(source: &str) -> VecDeque<Token> {
    let mut l = VecDeque::new();
    let mut start = 0;

    for (idx, r) in source.char_indices() {
        if SPLITTABLE_CHARS.contains(&r) {
            let length = idx - start;
            if length > 0 {
                l.push_back(Token::new(
                    start,
                    length,
                    TokenType::Unknown,
                    &source[start..start + length],
                ));
            }

            let char_len = r.len_utf8();
            l.push_back(Token::new(
                idx,
                char_len,
                TokenType::Unknown,
                &source[idx..idx + char_len],
            ));

            start = idx + char_len;
        }
    }

    if start < source.len() {
        let length = source.len() - start;
        l.push_back(Token::new(
            start,
            length,
            TokenType::Unknown,
            &source[start..start + length],
        ));
    }

    l
}

fn merge_partial_tokens(mut tokens: VecDeque<Token>) -> VecDeque<Token> {
    let mut l = VecDeque::new();
    let processors: [fn(&VecDeque<Token>) -> usize; 4] = [
        check_decimal_tokens,
        check_string_tokens,
        check_character_tokens,
        check_delimiters,
    ];

    let mut newline = false;
    while !tokens.is_empty() {
        if newline && is_indent_string(&tokens[0].value) {
            tokens.pop_front();
            continue;
        }
        newline = false;

        for processor in &processors {
            let merge_amount = processor(&tokens);
            if merge_amount > 0 {
                if let Some(merged) = merge_tokens(&mut tokens, merge_amount) {
                    tokens.push_front(merged);
                }
            }
        }

        if tokens[0].length == 1 && tokens[0].value == "\n" {
            newline = true;
        }

        if let Some(tok) = tokens.pop_front() {
            l.push_back(tok);
        }
    }

    l
}

fn check_decimal_tokens(tokens: &VecDeque<Token>) -> usize {
    if tokens.len() < 3 {
        return 0;
    }
    if !is_string_number(&tokens[0].value) {
        return 0;
    }
    if tokens[1].value != "." {
        return 0;
    }
    if !is_string_positive_number(&tokens[2].value) {
        return 0;
    }
    3
}

fn check_string_tokens(tokens: &VecDeque<Token>) -> usize {
    if tokens.is_empty() || tokens[0].value != "\"" {
        return 0;
    }

    let mut ignore_next = false;
    for idx in 1..tokens.len() {
        let val = &tokens[idx].value;
        if val == "\n" {
            return 0;
        }
        if ignore_next {
            ignore_next = false;
            continue;
        }
        if val == "\\" {
            ignore_next = true;
            continue;
        }
        if val == "\"" {
            return idx + 1;
        }
    }
    0
}

fn check_character_tokens(tokens: &VecDeque<Token>) -> usize {
    if tokens.is_empty() || tokens[0].value != "'" {
        return 0;
    }

    let mut merge_amount = 0;

    if tokens.len() >= 4
        && tokens[1].value == "\\"
        && tokens[2].value.len() == 1
        && tokens[3].value == "'"
    {
        merge_amount = 4;
    }

    if tokens.len() >= 3 && tokens[1].value.len() == 1 && tokens[2].value == "'" {
        merge_amount = 3;
    }

    merge_amount
}

fn check_delimiters(tokens: &VecDeque<Token>) -> usize {
    if tokens.is_empty() || !tokens[0].value.contains('(') {
        return 0;
    }

    for idx in 1..tokens.len() {
        let val = &tokens[idx].value;
        if val == "\n" {
            return 0;
        }
        if val.contains(')') {
            return idx + 1;
        }
    }
    0
}

// ============================================================================
// FULL LEXER (twilight/lexer.go)
// ============================================================================

fn create_tokens(mut partial_tokens: VecDeque<Token>) -> VecDeque<Token> {
    let mut tokens = VecDeque::new();

    while let Some(mut t) = partial_tokens.pop_front() {
        t.token_type = TokenType::Identifier;

        if t.length == 1 && t.value.chars().next().is_some_and(|c| PUNCTUATIONS.contains(&c)) {
            t.token_type = TokenType::Punctuation;
        } else if t.length == 1 && t.value == "\n" {
            t.token_type = TokenType::NewLine;
        } else if t.length == 1 && t.value == " " {
            t.token_type = TokenType::Whitespace;
        } else if t.length >= 1 && t.value.starts_with('(') && t.value.ends_with(')') {
            t.token_type = TokenType::CommentParen;
        } else if t.length >= 1 && t.value.starts_with('"') && t.value.ends_with('"') {
            t.token_type = TokenType::String;
        } else if t.length >= 1 && t.value.starts_with('\'') && t.value.ends_with('\'') {
            t.token_type = TokenType::Character;
        } else if t.length >= 1 && t.value.parse::<f64>().is_ok() {
            t.token_type = TokenType::Number;
        } else if t.length >= 1 && as_boolean_value(&t.value).is_some() {
            t.token_type = TokenType::Boolean;
        } else if t.value == "nothing" {
            t.token_type = TokenType::Null;
        }

        if t.length == 0 && t.token_type == TokenType::Identifier {
            continue;
        }

        tokens.push_back(t);
    }

    let start_index = tokens.back().map_or(0, |last| last.start);
    tokens.push_back(Token::new(start_index, 0, TokenType::EndOfFile, ""));

    tokens
}

fn merge_multi_tokens(mut old_tokens: VecDeque<Token>) -> VecDeque<Token> {
    let mut tokens = VecDeque::new();

    let processors: &[(fn(&VecDeque<Token>) -> usize, TokenType)] = &[
        (check_report_header, TokenType::ReportHeader),
        (check_report_footer, TokenType::ReportFooter),
        (check_function_header_main, TokenType::FunctionMain),
        (check_function_header, TokenType::FunctionHeader),
        (check_function_footer, TokenType::FunctionFooter),
        (check_function_parameter, TokenType::FunctionParameter),
        (check_function_return, TokenType::FunctionReturn),
        (check_print_method, TokenType::Print),
        (check_print_newline_method, TokenType::PrintNewline),
        (check_read_method, TokenType::Prompt),
        (check_function_call_method, TokenType::FunctionCall),
        (check_variable_declaration, TokenType::Declaration),
        (check_variable_modifier, TokenType::Modify),
        (check_boolean_type, TokenType::TypeBoolean),
        (check_boolean_array_type, TokenType::TypeBooleanArray),
        (check_number_type, TokenType::TypeNumber),
        (check_number_array_type, TokenType::TypeNumberArray),
        (check_string_type, TokenType::TypeString),
        (check_string_array_type, TokenType::TypeStringArray),
        (check_character_type, TokenType::TypeChar),
        (check_postscript, TokenType::CommentPostScript),
        (check_if_keyword, TokenType::IfClause),
        (check_else_keyword, TokenType::ElseClause),
        (check_if_end_keyword, TokenType::IfEndClause),
        (check_while_keyword, TokenType::WhileClause),
        (check_for_every_keyword, TokenType::ForEveryClause),
        (check_statement_end_keyword, TokenType::KeywordStatementEnd),
        (check_infix_addition, TokenType::OperatorAddInfix),
        (check_prefix_addition, TokenType::OperatorAddPrefix),
        (check_infix_subtraction, TokenType::OperatorSubInfix),
        (check_prefix_subtraction, TokenType::OperatorSubPrefix),
        (check_infix_multiplication, TokenType::OperatorMulInfix),
        (check_prefix_multiplication, TokenType::OperatorMulPrefix),
        (check_infix_division, TokenType::OperatorDivInfix),
        (check_prefix_division, TokenType::OperatorDivPrefix),
        (check_prefix_modulo, TokenType::OperatorModPrefix),
        (check_infix_modulo, TokenType::OperatorModInfix),
        (check_less_than_equal_operator, TokenType::OperatorLte),
        (check_greater_than_equal_operator, TokenType::OperatorGte),
        (check_greater_than_operator, TokenType::OperatorGt),
        (check_less_than_operator, TokenType::OperatorLt),
        (check_not_equal_operator, TokenType::OperatorNeq),
        (check_equal_operator, TokenType::OperatorEq),
        (check_unary_prefix_increment, TokenType::UnaryIncrementPrefix),
        (check_unary_postfix_increment, TokenType::UnaryIncrementPostfix),
        (check_unary_prefix_decrement, TokenType::UnaryDecrementPrefix),
        (check_unary_postfix_decrement, TokenType::UnaryDecrementPostfix),
        (check_return_keyword, TokenType::KeywordReturn),
        (check_constant_keyword, TokenType::KeywordConst),
        (check_then_keyword, TokenType::KeywordThen),
        (check_and_keyword, TokenType::KeywordAnd),
        (check_or_keyword, TokenType::KeywordOr),
        (check_of_keyword, TokenType::KeywordOf),
        (check_in_keyword, TokenType::KeywordIn),
        (check_from_keyword, TokenType::KeywordFrom),
        (check_to_keyword, TokenType::KeywordTo),
    ];

    while !old_tokens.is_empty() {
        for &(condition, result_type) in processors {
            if old_tokens.is_empty() {
                break;
            }
            if old_tokens[0].token_type != TokenType::Identifier {
                continue;
            }
            let amount = condition(&old_tokens);
            if amount == 0 {
                continue;
            }
            if let Some(mut merged) = merge_tokens(&mut old_tokens, amount) {
                merged.token_type = result_type;
                old_tokens.push_front(merged);
            }
        }

        if let Some(tok) = old_tokens.pop_front() {
            tokens.push_back(tok);
        }
    }

    tokens
}

fn smart_identifier_tokens(mut old_tokens: VecDeque<Token>) -> VecDeque<Token> {
    let mut tokens = VecDeque::new();
    let mut is_for_every = false;

    while let Some(mut t) = old_tokens.pop_front() {
        if t.token_type == TokenType::ForEveryClause {
            is_for_every = true;
        }

        if !is_for_every {
            if matches!(
                t.token_type,
                TokenType::KeywordIn | TokenType::KeywordFrom | TokenType::KeywordTo
            ) {
                t.token_type = TokenType::Identifier;
            }
        }

        if matches!(t.token_type, TokenType::Punctuation | TokenType::NewLine) {
            is_for_every = false;
        }

        tokens.push_back(t);
    }

    tokens
}

fn merge_identifiers(mut old_tokens: VecDeque<Token>) -> VecDeque<Token> {
    let mut tokens = VecDeque::new();
    let mut identifier_token: Option<Token> = None;

    while let Some(t) = old_tokens.pop_front() {
        if t.token_type != TokenType::Identifier {
            if let Some(ref mut id_tok) = identifier_token {
                if t.token_type == TokenType::Whitespace
                    && !old_tokens.is_empty()
                    && old_tokens[0].token_type == TokenType::Identifier
                {
                    id_tok.append(&t);
                    continue;
                }
            }

            if let Some(id_tok) = identifier_token.take() {
                tokens.push_back(id_tok);
            }
            tokens.push_back(t);
            continue;
        }

        if let Some(ref mut id_tok) = identifier_token {
            id_tok.append(&t);
        } else {
            identifier_token = Some(t);
        }
    }

    if let Some(id_tok) = identifier_token.take() {
        tokens.push_back(id_tok);
    }

    tokens
}

fn clean_tokens(mut old_tokens: VecDeque<Token>) -> VecDeque<Token> {
    let mut tokens = VecDeque::new();

    while let Some(t) = old_tokens.pop_front() {
        match t.token_type {
            TokenType::NewLine | TokenType::Whitespace | TokenType::CommentParen => continue,
            TokenType::CommentPostScript => {
                while !old_tokens.is_empty()
                    && old_tokens[0].token_type != TokenType::NewLine
                    && old_tokens[0].token_type != TokenType::EndOfFile
                {
                    old_tokens.pop_front();
                }
                continue;
            }
            _ => tokens.push_back(t),
        }
    }

    tokens
}

// ============================================================================
// REGLAS DE FRASES MULTI-TOKEN (twilight/parsers/*.go)
// ============================================================================

fn check_any_sequence(tokens: &VecDeque<Token>, sequences: &[&[&str]]) -> usize {
    for &seq in sequences {
        if check_token_sequence(tokens, seq) {
            return seq.len();
        }
    }
    0
}

fn check_single_or_multi(tokens: &VecDeque<Token>, singles: &[&str], multis: &[&[&str]]) -> usize {
    if !tokens.is_empty() && singles.contains(&tokens[0].value.as_str()) {
        return 1;
    }
    check_any_sequence(tokens, multis)
}

// Report
fn check_report_header(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["Dear", " ", "Princess", " ", "Celestia", ":"]])
}
fn check_report_footer(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["Your", " ", "faithful", " ", "student", ","]])
}

// Functions
fn check_function_header_main(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["Today", " ", "I", " ", "learned"]])
}
fn check_function_header(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["I", " ", "learned"]])
}
fn check_function_footer(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["That", "'", "s", " ", "all", " ", "about"]])
}
fn check_function_parameter(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["using"], &[])
}
fn check_function_return(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["with"], &[&["to", " ", "get"]])
}
fn check_return_keyword(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["Then", " ", "you", " ", "get"]])
}
fn check_function_call_method(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["I", " ", "remembered"], &["I", " ", "would"]])
}

// Variables
fn check_variable_declaration(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["Did", " ", "you", " ", "know", " ", "that"]])
}
fn check_variable_modifier(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["becomes", "become", "became"], &[&["is", " ", "now"]])
}

// Arithmetic
fn check_infix_addition(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["plus"], &[&["added", " ", "to"]])
}
fn check_prefix_addition(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["add"], &[])
}
fn check_infix_subtraction(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["minus", "without"], &[])
}
fn check_prefix_subtraction(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["subtract"], &[&["the", " ", "difference", " ", "between"]])
}
fn check_infix_multiplication(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["times"], &[&["multiplied", " ", "with"]])
}
fn check_prefix_multiplication(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["multiply"], &[])
}
fn check_infix_division(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["divided", " ", "by"]])
}
fn check_prefix_division(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["divide"], &[])
}
fn check_infix_modulo(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["mod", "modulo", "remainder"], &[])
}
fn check_prefix_modulo(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["remainder", " ", "of"]])
}

// Relational
fn check_less_than_equal_operator(t: &VecDeque<Token>) -> usize {
    check_any_sequence(
        t,
        &[
            &["had", " ", "no", " ", "more", " ", "than"],
            &["has", " ", "no", " ", "more", " ", "than"],
            &["is", " ", "no", " ", "greater", " ", "than"],
            &["is", " ", "no", " ", "more", " ", "than"],
            &["is", " ", "not", " ", "greater", " ", "than"],
            &["is", " ", "not", " ", "more", " ", "than"],
            &["isn", "'", "t", " ", "greater", " ", "than"],
            &["isn", "'", "t", " ", "more", " ", "than"],
            &["was", " ", "no", " ", "greater", " ", "than"],
            &["was", " ", "no", " ", "more", " ", "than"],
            &["was", " ", "not", " ", "greater", " ", "than"],
            &["was", " ", "not", " ", "more", " ", "than"],
            &["wasn", "'", "t", " ", "greater", " ", "than"],
            &["wasn", "'", "t", " ", "more", " ", "than"],
            &["were", " ", "no", " ", "greater", " ", "than"],
            &["were", " ", "no", " ", "more", " ", "than"],
            &["were", " ", "not", " ", "greater", " ", "than"],
            &["were", " ", "not", " ", "more", " ", "than"],
            &["weren", "'", "t", " ", "greater", " ", "than"],
            &["weren", "'", "t", " ", "more", " ", "than"],
        ],
    )
}

fn check_greater_than_equal_operator(t: &VecDeque<Token>) -> usize {
    check_any_sequence(
        t,
        &[
            &["had", " ", "no", " ", "less", " ", "than"],
            &["has", " ", "no", " ", "less", " ", "than"],
            &["is", " ", "no", " ", "less", " ", "than"],
            &["is", " ", "not", " ", "less", " ", "than"],
            &["isn", "'", "t", " ", "less", " ", "than"],
            &["was", " ", "no", " ", "less", " ", "than"],
            &["was", " ", "not", " ", "less", " ", "than"],
            &["wasn", "'", "t", " ", "less", " ", "than"],
            &["were", " ", "no", " ", "less", " ", "than"],
            &["were", " ", "not", " ", "less", " ", "than"],
            &["weren", "'", "t", " ", "less", " ", "than"],
        ],
    )
}

fn check_greater_than_operator(t: &VecDeque<Token>) -> usize {
    check_any_sequence(
        t,
        &[
            &["had", " ", "more", " ", "than"],
            &["has", " ", "more", " ", "than"],
            &["were", " ", "more", " ", "than"],
            &["was", " ", "more", " ", "than"],
            &["is", " ", "greater", " ", "than"],
            &["was", " ", "greater", " ", "than"],
            &["were", " ", "greater", " ", "than"],
        ],
    )
}

fn check_less_than_operator(t: &VecDeque<Token>) -> usize {
    check_any_sequence(
        t,
        &[
            &["had", " ", "less", " ", "than"],
            &["has", " ", "less", " ", "than"],
            &["is", " ", "less", " ", "than"],
            &["was", " ", "less", " ", "than"],
            &["were", " ", "less", " ", "than"],
        ],
    )
}

fn check_not_equal_operator(t: &VecDeque<Token>) -> usize {
    check_any_sequence(
        t,
        &[
            &["wasn", "'", "t", " ", "equal", " ", "to"],
            &["isn", "'", "t", " ", "equal", " ", "to"],
            &["weren", "'", "t", " ", "equal", " ", "to"],
            &["had", "'", "t"],
            &["has", "'", "t"],
            &["isn", "'", "t"],
            &["wasn", "'", "t"],
            &["weren", "'", "t"],
            &["had", " ", "not"],
            &["has", " ", "not"],
            &["is", " ", "not"],
            &["was", " ", "not"],
            &["were", " ", "not"],
        ],
    )
}

fn check_equal_operator(t: &VecDeque<Token>) -> usize {
    let multi = check_any_sequence(
        t,
        &[
            &["is", " ", "equal", " ", "to"],
            &["was", " ", "equal", " ", "to"],
            &["were", " ", "equal", " ", "to"],
        ],
    );
    if multi > 0 {
        return multi;
    }
    check_single_or_multi(t, &["is", "was", "were", "had", "has", "likes", "like"], &[])
}

// Unary
fn check_unary_prefix_increment(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["There", " ", "was", " ", "one", " ", "more"]])
}
fn check_unary_postfix_increment(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["got", " ", "one", " ", "more"]])
}
fn check_unary_prefix_decrement(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["There", " ", "was", " ", "one", " ", "less"]])
}
fn check_unary_postfix_decrement(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["got", " ", "one", " ", "less"]])
}

// IO
fn check_print_newline_method(t: &VecDeque<Token>) -> usize {
    if t.len() >= 3
        && t[0].value == "I"
        && t[1].value == " "
        && ["said", "sang", "wrote"].contains(&t[2].value.as_str())
    {
        return 3;
    }
    0
}

fn check_print_method(t: &VecDeque<Token>) -> usize {
    if t.len() >= 5
        && t[0].value == "I"
        && t[1].value == " "
        && t[2].value == "quickly"
        && t[3].value == " "
        && ["said", "sang", "wrote"].contains(&t[4].value.as_str())
    {
        return 5;
    }
    0
}

fn check_read_method(t: &VecDeque<Token>) -> usize {
    if t.len() >= 3
        && t[0].value == "I"
        && t[1].value == " "
        && ["heard", "read", "asked"].contains(&t[2].value.as_str())
    {
        return 3;
    }
    0
}

// Keywords
fn check_constant_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["always"], &[])
}
fn check_and_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["and"], &[])
}
fn check_or_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["or"], &[])
}
fn check_of_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["of"], &[])
}
fn check_in_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["in"], &[])
}
fn check_from_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["from"], &[])
}
fn check_to_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["to"], &[])
}
fn check_statement_end_keyword(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["That", "'", "s", " ", "what", " ", "I", " ", "did"]])
}

// Postscript (P.S. / P.S.S. ...)
fn check_postscript(tokens: &VecDeque<Token>) -> usize {
    if tokens.len() < 5 || tokens[0].value != "P" || tokens[1].value != "." {
        return 0;
    }
    let mut idx = 2;
    while idx < tokens.len() {
        if idx + 1 >= tokens.len() {
            return 0;
        }
        if tokens[idx].value != "S" || tokens[idx + 1].value != "." {
            return 0;
        }
        if idx + 2 < tokens.len() && tokens[idx + 2].value.trim_end_matches(' ').is_empty() {
            return idx + 1;
        }
        idx += 2;
    }
    0
}

// Types
fn check_number_type(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(
        t,
        &["number"],
        &[&["a", " ", "number"], &["the", " ", "number"]],
    )
}
fn check_number_array_type(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(
        t,
        &["numbers"],
        &[&["the", " ", "numbers"], &["many", " ", "numbers"]],
    )
}
fn check_boolean_type(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(
        t,
        &["argument", "logic"],
        &[
            &["an", " ", "argument"],
            &["the", " ", "argument"],
            &["the", " ", "logic"],
        ],
    )
}
fn check_boolean_array_type(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(
        t,
        &["arguments", "logics"],
        &[
            &["many", " ", "arguments"],
            &["many", " ", "logics"],
            &["the", " ", "arguments"],
            &["the", " ", "logics"],
        ],
    )
}
fn check_character_type(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(
        t,
        &["character", "letter"],
        &[
            &["a", " ", "character"],
            &["a", " ", "letter"],
            &["the", " ", "character"],
            &["the", " ", "letter"],
        ],
    )
}
fn check_string_type(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(
        t,
        &["characters", "letters", "phrase", "quote", "sentence", "word"],
        &[
            &["a", " ", "phrase"],
            &["a", " ", "quote"],
            &["a", " ", "sentence"],
            &["a", " ", "word"],
            &["the", " ", "characters"],
            &["the", " ", "letters"],
            &["the", " ", "phrase"],
            &["the", " ", "quote"],
            &["the", " ", "sentence"],
            &["the", " ", "word"],
        ],
    )
}
fn check_string_array_type(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(
        t,
        &["phrases", "quotes", "sentences", "words"],
        &[
            &["many", " ", "phrases"],
            &["many", " ", "quotes"],
            &["many", " ", "sentences"],
            &["many", " ", "words"],
            &["the", " ", "phrases"],
            &["the", " ", "quotes"],
            &["the", " ", "sentences"],
            &["the", " ", "words"],
        ],
    )
}

// Control Flow
fn check_for_every_keyword(t: &VecDeque<Token>) -> usize {
    check_any_sequence(t, &[&["For", " ", "every"]])
}
fn check_if_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["If", "When"], &[])
}
fn check_then_keyword(t: &VecDeque<Token>) -> usize {
    check_single_or_multi(t, &["then"], &[])
}
fn check_else_keyword(t: &VecDeque<Token>) -> usize {
    let multi = check_any_sequence(
        t,
        &[
            &["Otherwise", " ", "if"],
            &["Or", " ", "else", " ", "if"],
            &["Or", " ", "else"],
        ],
    );
    if multi > 0 {
        return multi;
    }
    check_single_or_multi(t, &["Otherwise"], &[])
}
fn check_if_end_keyword(t: &VecDeque<Token>) -> usize {
    check_any_sequence(
        t,
        &[&["That", "'", "s", " ", "what", " ", "I", " ", "would", " ", "do"]],
    )
}
fn check_while_keyword(t: &VecDeque<Token>) -> usize {
    check_any_sequence(
        t,
        &[
            &["As", " ", "long", " ", "as"],
            &["Here", "'", "s", " ", "what", " ", "I", " ", "did", " ", "while"],
        ],
    )
}