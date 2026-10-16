// =====================================================================
// BYTECODE — código de pila para Fim++.
//
// Este módulo es el primer paso del camino "intérprete -> compilador":
// Define un conjunto pequeño de instrucciones de máquina virtual de pila
// que las siguientes versiones podrán emitir y ejecutar.
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unknown,
    Boolean(bool),
    Character(String),
    Number(f64),
    String(String),
    ArrayValue, // placeholder for array refs in the minimal VM
}

#[derive(Debug, Clone)]
pub enum Op {
    /// Empuja una constante numérica.
    ConstNumber(f64),
    /// Empuja una constante de texto.
    ConstString(String),
    /// Empuja una constante booleana.
    ConstBoolean(bool),
    /// Empuja un carácter.
    ConstCharacter(String),
    /// Empuja el valor `nothing` (Unknown).
    ConstNothing,

    /// Empuja el valor de una variable global (por índice de slot).
    LoadGlobal(usize),
    /// Pop el tope de pila y lo guarda en un slot global.
    StoreGlobal(usize),
    /// Empuja el valor de una local (por índice de slot dentro del frame).
    LoadLocal(usize),
    /// Pop el tope de pila y lo guarda en un slot local.
    StoreLocal(usize),

    /// Incrementa la variable local indicada en un frame.
    IncLocal(usize),
    DecLocal(usize),
    /// Incrementa la variable global indicada.
    IncGlobal(usize),
    DecGlobal(usize),

    /// Aritmética numérica (pop r, pop l, push l OP r).
    AddNum,
    Sub,
    Mul,
    Div,
    Rem,

    /// Concatenación de texto (pop r, pop l, push l++r).
    AddStr,

    /// Igualdad por representación string.
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    And,
    Or,

    /// Acceso a array: tope = índice o valor. Convención del VM: se deja
    /// preparada una carga indireccional basada en LoadLocal/Global + índice.
    ArrayLoadIndex,
    ArrayStoreIndex,

    /// Empuja el resultado de `N of X` (idx se toma de la pila).
    GetArraySlot {
        global_slot: usize,
        base_type: crate::variable::VariableType,
    },
    /// Guarda `value` (tope) en `idx`-ésima posición de `X` (idx tope-1).
    SetArraySlot {
        global_slot: usize,
        value_type: crate::variable::VariableType,
    },

    /// Salto incondicional relativo (+1 adelante del siguiente ip).
    Jmp(i32),
    /// Salta si el tope (pop) es false.
    JmpIfFalse(i32),

    /// Llamada a función por índice. nargs argumentos se toman de la pila.
    Call(usize, usize),
    /// Retorno de función. Si devuelve valor, queda en la pila del llamante.
    Ret,

    /// Elimina el tope de la pila.
    Pop,

    /// Imprime el tope: con salto (PrintNl) o sin salto (PrintQuick).
    PrintNewLine,
    PrintNoNewLine,
}

/// Código de una función.
#[derive(Debug, Clone)]
pub struct FunctionCode {
    pub name: String,
    pub params: usize,
    pub locals: usize,
    pub code: Vec<Op>,
    pub returns: bool,
}

/// Unidad de compilación: código de toutes las funciones + main.
#[derive(Debug, Clone)]
pub struct BytecodeUnit {
    pub globals: usize,
    pub functions: Vec<FunctionCode>,
    pub main_index: usize,
}

impl BytecodeUnit {
    pub fn new() -> Self {
        Self {
            globals: 0,
            functions: vec![],
            main_index: 0,
        }
    }
}