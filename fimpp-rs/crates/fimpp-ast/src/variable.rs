use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use fimpp_lexer::lexer::{as_boolean_value, as_character_value};
use fimpp_lexer::token::TokenType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableType {
    Unknown,
    Boolean,
    Character,
    Number,
    String,
    BooleanArray,
    NumberArray,
    StringArray,
}

impl VariableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VariableType::Unknown => "",
            VariableType::Boolean => "BOOLEAN",
            VariableType::Character => "CHARACTER",
            VariableType::Number => "NUMBER",
            VariableType::String => "STRING",
            VariableType::BooleanArray => "ARRAY(BOOLEAN)",
            VariableType::NumberArray => "ARRAY(NUMBER)",
            VariableType::StringArray => "ARRAY(STRING)",
        }
    }

    pub fn is_array(&self) -> bool {
        matches!(
            self,
            VariableType::BooleanArray | VariableType::NumberArray | VariableType::StringArray
        )
    }

    pub fn get_default_value(&self) -> Option<String> {
        match self {
            VariableType::Boolean => Some("false".to_string()),
            VariableType::Character => Some("'\0'".to_string()),
            VariableType::Number => Some("0".to_string()),
            VariableType::String => Some("\"\"".to_string()),
            _ => None,
        }
    }

    pub fn as_base_type(&self) -> VariableType {
        match self {
            VariableType::BooleanArray => VariableType::Boolean,
            VariableType::NumberArray => VariableType::Number,
            VariableType::StringArray => VariableType::String,
            _ => VariableType::Unknown,
        }
    }

    pub fn from_token_type(t: TokenType) -> VariableType {
        match t {
            TokenType::Boolean => VariableType::Boolean,
            TokenType::Number => VariableType::Number,
            TokenType::Character => VariableType::Character,
            TokenType::String => VariableType::String,
            _ => VariableType::Unknown,
        }
    }

    pub fn from_token_type_hint(t: TokenType) -> VariableType {
        match t {
            TokenType::TypeBoolean => VariableType::Boolean,
            TokenType::TypeNumber => VariableType::Number,
            TokenType::TypeChar => VariableType::Character,
            TokenType::TypeString => VariableType::String,
            TokenType::TypeBooleanArray => VariableType::BooleanArray,
            TokenType::TypeNumberArray => VariableType::NumberArray,
            TokenType::TypeStringArray => VariableType::StringArray,
            _ => VariableType::Unknown,
        }
    }
}

impl fmt::Display for VariableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn unsanitize_string(value: &str, trim: bool) -> String {
    let bytes = value.as_bytes();
    let mut start = 0;
    let mut amount = bytes.len();

    if trim && amount >= 2 {
        start += 1;
        amount -= 1;
    }

    let mut out = Vec::with_capacity(amount.saturating_sub(start));
    let mut idx = start;

    while idx < amount {
        if bytes[idx] != b'\\' || idx + 1 >= amount {
            out.push(bytes[idx]);
            idx += 1;
            continue;
        }

        match bytes[idx + 1] {
            b'0' => out.push(0),
            b'r' => out.push(b'\r'),
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'"' => out.push(b'"'),
            b'\\' => out.push(b'\\'),
            other => {
                out.push(bytes[idx]);
                out.push(other);
            }
        }
        idx += 2;
    }

    String::from_utf8_lossy(&out).into_owned()
}

