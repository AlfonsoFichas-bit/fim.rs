#![allow(clippy::result_large_err)]
use chumsky::prelude::*;
use fimpp_ast::{Expr, Op, Program, Statement};
use fimpp_lexer::Token;

pub type ParserError = Simple<Token>;

pub fn parser() -> impl Parser<Token, Program, Error = ParserError> {
    let ident = select! { Token::Ident(s) => s };

    // An identifier can be multiple words, separated by spaces. In our token stream, they are just contiguous Idents.
    let multi_ident = ident
        .repeated()
        .at_least(1)
        .map(|idents: Vec<String>| idents.join(" "));

    let expr = recursive(|_expr| {
        let val = select! {
            Token::Number(n) => Expr::Number(n.parse().unwrap()),
            Token::String(s) => Expr::String(s),
        }
        .or(multi_ident.map(Expr::Var));

        val.then(
            select! {
                Token::Plus => Op::Add,
                Token::Minus => Op::Sub,
                Token::Times => Op::Mul,
                Token::DividedBy => Op::Div,
            }
            .then(val)
            .repeated(),
        )
        .map(|(lhs, rhs)| {
            rhs.into_iter().fold(lhs, |acc, (op, rhs)| {
                Expr::BinOp(op, Box::new(acc), Box::new(rhs))
            })
        })
    });

    let statement = choice((
        just(Token::Print)
            .ignore_then(expr.clone())
            .then_ignore(just(Token::Dot).or(just(Token::Exclamation)))
            .map(Statement::Print),
        just(Token::VarDeclare)
            .ignore_then(multi_ident)
            .then_ignore(just(Token::IsTheNumber))
            .then(expr.clone())
            .then_ignore(just(Token::Question).or(just(Token::Dot)))
            .map(|(name, e)| Statement::VarDeclNum(name, e)),
        just(Token::VarDeclare)
            .ignore_then(multi_ident)
            .then_ignore(just(Token::IsTheWord))
            .then(expr.clone())
            .then_ignore(just(Token::Question).or(just(Token::Dot)))
            .map(|(name, e)| Statement::VarDeclStr(name, e)),
        multi_ident
            .then_ignore(just(Token::Becomes))
            .then(expr.clone())
            .then_ignore(just(Token::Dot))
            .map(|(name, e)| Statement::Assign(name, e)),
        just(Token::Return)
            .ignore_then(expr.clone())
            .then_ignore(just(Token::Exclamation).or(just(Token::Dot)))
            .map(Statement::Return),
        expr.clone()
            .then_ignore(just(Token::Dot))
            .map(Statement::Expr),
    ));

    let function = just(Token::FunctionStart)
        .ignore_then(multi_ident)
        .then_ignore(just(Token::Dot).or(just(Token::Exclamation)))
        .then(statement.repeated())
        .then_ignore(just(Token::FunctionEnd))
        .then_ignore(multi_ident) // We should ideally check it matches the start name, but let's ignore
        .then_ignore(just(Token::Dot))
        .map(|(name, statements)| fimpp_ast::Function { name, statements });

    just(Token::ProgramStart)
        .ignore_then(multi_ident)
        .then_ignore(just(Token::Exclamation).or(just(Token::Dot)))
        .then(function.repeated())
        .then_ignore(just(Token::ProgramEnd))
        .then(multi_ident)
        .then_ignore(just(Token::Dot))
        .map(|((name, functions), author)| Program {
            name,
            functions,
            author,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    #[test]
    fn test_hello_world() {
        let code = r#"
        Dear Princess Celestia: Hello World!
        Today I learned how to say hello world!
            I said "Hello World!".
        That's all about how to say hello world.
        Your faithful student, Twilight Sparkle.
        "#;
        let tokens: Vec<Token> = Token::lexer(code).map(|res| res.unwrap()).collect();
        let ast = parser().parse(tokens).unwrap();
        assert_eq!(ast.name, "Hello World");
        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "how to say hello world");
        assert_eq!(ast.author, "Twilight Sparkle");
    }
}
