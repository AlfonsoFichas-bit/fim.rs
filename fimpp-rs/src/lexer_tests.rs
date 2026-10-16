use std::collections::VecDeque;

use crate::token::TokenType;

use super::{create_partial_tokens, is_string_number, merge_partial_tokens, merge_tokens, parse};

fn check_tokens(tokens: &[crate::token::Token], checks: &[(TokenType, &str)]) {
    assert_eq!(checks.len(), tokens.len(), "Mismatch token count");
    for (idx, (tt, val)) in checks.iter().enumerate() {
        assert_eq!(tt, &tokens[idx].token_type, "token type mismatch at {}", idx);
        assert_eq!(val, &tokens[idx].value, "token value mismatch at {}", idx);
    }
}

#[test]
fn token_test_basic_report() {
    let source = "Dear Princess Celestia: Hello World!\nYour faithful student, Twilight Sparkle.\n";
    let tokens = parse(source);
    check_tokens(
        &tokens,
        &[
            (TokenType::ReportHeader, "Dear Princess Celestia:"),
            (TokenType::Identifier, "Hello World"),
            (TokenType::Punctuation, "!"),
            (TokenType::ReportFooter, "Your faithful student,"),
            (TokenType::Identifier, "Twilight Sparkle"),
            (TokenType::Punctuation, "."),
            (TokenType::EndOfFile, ""),
        ],
    );
}

#[test]
fn token_test_postscript_clean() {
    let source = "P.S. Hello!";
    let tokens = parse(source);
    check_tokens(&tokens, &[(TokenType::EndOfFile, "")]);
}

#[test]
fn token_test_postscript_clean_postpostscript() {
    let source = "P.S.S. Hello!";
    let tokens = parse(source);
    check_tokens(&tokens, &[(TokenType::EndOfFile, "")]);
}

#[test]
fn token_test_ignore_invalid_postscript() {
    let source = "P.S.Hello!";
    let tokens = parse(source);
    check_tokens(
        &tokens,
        &[
            (TokenType::Identifier, "P"),
            (TokenType::Punctuation, "."),
            (TokenType::Identifier, "S"),
            (TokenType::Punctuation, "."),
            (TokenType::Identifier, "Hello"),
            (TokenType::Punctuation, "!"),
            (TokenType::EndOfFile, ""),
        ],
    );
}

#[test]
fn token_test_clean_inside_postscript() {
    let source = "Hello\nP.S. Hello!\nWorld";
    let tokens = parse(source);
    check_tokens(
        &tokens,
        &[
            (TokenType::Identifier, "Hello"),
            (TokenType::Identifier, "World"),
            (TokenType::EndOfFile, ""),
        ],
    );
}

#[test]
fn utilities_is_string_number() {
    assert!(is_string_number("1"));
    assert!(is_string_number("10"));
    assert!(!is_string_number("a"));
    assert!(!is_string_number("Zz"));
}

#[test]
fn splittable_runes() {
    // SPLITTABLE_CHARS contains ' '
    assert!(super::SPLITTABLE_CHARS.contains(&' '));
    assert!(!super::SPLITTABLE_CHARS.contains(&'a'));
}

#[test]
fn partial_tokens_one() {
    let l = create_partial_tokens("hello");
    assert_eq!(l.len(), 1);
    assert_eq!(l.front().unwrap().value, "hello");
}

#[test]
fn partial_tokens_three() {
    let mut l = create_partial_tokens("hello world");
    assert_eq!(l.len(), 3);
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, "hello");
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, " ");
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, "world");
    assert_eq!(l.len(), 0);
}

#[test]
fn clean_tokens_partial() {
    let mut l = create_partial_tokens("hello\n\t\t\tworld\n\t\tbye");
    l = merge_partial_tokens(l);
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, "hello");
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, "\n");
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, "world");
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, "\n");
    let t = l.pop_front().unwrap();
    assert_eq!(t.value, "bye");
    assert_eq!(l.len(), 0);
}

