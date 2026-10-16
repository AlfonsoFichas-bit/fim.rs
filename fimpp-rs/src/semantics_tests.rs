use std::fs;

use crate::ast;
use crate::lexer;
use crate::semantics;

fn analyze_source(source: &str) {
    let tokens = lexer::parse(source);
    let report = ast::create_report(&tokens, source).expect("report creation failed");
    semantics::analyze(&report, source).expect("semantic analysis failed");
}

fn analyze_error(source: &str) {
    let tokens = lexer::parse(source);
    let report = ast::create_report(&tokens, source).expect("report creation failed");
    assert!(semantics::analyze(&report, source).is_err(), "expected semantic error");
}

fn all_samples_ok() {
    let entries = fs::read_dir("../fim/samples").expect("samples dir");
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("fim") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        let tokens = lexer::parse(&source);
        let report = ast::create_report(&tokens, &source).unwrap_or_else(|e| panic!("parse {} failed: {}", path.display(), e));
        semantics::analyze(&report, &source).unwrap_or_else(|e| panic!("semantics {} failed: {}", path.display(), e));
    }
}

#[test]
fn semantics_all_samples_valid() {
    all_samples_ok();
}

#[test]
fn semantics_undefined_variable_errors() {
    let source = r#"Dear Princess Celestia: X!
Today I learned how to run code!
I said DefinitelyUndefined.
That's all about how to run code.
Your faithful student, Twilight Sparkle.
"#;
    analyze_error(source);
}

#[test]
fn semantics_type_mismatch_in_modify_errors() {
    let source = r#"Dear Princess Celestia: X!
Today I learned how to run code!
Did you know that Spike is the number 1?
Spike became correct.
I said Spike.
That's all about how to run code.
Your faithful student, Twilight Sparkle.
"#;
    analyze_error(source);
}

#[test]
fn semantics_non_boolean_condition_errors() {
    let source = r#"Dear Princess Celestia: X!
Today I learned how to branch!
Did you know that Spike is the number 1?
If Spike,
    I said "oops".
That's what I would do.
That's all about how to branch.
Your faithful student, Twilight Sparkle.
"#;
    analyze_error(source);
}

#[test]
fn semantics_unknown_function_call_errors() {
    let source = r#"Dear Princess Celestia: X!
Today I learned how to run code!
I remembered how to definitely not exist.
That's all about how to run code.
Your faithful student, Twilight Sparkle.
"#;
    analyze_error(source);
}

#[test]
fn semantics_arithmetic_ok() {
    let source = r#"Dear Princess Celestia: X!
Today I learned how to run code!
I said 1 plus 2 minus 3 times 4.
That's all about how to run code.
Your faithful student, Twilight Sparkle.
"#;
    analyze_source(source);
}
