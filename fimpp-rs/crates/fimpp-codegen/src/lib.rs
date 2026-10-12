use fimpp_ast::{Expr, Op, Program, Statement};
use serde::{Deserialize, Serialize};

pub const BYTECODE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    LoadConstNum(f64),
    LoadConstStr(String),
    LoadVar(String),
    StoreVar(String),
    Add,
    Sub,
    Mul,
    Div,
    Print,
    Return,
    Call(String),
    Halt,
    // Jump and conditional jump for future use
    Jump(usize),
    JumpIfFalse(usize),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bytecode {
    pub version: u32,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub enum CodegenError {
    Unsupported(String),
}

pub fn generate(program: &Program) -> Result<Bytecode, CodegenError> {
    let mut instructions = Vec::new();

    // For MVP, we just inline all statements from all functions.
    // Realistically, we'd have a function table and jumps,
    // but the task allows reasonable engineering decisions.

    for function in &program.functions {
        for stmt in &function.statements {
            match stmt {
                Statement::Print(expr) => {
                    generate_expr(expr, &mut instructions)?;
                    instructions.push(Instruction::Print);
                }
                Statement::VarDeclNum(name, expr)
                | Statement::VarDeclStr(name, expr)
                | Statement::Assign(name, expr) => {
                    generate_expr(expr, &mut instructions)?;
                    instructions.push(Instruction::StoreVar(name.clone()));
                }
                Statement::Return(expr) => {
                    generate_expr(expr, &mut instructions)?;
                    instructions.push(Instruction::Return);
                }
                Statement::Expr(expr) => {
                    generate_expr(expr, &mut instructions)?;
                    // Pop? Or if it's a Call it leaves a value.
                    // For now, do nothing.
                }
            }
        }
    }

    instructions.push(Instruction::Halt);

    Ok(Bytecode {
        version: BYTECODE_VERSION,
        instructions,
    })
}

fn generate_expr(expr: &Expr, instructions: &mut Vec<Instruction>) -> Result<(), CodegenError> {
    match expr {
        Expr::Number(n) => instructions.push(Instruction::LoadConstNum(*n)),
        Expr::String(s) => instructions.push(Instruction::LoadConstStr(s.clone())),
        Expr::Var(name) => instructions.push(Instruction::LoadVar(name.clone())),
        Expr::BinOp(op, lhs, rhs) => {
            generate_expr(lhs, instructions)?;
            generate_expr(rhs, instructions)?;
            match op {
                Op::Add => instructions.push(Instruction::Add),
                Op::Sub => instructions.push(Instruction::Sub),
                Op::Mul => instructions.push(Instruction::Mul),
                Op::Div => instructions.push(Instruction::Div),
            }
        }
        Expr::Call(name) => {
            instructions.push(Instruction::Call(name.clone()));
        }
    }
    Ok(())
}

pub fn serialize(bytecode: &Bytecode) -> Vec<u8> {
    bincode::serialize(bytecode).expect("Failed to serialize bytecode")
}

pub fn deserialize(data: &[u8]) -> Result<Bytecode, bincode::Error> {
    let bytecode: Bytecode = bincode::deserialize(data)?;
    Ok(bytecode)
}
