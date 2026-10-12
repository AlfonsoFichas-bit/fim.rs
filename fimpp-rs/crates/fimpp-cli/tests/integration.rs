use std::fs;
use std::process::Command;

#[test]
fn test_hello_world() {
    let source = r#"
Dear Princess Celestia: Hello World!
Today I learned how to say hello world!
    I said "Hello World!".
That's all about how to say hello world.
Your faithful student, Twilight Sparkle.
    "#;
    fs::write("tests/hello.fimpp", source).unwrap();
    let output = Command::new("cargo")
        .args(&["run", "-p", "fimpp-cli", "--", "run", "tests/hello.fimpp"])
        .output()
        .expect("Failed to run hello.fimpp");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello World!"));
}

#[test]
fn test_arithmetic() {
    let source = r#"
Dear Princess Celestia: Math!
Today I learned some math!
    I said 10 plus 5.
    I said 10 minus 5.
    I said 10 times 5.
    I said 10 divided by 5.
That's all about some math.
Your faithful student, Twilight Sparkle.
    "#;
    fs::write("tests/arithmetic.fimpp", source).unwrap();
    let output = Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "fimpp-cli",
            "--",
            "run",
            "tests/arithmetic.fimpp",
        ])
        .output()
        .expect("Failed to run arithmetic.fimpp");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("15"));
    assert!(stdout.contains("5"));
    assert!(stdout.contains("50"));
    assert!(stdout.contains("2"));
}

#[test]
fn test_variables() {
    let source = r#"
Dear Princess Celestia: Variables!
Today I learned some variables!
    Did you know that Applejack is the number 10?
    Did you know that Big Mac is the number Applejack plus 5?
    Big Mac becomes Big Mac times 2.
    I said Big Mac.
    Did you know that Name is the word "Apple"?
    I said Name.
That's all about some variables.
Your faithful student, Twilight Sparkle.
    "#;
    fs::write("tests/variables.fimpp", source).unwrap();
    let output = Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "fimpp-cli",
            "--",
            "run",
            "tests/variables.fimpp",
        ])
        .output()
        .expect("Failed to run variables.fimpp");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("30"));
    assert!(stdout.contains("Apple"));
}
