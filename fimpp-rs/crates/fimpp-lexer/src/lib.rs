use logos::Logos;

#[derive(Logos, Debug, PartialEq, Eq, Hash, Clone)]
#[logos(skip r"[ \t\n\f\r]+")]
#[logos(skip r"(?i)p\.s\..*")] // comment
pub enum Token {
    #[token("Dear Princess Celestia:")]
    ProgramStart,

    #[token("Your faithful student,")]
    ProgramEnd,

    #[token("Today I learned")]
    FunctionStart,

    #[token("That's all about")]
    FunctionEnd,

    #[token("Did you know that")]
    VarDeclare,

    #[token("is the number")]
    IsTheNumber,

    #[token("is the word")]
    IsTheWord,

    #[token("becomes")]
    Becomes,

    #[token("I said")]
    Print,

    #[token("Then you get")]
    Return,

    #[token("plus")]
    Plus,

    #[token("minus")]
    Minus,

    #[token("times")]
    Times,

    #[token("divided by")]
    DividedBy,

    #[regex(r#""[^"]*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    String(String),

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().to_string())]
    Number(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[token(".")]
    Dot,

    #[token("!")]
    Exclamation,

    #[token("?")]
    Question,

    #[token(",")]
    Comma,
}
