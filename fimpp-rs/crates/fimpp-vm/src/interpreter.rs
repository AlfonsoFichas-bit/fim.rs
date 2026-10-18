// Port de celestia/*.go (interpreter.go, paragraph.go, variables.go, value.go, statements.go)
//
// Es un intérprete que recorre el árbol que produce `ast.rs`, replicando la
// semántica del original en Go (incluyendo los detalles "raros" como que los
// arrays se comparten por referencia al pasarlos a un párrafo/función).

use std::cell::RefCell;
use std::cmp::min;
use std::io::{self, Write};
use std::rc::Rc;

use fimpp_ast::ast::*;
use fimpp_ast::variable::{DynamicVariable, VariableType};
use fimpp_lexer::lexer::{as_boolean_value, as_character_value};
use fimpp_semantics::errors::parse_error;

// ============================================================================
// VARIABLES (celestia/variables.go)
// ============================================================================

pub struct Variable {
    pub name: String,
    pub value: Rc<RefCell<DynamicVariable>>,
    pub constant: bool,
}

impl Variable {
    fn new(name: String, value: DynamicVariable, constant: bool) -> Self {
        Self {
            name,
            value: Rc::new(RefCell::new(value)),
            constant,
        }
    }
}

struct VariableManager {
    globals: Vec<Rc<RefCell<Variable>>>,
    // Pila de ámbitos (uno por párrafo en ejecución).
    locals: Vec<Vec<Rc<RefCell<Variable>>>>,
}

