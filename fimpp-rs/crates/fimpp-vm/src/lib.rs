use fimpp_codegen::{BYTECODE_VERSION, Bytecode, Instruction};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    StackUnderflow,
    TypeError(String),
    UnknownVariable(String),
    MissingFunction(String),
    InvalidJump(usize),
    InvalidVersion(u32),
    UnknownOpcode, // If a deserialized opcode is malformed (handled by bincode, but good to have)
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackUnderflow => write!(f, "Stack underflow"),
            RuntimeError::TypeError(msg) => write!(f, "Type error: {}", msg),
            RuntimeError::UnknownVariable(name) => write!(f, "Unknown variable: {}", name),
            RuntimeError::MissingFunction(name) => write!(f, "Missing function: {}", name),
            RuntimeError::InvalidJump(addr) => write!(f, "Invalid jump address: {}", addr),
            RuntimeError::InvalidVersion(v) => write!(f, "Invalid bytecode version: {}", v),
            RuntimeError::UnknownOpcode => write!(f, "Unknown opcode"),
        }
    }
}

pub struct VM {
    stack: Vec<Value>,
    variables: HashMap<String, Value>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
        }
    }

    pub fn execute(&mut self, bytecode: &Bytecode) -> Result<(), RuntimeError> {
        if bytecode.version != BYTECODE_VERSION {
            return Err(RuntimeError::InvalidVersion(bytecode.version));
        }

        let mut ip = 0;
        while ip < bytecode.instructions.len() {
            let inst = &bytecode.instructions[ip];
            match inst {
                Instruction::LoadConstNum(n) => {
                    self.stack.push(Value::Number(*n));
                }
                Instruction::LoadConstStr(s) => {
                    self.stack.push(Value::String(s.clone()));
                }
                Instruction::LoadVar(name) => {
                    if let Some(val) = self.variables.get(name) {
                        self.stack.push(val.clone());
                    } else {
                        return Err(RuntimeError::UnknownVariable(name.clone()));
                    }
                }
                Instruction::StoreVar(name) => {
                    let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    self.variables.insert(name.clone(), val);
                }
                Instruction::Add => {
                    let rhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    let lhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    match (lhs, rhs) {
                        (Value::Number(a), Value::Number(b)) => {
                            self.stack.push(Value::Number(a + b))
                        }
                        _ => {
                            return Err(RuntimeError::TypeError(
                                "Cannot add non-numbers".to_string(),
                            ));
                        }
                    }
                }
                Instruction::Sub => {
                    let rhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    let lhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    match (lhs, rhs) {
                        (Value::Number(a), Value::Number(b)) => {
                            self.stack.push(Value::Number(a - b))
                        }
                        _ => {
                            return Err(RuntimeError::TypeError(
                                "Cannot subtract non-numbers".to_string(),
                            ));
                        }
                    }
                }
                Instruction::Mul => {
                    let rhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    let lhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    match (lhs, rhs) {
                        (Value::Number(a), Value::Number(b)) => {
                            self.stack.push(Value::Number(a * b))
                        }
                        _ => {
                            return Err(RuntimeError::TypeError(
                                "Cannot multiply non-numbers".to_string(),
                            ));
                        }
                    }
                }
                Instruction::Div => {
                    let rhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    let lhs = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    match (lhs, rhs) {
                        (Value::Number(a), Value::Number(b)) => {
                            self.stack.push(Value::Number(a / b))
                        }
                        _ => {
                            return Err(RuntimeError::TypeError(
                                "Cannot divide non-numbers".to_string(),
                            ));
                        }
                    }
                }
                Instruction::Print => {
                    let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    println!("{}", val);
                }
                Instruction::Return => {
                    // For now, MVP return just stops execution or returns a value from the function.
                    // Since we flattened functions, we just stop.
                    break;
                }
                Instruction::Call(name) => {
                    return Err(RuntimeError::MissingFunction(name.clone()));
                }
                Instruction::Halt => {
                    break;
                }
                Instruction::Jump(target) => {
                    if *target >= bytecode.instructions.len() {
                        return Err(RuntimeError::InvalidJump(*target));
                    }
                    ip = *target;
                    continue;
                }
                Instruction::JumpIfFalse(target) => {
                    if *target >= bytecode.instructions.len() {
                        return Err(RuntimeError::InvalidJump(*target));
                    }
                    let val = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    let condition = match val {
                        Value::Number(n) => n != 0.0,
                        Value::String(_) => true,
                    };
                    if !condition {
                        ip = *target;
                        continue;
                    }
                }
            }
            ip += 1;
        }
        Ok(())
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
