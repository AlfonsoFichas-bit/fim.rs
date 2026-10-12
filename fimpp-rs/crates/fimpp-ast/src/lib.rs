#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub name: String,
    pub functions: Vec<Function>,
    pub author: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Print(Expr),
    VarDeclNum(String, Expr),
    VarDeclStr(String, Expr),
    Assign(String, Expr),
    Return(Expr),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    String(String),
    Var(String),
    BinOp(Op, Box<Expr>, Box<Expr>),
    Call(String), // parameterless function call
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}