impl VariableManager {
    fn new() -> Self {
        Self {
            globals: Vec::new(),
            locals: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.locals.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    fn scope_depth(&self) -> usize {
        self.locals.len()
    }

    fn push_variable(&mut self, variable: Variable, global: bool) {
        if global {
            self.globals.push(Rc::new(RefCell::new(variable)));
        } else {
            if self.scope_depth() == 0 {
                panic!("VariableManager@PushVariable called with no variable scopes");
            }
            self.locals
                .last_mut()
                .unwrap()
                .push(Rc::new(RefCell::new(variable)));
        }
    }

    fn pop_variable(&mut self, global: bool) -> Option<Rc<RefCell<Variable>>> {
        if global {
            self.globals.pop()
        } else {
            if self.scope_depth() == 0 {
                panic!("VariableManager@PopVariable called with no variable scopes");
            }
            self.locals.last_mut().unwrap().pop()
        }
    }

    fn pop_variable_amount(&mut self, global: bool, amount: usize) {
        if global {
            for _ in 0..amount {
                if self.globals.pop().is_none() {
                    break;
                }
            }
        } else {
            if self.scope_depth() == 0 {
                panic!("VariableManager@PopVariableAmount called with no variable scopes");
            }
            let current = self.locals.last_mut().unwrap();
            for _ in 0..amount {
                if current.pop().is_none() {
                    break;
                }
            }
        }
    }

    fn get(&self, name: &str, local: bool) -> Option<Rc<RefCell<Variable>>> {
        for variable in &self.globals {
            if variable.borrow().name == name {
                return Some(variable.clone());
            }
        }

        if local && self.scope_depth() > 0 {
            let current = self.locals.last().unwrap();
            for variable in current {
                if variable.borrow().name == name {
                    return Some(variable.clone());
                }
            }
        }

        None
    }

    fn has(&self, name: &str, local: bool) -> bool {
        self.get(name, local).is_some()
    }
}

// ============================================================================
// PÁRRAFO (celestia/paragraph.go)
// ============================================================================

struct Paragraph {
    name: String,
    main: bool,
    return_type: VariableType,
    node: FunctionNode,
}

// ============================================================================
// INTÉRPRETE (celestia/interpreter.go + value.go + statements.go)
// ============================================================================

pub struct Interpreter {
    source: String,
    title: String,
    author: String,

    variables: VariableManager,
    paragraphs: Vec<Paragraph>,

    out: Box<dyn Write>,
    prompt: Box<dyn FnMut(String) -> Result<String, String>>,
}

fn default_prompt() -> Box<dyn FnMut(String) -> Result<String, String>> {
    Box::new(|prompt: String| {
        print!("{}", prompt);
        io::stdout().flush().ok();

        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
        Ok(line)
    })
}

impl Interpreter {
    pub fn new(report: &ReportNode, source: &str) -> Result<Interpreter, String> {
        let mut interpreter = Interpreter {
            source: source.to_string(),
            title: report.title.clone(),
            author: report.author.clone(),
            variables: VariableManager::new(),
            paragraphs: Vec::new(),
            out: Box::new(io::stdout()),
            prompt: default_prompt(),
        };

        for node in &report.body {
            match node {
                DynamicNode::Function(function) => {
                    if interpreter
                        .paragraphs
                        .iter()
                        .any(|p| p.name == function.name)
                    {
                        return Err(parse_error(
                            &format!("Paragraph '{}' already exists", function.name),
                            &interpreter.source,
                            function.node.start,
                        ));
                    }

                    interpreter.paragraphs.push(Paragraph {
                        name: function.name.clone(),
                        main: function.main,
                        return_type: function.return_type,
                        node: function.clone(),
                    });
                }

                DynamicNode::VariableDeclaration(declaration) => {
                    if interpreter
                        .variables
                        .get(&declaration.identifier, true)
                        .is_some()
                    {
                        return Err(parse_error(
                            &format!("Variable '{}' already exists.", declaration.identifier),
                            &interpreter.source,
                            declaration.node.start,
                        ));
                    }

                    let mut value = interpreter.evaluate_value_node(&declaration.value, false)?;

                    if value.value_type == VariableType::Unknown {
                        if declaration.value_type.is_array() {
                            value = DynamicVariable::new_dictionary(declaration.value_type);
                        } else {
                            let default = declaration
                                .value_type
                                .get_default_value()
                                .expect("could not get default value");
                            value =
                                DynamicVariable::from_value_type(&default, declaration.value_type);
                        }
                    }

                    if !declaration.value_type.is_array() {
                        value = value.clone();
                    }

                    interpreter.variables.push_variable(
                        Variable::new(declaration.identifier.clone(), value, declaration.constant),
                        true,
                    );
                }

                _ => {
                    return Err(parse_error(
                        "Unsupported report body node",
                        &interpreter.source,
                        node.to_node().start,
                    ));
                }
            }
        }

        Ok(interpreter)
    }

    pub fn report_title(&self) -> &str {
        &self.title
    }

    pub fn report_author(&self) -> &str {
        &self.author
    }

    pub fn set_writer(&mut self, writer: Box<dyn Write>) {
        self.out = writer;
    }

    pub fn set_prompt(&mut self, prompt: Box<dyn FnMut(String) -> Result<String, String>>) {
        self.prompt = prompt;
    }

    // Ejecuta el párrafo principal (equivalente a main.go).
    pub fn run(&mut self) -> Result<(), String> {
        if let Some(index) = self.paragraphs.iter().position(|p| p.main) {
            self.execute_paragraph_index(index, Vec::new())?;
        }
        Ok(())
    }

    // --- Evaluación de valores (celestia/value.go) ---

    fn evaluate_value_node(
        &mut self,
        node: &DynamicNode,
        local: bool,
    ) -> Result<DynamicVariable, String> {
        match node {
            DynamicNode::Literal(literal) => Ok(literal.variable.clone()),

            DynamicNode::LiteralDictionary(dictionary) => {
                let value = DynamicVariable::new_dictionary(dictionary.array_type);
                for index in dictionary.values.keys().copied() {
                    let evaluated = self.evaluate_value_node(&dictionary.values[&index], local)?;
                    value.dictionary_set(index, evaluated);
                }
                Ok(value)
            }

            DynamicNode::Identifier(identifier) => {
                if let Some(variable) = self.variables.get(&identifier.identifier, local) {
                    return Ok(variable.borrow().value.borrow().clone());
                }

                if let Some(index) = self
                    .paragraphs
                    .iter()
                    .position(|p| p.name == identifier.identifier)
                {
                    let value = self.execute_paragraph_index(index, Vec::new())?;
                    return Ok(value.unwrap_or_else(DynamicVariable::new_unknown));
                }

                Err(parse_error(
                    &format!("Unknown identifier ({})", identifier.identifier),
                    &self.source,
                    identifier.node.start,
                ))
            }

            DynamicNode::FunctionCall(call) => {
                let index = self
                    .paragraphs
                    .iter()
                    .position(|p| p.name == call.identifier)
                    .ok_or_else(|| {
                        parse_error(
                            &format!("Unknown paragraph ({})", call.identifier),
                            &self.source,
                            call.node.start,
                        )
                    })?;

                if self.paragraphs[index].return_type == VariableType::Unknown {
                    return Err(parse_error(
                        "Tried calling a function that doesn't return a value",
                        &self.source,
                        call.node.start,
                    ));
                }

                let mut parameters = Vec::new();
                for parameter in &call.parameters {
                    parameters.push(self.evaluate_value_node(parameter, local)?);
                }

                let value = self.execute_paragraph_index(index, parameters)?;
                Ok(value.unwrap_or_else(DynamicVariable::new_unknown))
            }

            DynamicNode::DictionaryIdentifier(identifier) => {
                let variable = self
                    .variables
                    .get(&identifier.identifier, local)
                    .ok_or_else(|| {
                        parse_error(
                            &format!("Unknown identifier ({})", identifier.identifier),
                            &self.source,
                            identifier.node.start,
                        )
                    })?;

                let value_type = variable.borrow().value.borrow().value_type;
                if !value_type.is_array() && value_type != VariableType::String {
                    return Err(parse_error(
                        &format!(
                            "Invalid non-dicionary identifier ({})",
                            identifier.identifier
                        ),
                        &self.source,
                        identifier.node.start,
                    ));
                }

                let index = self.evaluate_value_node(&identifier.index, local)?;
                if index.value_type != VariableType::Number {
                    return Err(parse_error(
                        &format!("Expected numeric index, got type {}", index.value_type),
                        &self.source,
                        identifier.index.to_node().start,
                    ));
                }

                let index_as_integer = index.get_value_number() as i64;

                if value_type == VariableType::String {
                    let string = variable.borrow().value.borrow().get_value_string();
                    let bytes = string.as_bytes();
                    let position = index_as_integer - 1;
                    if position < 0 || position as usize >= bytes.len() {
                        return Ok(DynamicVariable::new_raw_character(""));
                    }
                    let byte = bytes[position as usize];
                    return Ok(DynamicVariable::new_raw_character(
                        char::from(byte).to_string(),
                    ));
                }

                let base_type = value_type.as_base_type();
                if index_as_integer < 0 {
                    let default = base_type.get_default_value().unwrap();
                    return Ok(DynamicVariable::from_value_type(&default, base_type));
                }

                if let Some(value) = variable
                    .borrow()
                    .value
                    .borrow()
                    .dictionary_get(index_as_integer as usize)
                {
                    return Ok(value);
                }

                let default = base_type.get_default_value().unwrap();
                Ok(DynamicVariable::from_value_type(&default, base_type))
            }

            DynamicNode::BinaryExpression(expression) => {
                let left = self.evaluate_value_node(&expression.left, local)?;
                let right = self.evaluate_value_node(&expression.right, local)?;

                if expression.operator == BinaryExpressionOperator::Add
                    && (left.value_type == VariableType::String
                        || right.value_type == VariableType::String)
                {
                    return Ok(DynamicVariable::new_raw_string(format!(
                        "{}{}",
                        left.get_value_string(),
                        right.get_value_string()
                    )));
                }

                match expression.operator {
                    BinaryExpressionOperator::Add => Ok(DynamicVariable::new_number(
                        left.get_value_number() + right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Sub => Ok(DynamicVariable::new_number(
                        left.get_value_number() - right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Mul => Ok(DynamicVariable::new_number(
                        left.get_value_number() * right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Div => Ok(DynamicVariable::new_number(
                        left.get_value_number() / right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Mod => {
                        // math.Mod(a, b) == a % b para f64
                        Ok(DynamicVariable::new_number(
                            left.get_value_number() % right.get_value_number(),
                        ))
                    }
                    BinaryExpressionOperator::And => Ok(DynamicVariable::new_boolean(
                        left.get_value_boolean() && right.get_value_boolean(),
                    )),
                    BinaryExpressionOperator::Or => Ok(DynamicVariable::new_boolean(
                        left.get_value_boolean() || right.get_value_boolean(),
                    )),
                    BinaryExpressionOperator::Gte => Ok(DynamicVariable::new_boolean(
                        left.get_value_number() >= right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Lte => Ok(DynamicVariable::new_boolean(
                        left.get_value_number() <= right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Gt => Ok(DynamicVariable::new_boolean(
                        left.get_value_number() > right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Lt => Ok(DynamicVariable::new_boolean(
                        left.get_value_number() < right.get_value_number(),
                    )),
                    BinaryExpressionOperator::Neq => Ok(DynamicVariable::new_boolean(
                        left.get_value_string() != right.get_value_string(),
                    )),
                    BinaryExpressionOperator::Eq => Ok(DynamicVariable::new_boolean(
                        left.get_value_string() == right.get_value_string(),
                    )),
                    BinaryExpressionOperator::Unknown => Err(parse_error(
                        "Unsupported binary expression operator",
                        &self.source,
                        expression.node.start,
                    )),
                }
            }

            _ => Err(parse_error(
                "Unsupported value node",
                &self.source,
                node.to_node().start,
            )),
        }
    }

    // --- Ejecución de enunciados (celestia/statements.go) ---

    fn evaluate_statements_node(
        &mut self,
        statements: &StatementsNode,
    ) -> Result<Option<DynamicVariable>, String> {
        let mut new_variable_count = 0usize;
        let result = self.evaluate_statements_inner(statements, &mut new_variable_count);
        self.variables
            .pop_variable_amount(false, new_variable_count);
        result
    }

    fn evaluate_statements_inner(
        &mut self,
        statements: &StatementsNode,
        new_variable_count: &mut usize,
    ) -> Result<Option<DynamicVariable>, String> {
        for statement in &statements.statements {
            match statement {
                DynamicNode::Print(node) => {
                    let value = self.evaluate_value_node(&node.value, true)?;

                    if value.value_type.is_array() {
                        return Err(parse_error(
                            "Cannot print an array value",
                            &self.source,
                            node.node.start,
                        ));
                    }

                    self.out
                        .write_all(value.get_value_string().as_bytes())
                        .map_err(|e| e.to_string())?;

                    if node.newline {
                        self.out.write_all(b"\n").map_err(|e| e.to_string())?;
                    }
                }

                DynamicNode::Prompt(node) => {
                    if !self.variables.has(&node.identifier, true) {
                        return Err(parse_error(
                            &format!("Variable '{}' does not exist.", node.identifier),
                            &self.source,
                            node.node.start,
                        ));
                    }
                    let variable = self.variables.get(&node.identifier, true).unwrap();

                    if variable.borrow().constant {
                        return Err(parse_error(
                            "Cannot modify a constant variable.",
                            &self.source,
                            node.node.start,
                        ));
                    }

                    if variable.borrow().value.borrow().value_type.is_array() {
                        return Err(parse_error(
                            "Expected variable to be of non-array type",
                            &self.source,
                            node.node.start,
                        ));
                    }

                    let value = self.evaluate_value_node(&node.prompt, true)?;
                    if value.value_type != VariableType::String {
                        return Err(parse_error(
                            "Expected prompt to be of type STRING",
                            &self.source,
                            node.node.start,
                        ));
                    }

                    let response = (self.prompt)(value.get_value_string())?;

                    let value_type = variable.borrow().value.borrow().value_type;
                    let value_ref = variable.borrow().value.clone();
                    let mut target = value_ref.borrow_mut();
                    match value_type {
                        VariableType::String => target.set_value_string(response),
                        VariableType::Character => {
                            let character = as_character_value(&response).ok_or_else(|| {
                                parse_error(
                                    &format!("Invalid character value: {}", response),
                                    &self.source,
                                    node.prompt.to_node().start,
                                )
                            })?;
                            target.set_value_character(character);
                        }
                        VariableType::Boolean => {
                            let boolean = as_boolean_value(&response).ok_or_else(|| {
                                parse_error(
                                    &format!("Invalid boolean value: {}", response),
                                    &self.source,
                                    node.prompt.to_node().start,
                                )
                            })?;
                            target.set_value_boolean(boolean);
                        }
                        VariableType::Number => {
                            let number = response.parse::<f64>().map_err(|_| {
                                parse_error(
                                    &format!("Invalid number value: {}", response),
                                    &self.source,
                                    node.prompt.to_node().start,
                                )
                            })?;
                            target.set_value_number(number);
                        }
                        _ => {}
                    }
                }

                DynamicNode::VariableDeclaration(node) => {
                    if self.variables.get(&node.identifier, true).is_some() {
                        return Err(parse_error(
                            &format!("Variable '{}' already exists.", node.identifier),
                            &self.source,
                            node.node.start,
                        ));
                    }

                    let mut value = self.evaluate_value_node(&node.value, true)?;

                    if value.value_type == VariableType::Unknown {
                        if node.value_type.is_array() {
                            value = DynamicVariable::new_dictionary(node.value_type);
                        } else {
                            let default = node.value_type.get_default_value().unwrap();
                            value = DynamicVariable::from_value_type(&default, node.value_type);
                        }
                    }

                    if node.value_type != value.value_type {
                        if node.value_type == VariableType::String && !value.value_type.is_array() {
                            value = DynamicVariable::new_raw_string(value.get_value_string());
                        } else {
                            return Err(parse_error(
                                &format!(
                                    "Expected type '{}', got '{}'",
                                    node.value_type, value.value_type
                                ),
                                &self.source,
                                node.node.start,
                            ));
                        }
                    }

                    self.variables.push_variable(
                        Variable::new(node.identifier.clone(), value, node.constant),
                        false,
                    );
                    *new_variable_count += 1;
                }

                DynamicNode::VariableModify(node) => {
                    if !self.variables.has(&node.identifier, true) {
                        return Err(parse_error(
                            &format!("Variable '{}' does not exist.", node.identifier),
                            &self.source,
                            node.node.start,
                        ));
                    }
                    let variable = self.variables.get(&node.identifier, true).unwrap();
                    let value_type = variable.borrow().value.borrow().value_type;

                    if value_type.is_array() {
                        return Err(parse_error(
                            "Cannot modify an array.",
                            &self.source,
                            node.node.start,
                        ));
                    }
                    if variable.borrow().constant {
                        return Err(parse_error(
                            "Cannot modify a constant variable.",
                            &self.source,
                            node.node.start,
                        ));
                    }

                    if node.reinforcement_type != VariableType::Unknown
                        && value_type != node.reinforcement_type
                    {
                        return Err(parse_error(
                            &format!(
                                "Got reinforcement type '{}' when expecting type '{}'",
                                node.reinforcement_type, value_type
                            ),
                            &self.source,
                            node.value.to_node().start,
                        ));
                    }

                    let mut value = self.evaluate_value_node(&node.value, true)?;

                    if value.value_type == VariableType::Unknown {
                        let default = value_type.get_default_value().unwrap();
                        value = DynamicVariable::from_value_type(&default, value_type);
                    }

                    if value_type != value.value_type
                        && (value_type != VariableType::String && !value.value_type.is_array())
                    {
                        return Err(parse_error(
                            &format!(
                                "Expected type '{}', got '{}'.",
                                value_type, value.value_type
                            ),
                            &self.source,
                            node.node.start,
                        ));
                    }

                    let value_ref = variable.borrow().value.clone();
                    let mut target = value_ref.borrow_mut();
                    match value_type {
                        VariableType::String => target.set_value_string(value.get_value_string()),
                        VariableType::Character => {
                            target.set_value_character(value.get_value_character().to_string())
                        }
                        VariableType::Boolean => {
                            target.set_value_boolean(value.get_value_boolean())
                        }
                        VariableType::Number => target.set_value_number(value.get_value_number()),
                        _ => {}
                    }
                }

                DynamicNode::ArrayModify(node) => {
                    if !self.variables.has(&node.identifier, true) {
                        return Err(parse_error(
                            &format!("Variable '{}' does not exist.", node.identifier),
                            &self.source,
                            node.node.start,
                        ));
                    }
                    let variable = self.variables.get(&node.identifier, true).unwrap();
                    let value_type = variable.borrow().value.borrow().value_type;

                    if !value_type.is_array() {
                        return Err(parse_error(
                            "Invalid non-array variable.",
                            &self.source,
                            node.node.start,
                        ));
                    }

                    if node.reinforcement_type != VariableType::Unknown
                        && value_type.as_base_type() != node.reinforcement_type
                    {
                        return Err(parse_error(
                            &format!(
                                "Got reinforcement type '{}' when expecting type '{}'",
                                node.reinforcement_type, value_type
                            ),
                            &self.source,
                            node.value.to_node().start,
                        ));
                    }

                    let index = self.evaluate_value_node(&node.index, true)?;
                    if index.value_type != VariableType::Number {
                        return Err(parse_error(
                            &format!("Expected a numeric index, got type {}", index.value_type),
                            &self.source,
                            node.index.to_node().start,
                        ));
                    }

                    let mut value = self.evaluate_value_node(&node.value, true)?;
                    let base_type = value_type.as_base_type();

                    if value.value_type == VariableType::Unknown {
                        let default = base_type.get_default_value().unwrap();
                        value = DynamicVariable::from_value_type(&default, base_type);
                    }
                    if value.value_type.is_array() {
                        return Err(parse_error(
                            "Cannot insert an array value",
                            &self.source,
                            node.value.to_node().start,
                        ));
                    }

                    if base_type != value.value_type {
                        return Err(parse_error(
                            &format!("Expected type '{}', got '{}'.", base_type, value.value_type),
                            &self.source,
                            node.node.start,
                        ));
                    }

                    variable
                        .borrow()
                        .value
                        .borrow()
                        .dictionary_set(index.get_value_number() as usize, value);
                }

                DynamicNode::IfStatement(node) => {
                    for branch in &node.conditions {
                        let mut check = true;

                        if let Some(condition) = &branch.condition {
                            let branch_check = self.evaluate_value_node(condition, true)?;
                            if branch_check.value_type != VariableType::Boolean {
                                return Err(parse_error(
                                    &format!(
                                        "Expected condition to result in type {}, got {}",
                                        VariableType::Boolean,
                                        branch_check.value_type
                                    ),
                                    &self.source,
                                    branch.statements.node.start,
                                ));
                            }
                            check = branch_check.get_value_boolean();
                        }

                        if check {
                            if let Some(result) =
                                self.evaluate_statements_node(&branch.statements)?
                            {
                                return Ok(Some(result));
                            }
                            break;
                        }
                    }
                }

                DynamicNode::WhileStatement(node) => loop {
                    let condition = node.condition_stmt.condition.as_ref().unwrap();
                    let branch_check = self.evaluate_value_node(condition, true)?;

                    if branch_check.value_type != VariableType::Boolean {
                        return Err(parse_error(
                            &format!(
                                "Expected condition to result in type {}, got {}",
                                VariableType::Boolean,
                                branch_check.value_type
                            ),
                            &self.source,
                            node.condition_stmt.statements.node.start,
                        ));
                    }

                    if !branch_check.get_value_boolean() {
                        break;
                    }

                    if let Some(result) =
                        self.evaluate_statements_node(&node.condition_stmt.statements)?
                    {
                        return Ok(Some(result));
                    }
                },

                DynamicNode::ForEveryArray(node) => {
                    if !self.variables.has(&node.identifier, true) {
                        return Err(parse_error(
                            &format!("Variable '{}' does not exist.", node.identifier),
                            &self.source,
                            node.base.statements.node.start,
                        ));
                    }
                    let variable = self.variables.get(&node.identifier, true).unwrap();
                    let value_type = variable.borrow().value.borrow().value_type;

                    if value_type.is_array() {
                        if value_type.as_base_type() != node.base.variable_type {
                            return Err(parse_error(
                                &format!(
                                    "Expected loop variable to be type {}, got {}",
                                    value_type.as_base_type(),
                                    node.base.variable_type
                                ),
                                &self.source,
                                node.base.statements.node.start,
                            ));
                        }
                    } else if value_type == VariableType::String {
                        if node.base.variable_type != VariableType::Character {
                            return Err(parse_error(
                                &format!(
                                    "Expected loop variable to be type {}, got {}",
                                    VariableType::Character,
                                    node.base.variable_type
                                ),
                                &self.source,
                                node.base.statements.node.start,
                            ));
                        }
                    } else {
                        return Err(parse_error(
                            &format!("Expected an array variable, got type {}", value_type),
                            &self.source,
                            node.base.statements.node.start,
                        ));
                    }

                    if self.variables.has(&node.base.variable_name, true) {
                        return Err(parse_error(
                            &format!("Variable '{}' already exists.", node.base.variable_name),
                            &self.source,
                            node.base.statements.node.start,
                        ));
                    }

                    if value_type == VariableType::String {
                        let string = variable.borrow().value.borrow().get_value_string();
                        for character in string.chars() {
                            self.variables.push_variable(
                                Variable::new(
                                    node.base.variable_name.clone(),
                                    DynamicVariable::new_raw_character(character.to_string()),
                                    true,
                                ),
                                false,
                            );

                            let result = self.evaluate_statements_node(&node.base.statements);
                            self.variables.pop_variable(false);

                            match result? {
                                Some(value) => return Ok(Some(value)),
                                None => {}
                            }
                        }
                    } else {
                        let keys = variable.borrow().value.borrow().dictionary_keys();

                        for key in keys {
                            let element = variable
                                .borrow()
                                .value
                                .borrow()
                                .dictionary_get(key)
                                .unwrap();

                            let value = match value_type {
                                VariableType::StringArray => {
                                    DynamicVariable::new_raw_string(element.get_value_string())
                                }
                                VariableType::BooleanArray => {
                                    DynamicVariable::new_boolean(element.get_value_boolean())
                                }
                                VariableType::NumberArray => {
                                    DynamicVariable::new_number(element.get_value_number())
                                }
                                _ => unreachable!(),
                            };

                            self.variables.push_variable(
                                Variable::new(node.base.variable_name.clone(), value, true),
                                false,
                            );

                            let result = self.evaluate_statements_node(&node.base.statements);
                            self.variables.pop_variable(false);

                            match result? {
                                Some(value) => return Ok(Some(value)),
                                None => {}
                            }
                        }
                    }
                }

                DynamicNode::ForEveryRange(node) => {
                    if self.variables.has(&node.base.variable_name, true) {
                        return Err(parse_error(
                            &format!("Variable '{}' already exists.", node.base.variable_name),
                            &self.source,
                            node.base.statements.node.start,
                        ));
                    }

                    let from_range = self.evaluate_value_node(&node.range_start, true)?;
                    if from_range.value_type != VariableType::Number {
                        return Err(parse_error(
                            &format!("Expected a number type, got {}", from_range.value_type),
                            &self.source,
                            node.range_start.to_node().start,
                        ));
                    }

                    let to_range = self.evaluate_value_node(&node.range_end, true)?;
                    if to_range.value_type != VariableType::Number {
                        return Err(parse_error(
                            &format!("Expected a number type, got {}", to_range.value_type),
                            &self.source,
                            node.range_end.to_node().start,
                        ));
                    }

                    let start_value = from_range.get_value_number();
                    let end_value = to_range.get_value_number();

                    let mut current_value = start_value;
                    let is_forwards = end_value >= start_value;

                    loop {
                        if is_forwards && current_value > end_value {
                            break;
                        } else if !is_forwards && current_value < end_value {
                            break;
                        }

                        self.variables.push_variable(
                            Variable::new(
                                node.base.variable_name.clone(),
                                DynamicVariable::new_number(current_value),
                                true,
                            ),
                            false,
                        );

                        let result = self.evaluate_statements_node(&node.base.statements);
                        self.variables.pop_variable(false);

                        match result? {
                            Some(value) => return Ok(Some(value)),
                            None => {}
                        }

                        if is_forwards {
                            current_value += 1.0;
                        } else {
                            current_value -= 1.0;
                        }
                    }
                }

                DynamicNode::UnaryExpression(node) => match &node.identifier {
                    DynamicNode::Identifier(identifier) => {
                        if !self.variables.has(&identifier.identifier, true) {
                            return Err(parse_error(
                                &format!("Variable '{}' does not exist.", identifier.identifier),
                                &self.source,
                                node.node.start,
                            ));
                        }
                        let variable = self.variables.get(&identifier.identifier, true).unwrap();
                        let value_type = variable.borrow().value.borrow().value_type;

                        if value_type != VariableType::Number {
                            return Err(parse_error(
                                &format!("Expected a number type, got {}.", value_type),
                                &self.source,
                                node.node.start,
                            ));
                        }
                        if variable.borrow().constant {
                            return Err(parse_error(
                                "Cannot modify a constant variable.",
                                &self.source,
                                node.node.start,
                            ));
                        }

                        let value_ref = variable.borrow().value.clone();
                        let mut target = value_ref.borrow_mut();
                        let current = target.get_value_number();
                        target.set_value_number(if node.increment {
                            current + 1.0
                        } else {
                            current - 1.0
                        });
                    }

                    DynamicNode::DictionaryIdentifier(identifier) => {
                        if !self.variables.has(&identifier.identifier, true) {
                            return Err(parse_error(
                                &format!("Variable '{}' does not exist.", identifier.identifier),
                                &self.source,
                                node.node.start,
                            ));
                        }
                        let variable = self.variables.get(&identifier.identifier, true).unwrap();
                        let value_type = variable.borrow().value.borrow().value_type;

                        if value_type != VariableType::NumberArray {
                            return Err(parse_error(
                                &format!(
                                    "Expected a number array type for identifier, got {}.",
                                    value_type
                                ),
                                &self.source,
                                node.node.start,
                            ));
                        }

                        let index = self.evaluate_value_node(&identifier.index, true)?;
                        if index.value_type != VariableType::Number {
                            return Err(parse_error(
                                &format!(
                                    "Expected a number type for index, got {}.",
                                    index.value_type
                                ),
                                &self.source,
                                node.node.start,
                            ));
                        }

                        let key = index.get_value_number() as usize;
                        let mut value = variable
                            .borrow()
                            .value
                            .borrow()
                            .dictionary_get(key)
                            .unwrap_or_else(|| DynamicVariable::new_number(0.0));

                        let current = value.get_value_number();
                        value.set_value_number(if node.increment {
                            current + 1.0
                        } else {
                            current - 1.0
                        });

                        variable.borrow().value.borrow().dictionary_set(key, value);
                    }

                    _ => {}
                },

                DynamicNode::FunctionCall(call) => {
                    let index = self
                        .paragraphs
                        .iter()
                        .position(|p| p.name == call.identifier)
                        .ok_or_else(|| {
                            parse_error(
                                &format!("Paragraph '{}' not found", call.identifier),
                                &self.source,
                                statement.to_node().start,
                            )
                        })?;

                    let mut parameters = Vec::new();
                    for parameter in &call.parameters {
                        parameters.push(self.evaluate_value_node(parameter, true)?);
                    }

                    self.execute_paragraph_index(index, parameters)?;
                }

                DynamicNode::FunctionReturn(node) => {
                    let value = self.evaluate_value_node(&node.value, true)?;
                    return Ok(Some(value));
                }

                _ => {
                    return Err(parse_error(
                        "Unsupported statement node.",
                        &self.source,
                        statement.to_node().start,
                    ));
                }
            }
        }

        Ok(None)
    }

    // --- Ejecución de párrafos (celestia/paragraph.go) ---

    fn execute_paragraph_index(
        &mut self,
        index: usize,
        parameters: Vec<DynamicVariable>,
    ) -> Result<Option<DynamicVariable>, String> {
        let node = self.paragraphs[index].node.clone();

        self.variables.push_scope();

        for (idx, expecting) in node.parameters.iter().enumerate() {
            if idx < parameters.len() {
                let received = &parameters[idx];

                if received.value_type != expecting.variable_type {
                    return Err(parse_error(
                        &format!(
                            "Expecting parameter type {}, got {}",
                            expecting.variable_type, received.value_type
                        ),
                        &self.source,
                        node.node.start,
                    ));
                }

                self.variables.push_variable(
                    Variable::new(expecting.name.clone(), received.clone(), false),
                    false,
                );
            } else {
                let Some(default) = expecting.variable_type.get_default_value() else {
                    return Err(parse_error(
                        &format!(
                            "Could not get default value of {} (type {})",
                            expecting.name, expecting.variable_type
                        ),
                        &self.source,
                        node.node.start,
                    ));
                };

                self.variables.push_variable(
                    Variable::new(
                        expecting.name.clone(),
                        DynamicVariable::from_value_type(&default, expecting.variable_type),
                        false,
                    ),
                    false,
                );
            }
        }

        // El original empuja los parámetros recibidos una segunda vez (bug
        // inofensivo por cómo funciona `Get`, pero se replica por fidelidad).
        if !node.parameters.is_empty() {
            for i in 0..min(node.parameters.len(), parameters.len()) {
                let expecting = &node.parameters[i];
                let received = &parameters[i];

                if received.value_type != expecting.variable_type {
                    return Err(parse_error(
                        &format!(
                            "Expecting parameter type {}, got {}",
                            expecting.variable_type, received.value_type
                        ),
                        &self.source,
                        node.node.start,
                    ));
                }

                self.variables.push_variable(
                    Variable::new(expecting.name.clone(), received.clone(), false),
                    false,
                );
            }
        }

        let value = self.evaluate_statements_node(&node.body);
        self.variables.pop_scope();

        let value = value?;

        if let Some(returned) = &value {
            if returned.value_type != VariableType::Unknown {
                if node.return_type == VariableType::Unknown {
                    return Err(parse_error(
                        &format!(
                            "Paragraph '{}' with no return type returned a value",
                            node.name
                        ),
                        &self.source,
                        node.node.start,
                    ));
                }
                if returned.value_type != node.return_type {
                    return Err(parse_error(
                        &format!(
                            "Paragraph '{}' expected return value of type '{}', received '{}'",
                            node.name, node.return_type, returned.value_type
                        ),
                        &self.source,
                        node.node.start,
                    ));
                }
            }
        }

        Ok(value)
    }
}

#[cfg(test)]
#[path = "interpreter_tests.rs"]
mod tests;
