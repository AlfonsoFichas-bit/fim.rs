//! Port de `celestia/interpreter_test.go` del original en Go.
//!
//! Cada test define un reporte FiM++ en línea y comprueba que el intérprete
//! produce exactamente la misma salida (o el mismo error) que el original.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use super::Interpreter;

/// Escritor compartido para capturar la salida del intérprete.
struct SharedWriter(Rc<RefCell<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn execute(source: &str, prompt: Option<&str>) -> Result<String, String> {
    let tokens = fimpp_lexer::lexer::parse(source);
    let report = fimpp_ast::ast::create_report(&tokens, source)?;
    let mut interpreter = Interpreter::new(&report, source)?;

    let buffer = Rc::new(RefCell::new(Vec::new()));
    interpreter.set_writer(Box::new(SharedWriter(buffer.clone())));

    if let Some(value) = prompt {
        let value = value.to_string();
        interpreter.set_prompt(Box::new(move |_prompt: String| Ok(value.clone())));
    }

    interpreter.run()?;

    let output = String::from_utf8_lossy(&buffer.borrow()).into_owned();
    Ok(output)
}

fn expect(source: &str, expected: &str) {
    match execute(source, None) {
        Ok(output) => assert_eq!(output, expected, "\nfuente:\n{}", source),
        Err(err) => panic!("error inesperado:\n{}\nfuente:\n{}", err, source),
    }
}

fn expect_prompt(source: &str, prompt: &str, expected: &str) {
    match execute(source, Some(prompt)) {
        Ok(output) => assert_eq!(output, expected, "\nfuente:\n{}", source),
        Err(err) => panic!("error inesperado:\n{}\nfuente:\n{}", err, source),
    }
}

fn expect_error(source: &str) {
    match execute(source, None) {
        Ok(output) => panic!(
            "se esperaba un error, salida: {:?}\nfuente:\n{}",
            output, source
        ),
        Err(_) => {}
    }
}

// ============================================================================
// TestReport
// ============================================================================

#[test]
fn report_prints_hello_world() {
    let source = r#"Dear Princess Celestia: Hello World!
Today I learned how to say hello world!
I said "Hello World"!
That's all about how to say hello world.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

// ============================================================================
// TestArithmetic
// ============================================================================

#[test]
fn arithmetic_add_numbers() {
    let source = r#"Dear Princess Celestia: Arithmetic!
Today I learned how to run code!
I said 1 plus 1.
That's all about how to run code.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "2\n");
}

#[test]
fn arithmetic_respect_pemdas() {
    let source = r#"Dear Princess Celestia: Arithmetic!
Today I learned how to run code!
I said 2 times 2 plus 1.
That's all about how to run code.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "5\n");
}

// ============================================================================
// TestCases
// ============================================================================

#[test]
fn cases_accept_empty_lines() {
    let source = r#"Dear Princess Celestia: Hello!
Today I learned how to run code!
I said "Hello World!".
...
I said "Goodbye!".
That's all about how to run code.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World!\nGoodbye!\n");
}

// ============================================================================
// TestIO
// ============================================================================

#[test]
fn io_print() {
    let source = r#"Dear Princess Celestia: Outputs!
Today I learned how to output something!
I said 1!
That's all about how to output something.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

#[test]
fn io_print_without_newline() {
    let source = r#"Dear Princess Celestia: Newline Outputs!
Today I learned how to output in the same line!
I quickly said 1!
I quickly said 2!
I said 3!
That's all about how to output in the same line.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "123\n");
}

#[test]
fn io_print_escaped_character() {
    let source = r#"Dear Princess Celestia: Outputs!
Today I learned how to output something!
I said "Hello \\ World"!
That's all about how to output something.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello \\ World\n");
}

#[test]
fn io_ignore_escaped_character() {
    let source = r#"Dear Princess Celestia: Outputs!
Today I learned how to output something!
I said "Hello \e World"!
That's all about how to output something.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello \\e World\n");
}

#[test]
fn io_prompt() {
    let source = r#"Dear Princess Celestia: Prompts!
Today I learned how to acquire prompts!
Did you know that Spike is a word?
I asked Spike: "What do you want me to say? ".
I said Spike!
That's all about how to acquire prompts.
Your faithful student, Twilight Sparkle.
"#;

    expect_prompt(source, "Hello!", "Hello!\n");
}

#[test]
fn io_convert_response_type() {
    let source = r#"Dear Princess Celestia: Prompts!
Today I learned how to acquire prompts!
Did you know that Spike is a number?
I asked Spike: "Give me a number! ".
I said Spike!
That's all about how to acquire prompts.
Your faithful student, Twilight Sparkle.
"#;

    expect_prompt(source, "1", "1\n");
}

#[test]
fn io_error_on_invalid_response_type() {
    let source = r#"Dear Princess Celestia: Prompts!
Today I learned how to acquire prompts!
Did you know that Spike is a number?
I asked Spike: "Give me a number! ".
I said Spike!
That's all about how to acquire prompts.
Your faithful student, Twilight Sparkle.
"#;

    match execute(source, Some("Books!")) {
        Ok(output) => panic!("se esperaba un error, salida: {:?}", output),
        Err(_) => {}
    }
}

// ============================================================================
// TestBasicReports
// ============================================================================

#[test]
fn basic_print_nothing() {
    let source = r#"Dear Princess Celestia: Printing empty!
Today I learned how to print empty!
I said nothing!
That's all about how to print empty.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "\n");
}

#[test]
fn basic_print_string() {
    let source = r#"Dear Princess Celestia: Printing strings!
Today I learned how to print a string!
I said "Hello World"!
That's all about how to print a string.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

#[test]
fn basic_print_character() {
    let source = r#"Dear Princess Celestia: Printing a character!
Today I learned how to print a single char!
I said 'a'!
That's all about how to print a single char.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "a\n");
}

#[test]
fn basic_print_boolean() {
    let source = r#"Dear Princess Celestia: Printing a boolean!
Today I learned how to print a boolean!
I said correct!
That's all about how to print a boolean.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "true\n");
}

#[test]
fn basic_print_number() {
    let source = r#"Dear Princess Celestia: Printing numbers!
Today I learned how to print a numeric value!
I said 1!
That's all about how to print a numeric value.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

// ============================================================================
// TestExpressionNode
// ============================================================================

#[test]
fn expression_add_number() {
    let source = r#"Dear Princess Celestia: Expressions!
Today I learned how to evaluate expressions!
I said 1 plus 1!
That's all about how to evaluate expressions.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "2\n");
}

#[test]
fn expression_subtract_number() {
    let source = r#"Dear Princess Celestia: Expressions!
Today I learned how to evaluate expressions!
I said 1 minus 1!
That's all about how to evaluate expressions.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "0\n");
}

#[test]
fn expression_concatenate_string() {
    let source = r#"Dear Princess Celestia: Concatenations!
Today I learned how to concatenate strings!
I said "Hello" plus " " plus "World"!
That's all about how to concatenate strings.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

#[test]
fn expression_concatenate_multiple_types() {
    let source = r#"Dear Princess Celestia: Concatenations!
Today I learned how to concatenate different types!
I said "Hello " plus 1!
That's all about how to concatenate different types.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello 1\n");
}

// ============================================================================
// TestDeclaration
// ============================================================================

#[test]
fn declaration_create_global_variable() {
    let source = r#"Dear Princess Celestia: Globals!
Did you know that Spike is the number 1?
Today I learned how to print a value!
I said Spike!
That's all about how to print a value.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

#[test]
fn declaration_create_local_variable() {
    let source = r#"Dear Princess Celestia: Locals!
Today I learned how to print a value!
Did you know that Spike is the number 1?
I said Spike!
That's all about how to print a value.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

#[test]
fn declaration_create_empty_variable() {
    let source = r#"Dear Princess Celestia: Empties!
Today I learned how to output empty variables!
Did you know that Spike is a word?
I said Spike!
Did you know that Owlowiscious is an argument?
I said Owlowiscious!
Did you know that Gummy is a number?
I said Gummy!
Did you know that Tank is a letter?
I said Tank!
That's all about how to output empty variables.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "\nfalse\n0\n\u{0}\n");
}

#[test]
fn declaration_create_explicit_empty_variable() {
    let source = r#"Dear Princess Celestia: Empties!
Today I learned how to output an empty variable!
Did you know that Spike is the word nothing?
I said Spike!
Did you know that Owlowiscious is the argument nothing?
I said Owlowiscious!
Did you know that Gummy is the number nothing?
I said Gummy!
Did you know that Tank is the letter nothing?
I said Tank!
That's all about how to output an empty variable.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "\nfalse\n0\n\u{0}\n");
}

#[test]
fn declaration_unset_a_variable() {
    let source = r#"Dear Princess Celestia: Empties!
Today I learned how to output an empty variable!
Did you know that Spike is the number 1?
I said Spike!
Spike became the number nothing.
I said Spike!
That's all about how to output an empty variable.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n0\n");
}

#[test]
fn declaration_create_variable_from_another() {
    let source = r#"Dear Princess Celestia: Variables!
Today I learned how to create variables!
Did you know that Spike is the number 1?
Did you know that Owlowiscious is the number Spike plus 1?
I said Owlowiscious!
That's all about how to create variables.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "2\n");
}

#[test]
fn declaration_fail_on_invalid_value_type() {
    let source = r#"Dear Princess Celestia: Variable Type!
Today I learned how to throw an error!
Did you know that Spike is the number "Hello"?
I said Spike!
That's all about how to throw an error.
Your faithful student, Twilight Sparkle.
"#;

    expect_error(source);
}

// ============================================================================
// TestModify
// ============================================================================

#[test]
fn modify_local_variable() {
    let source = r#"Dear Princess Celestia: Variables!
Today I learned how to modify variables!
Did you know that Spike is the number 2?
Spike became 1.
I said Spike!
That's all about how to modify variables.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

#[test]
fn modify_global_variable() {
    let source = r#"Dear Princess Celestia: Variables!
Did you know that Spike is the number 2?
Today I learned how to modify variables!
Spike became 1.
I said Spike!
That's all about how to modify variables.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

#[test]
fn modify_set_to_default_value() {
    let source = r#"Dear Princess Celestia: Variables!
Today I learned how to reset a variable!
Did you know that Spike is the number 2?
Spike became the word nothing.
I said Spike!
That's all about how to reset a variable.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "0\n");
}

#[test]
fn modify_convert_number_to_string() {
    let source = r#"Dear Princess Celestia: Converting!
Today I learned how to convert types!
Did you know that Spike is the word "Hello"?
Spike became 1.
I said Spike!
That's all about how to convert types.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

#[test]
fn modify_convert_boolean_to_string() {
    let source = r#"Dear Princess Celestia: Converting!
Today I learned how to convert types!
Did you know that Spike is the word "Hello"?
Spike became correct.
I said Spike!
That's all about how to convert types.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "true\n");
}

#[test]
fn modify_set_value_from_function_return() {
    let source = r#"Dear Princess Celestia: Function Return!
I learned how to concatenate using the word x, the word y to get a word!
Then you get x plus " " plus y.
That's all about how to concatenate.
Today I learned how to set values!
Did you know that Spike is a word?
Spike became how to concatenate using "hello world".
I said Spike!
Spike became how to concatenate using "hello", "world".
I said Spike!
That's all about how to set values.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "hello world \nhello world\n");
}

// ============================================================================
// TestUnary
// ============================================================================

#[test]
fn unary_increment_number() {
    let source = r#"Dear Princess Celestia: Unaries!
Today I learned how to increment a value!
Did you know that Spike is the number 1?
Spike got one more.
There was one more Spike.
I said Spike!
That's all about how to increment a value.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "3\n");
}

#[test]
fn unary_decrement_number() {
    let source = r#"Dear Princess Celestia: Unaries!
Today I learned how to decrement a value!
Did you know that Spike is the number 3?
Spike got one less.
There was one less Spike.
I said Spike!
That's all about how to decrement a value.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n");
}

#[test]
fn unary_increment_array_number() {
    let source = r#"Dear Princess Celestia: Unaries!
Today I learned how to increment a value!
Did you know that Spike is the numbers 1, 2, 3?
1 of Spike got one more.
There was one more 2 of Spike.
I said 1 of Spike!
I said 2 of Spike!
I said 3 of Spike!
That's all about how to increment a value.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "2\n3\n3\n");
}

#[test]
fn unary_only_works_for_numbers() {
    let source = r#"Dear Princess Celestia: Unaries!
Today I learned how to decrement a value!
Did you know that Spike is the word "Apples"?
Spike got one more.
I said Spike!
That's all about how to decrement a value.
Your faithful student, Twilight Sparkle.
"#;

    expect_error(source);
}

// ============================================================================
// TestArray
// ============================================================================

#[test]
fn array_create_empty() {
    let source = r#"Dear Princess Celestia: Arrays!
Today I learned how to print arrays!
Did you know that Apples has many words?
I said 1 of Apples!
That's all about how to print arrays.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "\n");
}

#[test]
fn array_print_out_of_bounds() {
    let source = r#"Dear Princess Celestia: Arrays!
Today I learned how to print arrays!
Did you know that Apples has many words?
I said 100 of Apples!
That's all about how to print arrays.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "\n");
}

#[test]
fn array_print() {
    let source = r#"Dear Princess Celestia: Arrays!
Today I learned how to print arrays!
Did you know that Apples has the words "Gala", "Red Delicious", "Mcintosh", "Honeycrisp"?
I said 1 of Apples!
I said 2 of Apples!
I said 3 of Apples!
I said 4 of Apples!
That's all about how to print arrays.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Gala\nRed Delicious\nMcintosh\nHoneycrisp\n");
}

#[test]
fn array_print_nothing_on_out_of_range() {
    let source = r#"Dear Princess Celestia: Arrays!
Today I learned how to print arrays!
Did you know that Apples has the words "Gala"?
I said 1 of Apples!
I said 2 of Apples!
That's all about how to print arrays.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Gala\n\n");
}

#[test]
fn array_modify_at_index() {
    let source = r#"Dear Princess Celestia: Arrays!
Today I learned how to modify arrays!
Did you know that Apples has the words "Gala", "Red Delicious", "Mcintosh", "Honeycrisp"?
I said 1 of Apples!
1 of Apples is "Gala!".
I said 1 of Apples!

Did you know that Applebloom is the number 1?
Applebloom of Apples is "Gala".
I said Applebloom of Apples.
That's all about how to modify arrays.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Gala\nGala!\nGala\n");
}

// ============================================================================
// TestFunctions
// ============================================================================

#[test]
fn functions_run_function() {
    let source = r#"Dear Princess Celestia: Functions!
I learned how to say hello world!
I said "Hello World"!
That's all about how to say hello world.
Today I learned how to run a function!
I remembered how to say hello world.
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

#[test]
fn functions_return_a_value() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to ask to get a word!
Then you get "Hello World"!
That's all about how to ask.
Today I learned how to run a function!
I said how to ask.
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

#[test]
fn functions_run_even_with_return() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to ask to get a word!
Then you get "Hello World"!
That's all about how to ask.
Today I learned how to run a function!
I remembered how to ask.
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "");
}

#[test]
fn functions_accept_a_value() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to give a text using the word text!
I said "Hello " plus text.
That's all about how to give a text.
Today I learned how to run a function!
I remembered how to give a text using the word "World".
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

#[test]
fn functions_handle_multiple_values() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to give a text using the word x, the word y to get a word!
Then you get x plus " " plus y.
That's all about how to give a text.
Today I learned how to run a function!
I remembered how to give a text using the word "Hello", "World".
I said how to give a text using the word "Hello", "World".
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

#[test]
fn functions_handle_multiple_values_to_variable() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to give a text using the word x, the word y to get a word!
Then you get x plus " " plus y.
That's all about how to give a text.
Today I learned how to run a function!
Did you know that Spike is the word how to give a text using the word "Hello", "World".
I said Spike.
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\n");
}

#[test]
fn functions_handle_default_value() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to give a text using the word x, the number y!
I said x.
I said y.
That's all about how to give a text.
Today I learned how to run a function!
I remembered how to give a text using the word "Hello".
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello\n0\n");
}

#[test]
fn functions_allow_changing_value() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to receive a text using the word x!
x became "World!".
That's all about how to receive a text.
Today I learned how to run a function!
I remembered how to receive a text using the word "Hello".
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "");
}

#[test]
fn functions_allow_not_changing_variable_value() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to receive a text using the word x!
x became "World!".
That's all about how to receive a text.
Today I learned how to run a function!
Did you know that Spike is the word "Hello"?
I remembered how to receive a text using the word Spike.
I said Spike.
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello\n");
}

#[test]
fn functions_allow_changing_value_of_array() {
    let source = r#"Dear Princess Celestia: Returns!
I learned how to receive a text using the words Apples!
1 of Apples is "Gala!".
That's all about how to receive a text.
Today I learned how to run a function!
Did you know that Apples has the words "Gala", "Red Delicious", "Mcintosh", "Honeycrisp"?
I remembered how to receive a text using Apples.
I said 1 of Apples.
That's all about how to run a function.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Gala!\n");
}

// ============================================================================
// TestIfStatements
// ============================================================================

#[test]
fn if_run_statement() {
    let source = r#"Dear Princess Celestia: Statements!
Today I learned how to branch statements!
Did you know that Spike is the number 1?
If Spike is equal to 1 then,
I said "Hello World".
That's what I would do.
I said "Hello Equestria!".
That's all about how to branch statements.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello World\nHello Equestria!\n");
}

#[test]
fn if_ignore_statement() {
    let source = r#"Dear Princess Celestia: Statements!
Today I learned how to branch statements!
Did you know that Spike is the number 2?
If Spike is equal to 1,
I said "Hello World".
That's what I would do.
I said "Hello Equestria!".
That's all about how to branch statements.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello Equestria!\n");
}

#[test]
fn if_fallback_to_else_statement() {
    let source = r#"Dear Princess Celestia: Statements!
Today I learned how to branch statements!
Did you know that Spike is the number 2?
If Spike is equal to 1,
I said "Nope! Not this one.".
Otherwise,
I said "Hello Equestria".
That's what I would do.
That's all about how to branch statements.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello Equestria\n");
}

#[test]
fn if_run_if_else_statement() {
    let source = r#"Dear Princess Celestia: Statements!
Today I learned how to branch statements!
Did you know that Spike is the number 2?
If Spike is equal to 1,
I said "Nope! Not this one.".
Otherwise Spike is equal to 2,
I said "Hello Ponyville".
Otherwise,
I said "Well that isn't right!".
That's what I would do.
That's all about how to branch statements.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Hello Ponyville\n");
}

#[test]
fn if_error_on_multiple_else_statements() {
    let source = r#"Dear Princess Celestia: Statements!
Today I learned how to branch statements!
Did you know that Spike is the number 2?
If Spike is equal to 1,
I said "Nope! Not this one.".
Otherwise,
I said "Well that isn't right!".
Otherwise,
I said "This isn't right either!".
That's what I would do.
That's all about how to branch statements.
Your faithful student, Twilight Sparkle.
"#;

    expect_error(source);
}

// ============================================================================
// TestWhileStatements
// ============================================================================

#[test]
fn while_run_statement() {
    let source = r#"Dear Princess Celestia: While Loops!
Today I learned how to run loops!
Did you know that Spike is the number 1?
As long as Spike is no greater than 5...
I said Spike.
Spike got one more.
That's what I did.
That's all about how to run loops.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n2\n3\n4\n5\n");
}

#[test]
fn while_ignore_statement() {
    let source = r#"Dear Princess Celestia: While Loops!
Today I learned how to run loops!
Did you know that Spike is the number 10?
As long as Spike is no greater than 5...
I said Spike.
Spike got one more.
That's what I did.
That's all about how to run loops.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "");
}

#[test]
fn while_skip_statement() {
    let source = r#"Dear Princess Celestia: While Loops!
I learned how to count!
Did you know that Spike is the number 1?
As long as Spike is no greater than 5...
I said Spike.

If Spike is equal to 4,
Then you get nothing!
That's what I would do.

Spike got one more.
That's what I did.
That's all about how to count.

Today I learned how to run loops!
I remembered how to count.
That's all about how to run loops.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n2\n3\n4\n");
}

// ============================================================================
// TestForEveryStatements
// ============================================================================

#[test]
fn for_every_range_forwards() {
    let source = r#"Dear Princess Celestia: For every number!
Today I learned how to count!
For every number Spike from 1 to 5,
I said Spike.
That's what I did.
That's all about how to count.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "1\n2\n3\n4\n5\n");
}

#[test]
fn for_every_range_backwards() {
    let source = r#"Dear Princess Celestia: For every number!
Today I learned how to count!
For every number Spike from 5 to 1,
I said Spike.
That's what I did.
That's all about how to count.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "5\n4\n3\n2\n1\n");
}

#[test]
fn for_every_iterate_through_string() {
    let source = r#"Dear Princess Celestia: Iteration!
Today I learned how to iterate!
Did you know that Applejack is the word "Apples"?
For every character c in Applejack...
I quickly said c.
That's what I did.
That's all about how to iterate.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Apples");
}

#[test]
fn for_every_iterate_through_array() {
    let source = r#"Dear Princess Celestia: Iteration!
Today I learned how to iterate!
Did you know that Apples has the words "Gala", "Red Delicious", "Mcintosh", "Honeycrisp"?

For every word type in Apples...
I said type.
That's what I did.
That's all about how to iterate.
Your faithful student, Twilight Sparkle.
"#;

    expect(source, "Gala\nRed Delicious\nMcintosh\nHoneycrisp\n");
}