#[test]
fn merge_decimals() {
    let mut l = create_partial_tokens("1.0");
    assert_eq!(l.get(0).unwrap().value, "1");
    assert_eq!(l.get(1).unwrap().value, ".");
    assert_eq!(l.get(2).unwrap().value, "0");
    l = merge_partial_tokens(l);
    assert_eq!(l.len(), 1);
    assert_eq!(l.pop_front().unwrap().value, "1.0");
}

#[test]
fn merge_negative_decimals() {
    let mut l = create_partial_tokens("-1.0");
    assert_eq!(l.get(0).unwrap().value, "-1");
    assert_eq!(l.get(1).unwrap().value, ".");
    assert_eq!(l.get(2).unwrap().value, "0");
    l = merge_partial_tokens(l);
    assert_eq!(l.len(), 1);
    assert_eq!(l.pop_front().unwrap().value, "-1.0");
}

#[test]
fn merge_string() {
    let mut l = create_partial_tokens("\"hello world\"");
    assert_eq!(l.get(0).unwrap().value, "\"");
    assert_eq!(l.get(1).unwrap().value, "hello");
    assert_eq!(l.get(2).unwrap().value, " ");
    assert_eq!(l.get(3).unwrap().value, "world");
    assert_eq!(l.get(4).unwrap().value, "\"");
    l = merge_partial_tokens(l);
    assert_eq!(l.len(), 1);
    assert_eq!(l.pop_front().unwrap().value, "\"hello world\"");
}

#[test]
fn merge_escaped_string() {
    let mut l = create_partial_tokens("\"hello \\\"world\\\"\"");
    assert_eq!(l.get(0).unwrap().value, "\"");
    assert_eq!(l.get(1).unwrap().value, "hello");
    assert_eq!(l.get(2).unwrap().value, " ");
    assert_eq!(l.get(3).unwrap().value, "\\");
    assert_eq!(l.get(4).unwrap().value, "\"");
    assert_eq!(l.get(5).unwrap().value, "world");
    assert_eq!(l.get(6).unwrap().value, "\\");
    assert_eq!(l.get(7).unwrap().value, "\"");
    assert_eq!(l.get(8).unwrap().value, "\"");
    l = merge_partial_tokens(l);
    assert_eq!(l.len(), 1);
    assert_eq!(l.pop_front().unwrap().value, "\"hello \\\"world\\\"\"");
}

#[test]
fn merge_character() {
    let mut l = create_partial_tokens("'a'");
    assert_eq!(l.get(0).unwrap().value, "'");
    assert_eq!(l.get(1).unwrap().value, "a");
    assert_eq!(l.get(2).unwrap().value, "'");
    l = merge_partial_tokens(l);
    assert_eq!(l.len(), 1);
    assert_eq!(l.pop_front().unwrap().value, "'a'");
}

#[test]
fn merge_delimiters() {
    let mut l = create_partial_tokens("(hello world)");
    assert_eq!(l.get(0).unwrap().value, "(");
    assert_eq!(l.get(1).unwrap().value, "hello");
    assert_eq!(l.get(2).unwrap().value, " ");
    assert_eq!(l.get(3).unwrap().value, "world");
    assert_eq!(l.get(4).unwrap().value, ")");
    l = merge_partial_tokens(l);
    assert_eq!(l.len(), 1);
    assert_eq!(l.pop_front().unwrap().value, "(hello world)");
}

#[test]
fn merge_tokens_single() {
    let mut l = create_partial_tokens("hello");
    let t = merge_tokens(&mut l, 1).unwrap();
    assert_eq!(t.value, "hello");
}

#[test]
fn merge_tokens_multiple() {
    let mut l = create_partial_tokens("hello world");
    let t = merge_tokens(&mut l, 3).unwrap();
    assert_eq!(t.value, "hello world");
}

#[test]
fn merge_tokens_consume_only() {
    let mut l = create_partial_tokens("hello world");
    let t = merge_tokens(&mut l, 2).unwrap();
    assert_eq!(t.value, "hello ");
    assert_eq!(l.front().unwrap().value, "world");
}