#[derive(Debug, Clone, PartialEq)]
pub enum DynamicValue {
    Unknown,
    Boolean(bool),
    Character(String),
    Number(f64),
    String(String),
    Dictionary(Rc<RefCell<HashMap<usize, DynamicVariable>>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicVariable {
    pub value: DynamicValue,
    pub value_type: VariableType,
}

impl DynamicVariable {
    pub fn from_value_type(value: &str, t: VariableType) -> Self {
        match t {
            VariableType::Boolean => {
                let b = as_boolean_value(value)
                    .unwrap_or_else(|| panic!("FromValueType got invalid boolean: {}", value));
                Self::new_boolean(b)
            }
            VariableType::Character => Self::new_character(value),
            VariableType::Number => {
                let n = value
                    .parse::<f64>()
                    .unwrap_or_else(|e| panic!("FromValueType invalid number {}: {}", value, e));
                Self::new_number(n)
            }
            VariableType::String => Self::new_string(value),
            _ => panic!("DynamicVariable::from_value_type called with type {}", t),
        }
    }

    pub fn new_string(value: &str) -> Self {
        Self::new_raw_string(unsanitize_string(value, true))
    }

    pub fn new_raw_string(value: impl Into<String>) -> Self {
        Self {
            value: DynamicValue::String(value.into()),
            value_type: VariableType::String,
        }
    }

    pub fn new_character(value: &str) -> Self {
        let trimmed = &value[1..value.len() - 1];
        let c = as_character_value(trimmed)
            .unwrap_or_else(|| panic!("NewCharacterVariable got invalid character: {}", trimmed));
        Self::new_raw_character(c)
    }

    pub fn new_raw_character(value: impl Into<String>) -> Self {
        Self {
            value: DynamicValue::Character(value.into()),
            value_type: VariableType::Character,
        }
    }

    pub fn new_number(value: f64) -> Self {
        Self {
            value: DynamicValue::Number(value),
            value_type: VariableType::Number,
        }
    }

    pub fn new_boolean(value: bool) -> Self {
        Self {
            value: DynamicValue::Boolean(value),
            value_type: VariableType::Boolean,
        }
    }

    pub fn new_dictionary(t: VariableType) -> Self {
        Self {
            value: DynamicValue::Dictionary(Rc::new(RefCell::new(HashMap::new()))),
            value_type: t,
        }
    }

    pub fn new_unknown() -> Self {
        Self {
            value: DynamicValue::Unknown,
            value_type: VariableType::Unknown,
        }
    }

    pub fn get_value_string(&self) -> String {
        match (&self.value_type, &self.value) {
            (VariableType::String, DynamicValue::String(s)) => s.clone(),
            (VariableType::Character, DynamicValue::Character(c)) => c.clone(),
            (VariableType::Boolean, DynamicValue::Boolean(b)) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            (VariableType::Number, DynamicValue::Number(n)) => {
                if *n == 0.0 {
                    "0".to_string()
                } else {
                    format!("{}", n)
                }
            }
            (VariableType::Unknown, _) => String::new(),
            _ => panic!(
                "Called get_value_string on unhandled value type: {}",
                self.value_type
            ),
        }
    }

    pub fn get_value_character(&self) -> &str {
        if let (VariableType::Character, DynamicValue::Character(c)) =
            (&self.value_type, &self.value)
        {
            c
        } else {
            panic!("Called get_value_character on a non-character variable")
        }
    }

    pub fn set_value_character(&mut self, val: String) {
        if self.value_type != VariableType::Character {
            panic!("Called set_value_character on a non-character variable");
        }
        self.value = DynamicValue::Character(val);
    }

    pub fn set_value_string(&mut self, val: String) {
        if self.value_type != VariableType::String {
            panic!("Called set_value_string on a non-string variable");
        }
        self.value = DynamicValue::String(val);
    }

    pub fn get_value_boolean(&self) -> bool {
        if let (VariableType::Boolean, DynamicValue::Boolean(b)) = (&self.value_type, &self.value) {
            *b
        } else {
            panic!("Called get_value_boolean on a non-boolean variable")
        }
    }

    pub fn set_value_boolean(&mut self, val: bool) {
        if self.value_type != VariableType::Boolean {
            panic!("Called set_value_boolean on a non-boolean variable");
        }
        self.value = DynamicValue::Boolean(val);
    }

    pub fn get_value_number(&self) -> f64 {
        if let (VariableType::Number, DynamicValue::Number(n)) = (&self.value_type, &self.value) {
            *n
        } else {
            panic!("Called get_value_number on a non-number variable")
        }
    }

    pub fn set_value_number(&mut self, val: f64) {
        if self.value_type != VariableType::Number {
            panic!("Called set_value_number on a non-number variable");
        }
        self.value = DynamicValue::Number(val);
    }

    pub fn get_value_dictionary(&self) -> std::cell::Ref<'_, HashMap<usize, DynamicVariable>> {
        if !self.value_type.is_array() {
            panic!("Called get_value_dictionary on a non-dictionary variable");
        }
        if let DynamicValue::Dictionary(map) = &self.value {
            map.borrow()
        } else {
            panic!("Corrupted dictionary variable")
        }
    }

    pub fn dictionary_len(&self) -> usize {
        self.get_value_dictionary().len()
    }

    pub fn dictionary_keys(&self) -> Vec<usize> {
        let mut keys: Vec<usize> = self.get_value_dictionary().keys().copied().collect();
        keys.sort_unstable();
        keys
    }

    pub fn dictionary_get(&self, index: usize) -> Option<DynamicVariable> {
        self.get_value_dictionary().get(&index).cloned()
    }

    pub fn dictionary_set(&self, index: usize, value: DynamicVariable) {
        if !self.value_type.is_array() {
            panic!("Called dictionary_set on a non-dictionary variable");
        }
        if let DynamicValue::Dictionary(map) = &self.value {
            map.borrow_mut().insert(index, value);
        } else {
            panic!("Corrupted dictionary variable")
        }
    }
}
