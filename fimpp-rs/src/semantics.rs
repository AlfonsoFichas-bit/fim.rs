// =====================================================================
// SEMANTIC ANALYSIS — primera etapa del paso "intérprete -> compilador".
//
// Recorre el `ReportNode` que produce `ast.rs` y valida:
//   * párrafos/ variables globales duplicados,
//   * resolución de identificadores (variables, párrafos),
//   * aridad y tipos de llamadas,
//   * tipos en expresiones y condiciones,
//   * asignaciones / modificaciones coherentes con el tipo declarado,
//   * valor de retorno de cada párrafo.
//
// Devuelve el primer error semántico encontrado (o `Ok(())` si todo es válido).
// El intérprete ya hace algunas de estas comprobaciones en tiempo de ejecución;
// este pase las hace estáticamente, que es lo que un compilador necesita.
// =====================================================================

use std::collections::HashMap;

use crate::ast::*;
use crate::variable::VariableType;

// ----------------------------------------------------------------------------
// Signatura de un párrafo.
// ----------------------------------------------------------------------------
#[derive(Clone, Debug)]
struct ParagraphSignature {
    name: String,
    parameters: Vec<(String, VariableType)>,
    return_type: VariableType,
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub message: String,
    pub index: usize,
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for SemanticError {}

// ----------------------------------------------------------------------------
// Ámbito (scope) apilable.
// ----------------------------------------------------------------------------
struct Scope {
    vars: HashMap<String, VariableType>,
}
impl Scope {
    fn new() -> Self {
        Self { vars: HashMap::new() }
    }
    fn declare(&mut self, name: String, value_type: VariableType) {
        self.vars.insert(name, value_type);
    }
    fn lookup(&self, name: &str) -> Option<VariableType> {
        self.vars.get(name).copied()
    }
}

pub struct SemanticAnalyzer {
    paragraphs: HashMap<String, ParagraphSignature>,
    globals: HashMap<String, VariableType>,
    scopes: Vec<Scope>,
    source: String,
}

impl SemanticAnalyzer {
    pub fn new(report: &ReportNode, source: &str) -> Result<Self, SemanticError> {
        let mut analyzer = SemanticAnalyzer {
            paragraphs: HashMap::new(),
            globals: HashMap::new(),
            scopes: vec![Scope::new()],
            source: source.to_string(),
        };

        // ---------- Primera pasada: registrar párrafos y globales.
        for node in &report.body {
            match node {
                DynamicNode::Function(function) => {
                    if analyzer.paragraphs.contains_key(&function.name) {
                        return Err(SemanticError {
                            message: format!("Paragraph '{}' already exists", function.name),
                            index: function.node.start,
                        });
                    }
                    let signature = ParagraphSignature {
                        name: function.name.clone(),
                        parameters: function
                            .parameters
                            .iter()
                            .map(|p| (p.name.clone(), p.variable_type))
                            .collect(),
                        return_type: function.return_type,
                    };
                    analyzer.paragraphs.insert(function.name.clone(), signature);
                }
                DynamicNode::VariableDeclaration(declaration) => {
                    if analyzer.globals.contains_key(&declaration.identifier)
                        || analyzer.lookup_local(&declaration.identifier) {
                        return Err(SemanticError {
                            message: format!("Variable '{}' already exists.", declaration.identifier),
                            index: declaration.node.start,
                        });
                    }
                    // Validar el valor inicial.
                    let value_type = analyzer.type_of_value(&declaration.value)?;
                    // Compatibilidad con el intérprete: si es Unknown, se resuelve
                    // por el tipo declarado. Verificamos las combinaciones que
                    // el intérprete rechaza.
                    let mut resolved = value_type;
                    if resolved == VariableType::Unknown {
                        resolved = declaration.value_type;
                    }
                    if declaration.value_type != resolved
                        && (declaration.value_type != VariableType::String
                            || resolved.is_array())
                    {
                        return Err(SemanticError {
                            message: format!(
                                "Expected type '{}', got '{}'",
                                declaration.value_type, value_type
                            ),
                            index: declaration.node.start,
                        });
                    }

                    analyzer.scopes[0]
                        .vars
                        .insert(declaration.identifier.clone(), declaration.value_type);
                    analyzer.globals.insert(declaration.identifier.clone(), declaration.value_type);
                }
                _ => {
                    return Err(SemanticError {
                        message: "Unsupported report body node".to_string(),
                        index: node.to_node().start,
                    });
                }
            }
        }

        // ---------- Segunda pasada: validar cuerpos de los párrafos.
        for node in &report.body {
            if let DynamicNode::Function(function) = node {
                analyzer.validate_paragraph(function)?;
            }
        }

        Ok(analyzer)
    }

    fn lookup_local(&self, name: &str) -> bool {
        for scope in &self.scopes {
            if scope.lookup(name).is_some() {
                return true;
            }
        }
        false
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    fn define(&mut self, name: String, value_type: VariableType, index: usize) -> Result<(), SemanticError> {
        // Rechazar redeclaración en el mismo ámbito actual.
        let current = self.scopes.last_mut().unwrap();
        if current.lookup(&name).is_some() {
            return Err(SemanticError {
                message: format!("Variable '{}' already exists.", name),
                index,
            });
        }
        current.declare(name, value_type);
        Ok(())
    }
    fn lookup(&self, name: &str) -> Option<VariableType> {
        for scope in self.scopes.iter().rev() {
            if let Some(value_type) = scope.lookup(name) {
                return Some(value_type);
            }
        }
        // Globales (siempre disponibles).
        self.globals.get(name).copied()
    }

    fn validate_paragraph(&mut self, function: &FunctionNode) -> Result<(), SemanticError> {
        self.push_scope();

        // Parámetros.
        for parameter in &function.parameters {
            self.define(
                parameter.name.clone(),
                parameter.variable_type,
                function.node.start,
            )?;
        }

        // Segunda push para imitar... en realidad un solo ámbito; reutilizamos.
        // Validar cuerpo.
        let returned = self.validate_statements(&function.body)?;
        if let Some(returned_type) = returned {
            if returned_type != VariableType::Unknown
                && function.return_type == VariableType::Unknown
            {
                return Err(SemanticError {
                    message: format!(
                        "Paragraph '{}' with no return type returned a value",
                        function.name
                    ),
                    index: function.node.start,
                });
            }
            if returned_type != VariableType::Unknown
                && function.return_type != VariableType::Unknown
                && returned_type != function.return_type
            {
                return Err(SemanticError {
                    message: format!(
                        "Paragraph '{}' expected return value of type '{}', received '{}'",
                        function.name, function.return_type, returned_type
                    ),
                    index: function.node.start,
                });
            }
        }

        self.pop_scope();
        Ok(())
    }

    // Devuelve Some(tipo) si el bloque termina con `Then you get ...`.
    fn validate_statements(
        &mut self,
        statements: &StatementsNode,
    ) -> Result<Option<VariableType>, SemanticError> {
        for statement in &statements.statements {
            match statement {
                DynamicNode::Print(node) => {
                    let value_type = self.type_of_value(&node.value)?;
                    if value_type.is_array() {
                        return Err(SemanticError {
                            message: "Cannot print an array value".to_string(),
                            index: node.node.start,
                        });
                    }
                }
                DynamicNode::Prompt(node) => {
                    // Variable existente, no constante ni array.
                    let value_type = self.lookup(&node.identifier).ok_or_else(|| SemanticError {
                        message: format!("Variable '{}' does not exist.", node.identifier),
                        index: node.node.start,
                    })?;
                    if value_type.is_array() {
                        return Err(SemanticError {
                            message: "Expected variable to be of non-array type".to_string(),
                            index: node.node.start,
                        });
                    }
                    // prompt expr debe ser texto
                    let prompt_type = self.type_of_value(&node.prompt)?;
                    if prompt_type != VariableType::String {
                        return Err(SemanticError {
                            message: "Expected prompt to be of type STRING".to_string(),
                            index: node.node.start,
                        });
                    }
                }
                DynamicNode::VariableDeclaration(node) => {
                    // Si ya existe en este ámbito → error.
                    // (La redeclaración de la misma variable es ilegal.)
                    let value_type = self.type_of_value(&node.value)?;
                    let mut resolved = value_type;
                    if resolved == VariableType::Unknown {
                        resolved = node.value_type;
                    }
                    if node.value_type != resolved
                        && (node.value_type != VariableType::String
                            || resolved.is_array())
                    {
                        return Err(SemanticError {
                            message: format!(
                                "Expected type '{}', got '{}'",
                                node.value_type, value_type
                            ),
                            index: node.node.start,
                        });
                    }
                    self.define(
                        node.identifier.clone(),
                        node.value_type,
                        node.node.start,
                    )?;
                }
                DynamicNode::VariableModify(node) => {
                    let value_type = self
                        .lookup(&node.identifier)
                        .ok_or_else(|| SemanticError {
                            message: format!("Variable '{}' does not exist.", node.identifier),
                            index: node.node.start,
                        })?;
                    if value_type.is_array() {
                        return Err(SemanticError {
                            message: "Cannot modify an array.".to_string(),
                            index: node.node.start,
                        });
                    }
                    // reinforcement type check.
                    if node.reinforcement_type != VariableType::Unknown
                        && value_type != node.reinforcement_type
                    {
                        return Err(SemanticError {
                            message: format!(
                                "Got reinforcement type '{}' when expecting type '{}'",
                                node.reinforcement_type, value_type
                            ),
                            index: node.node.start,
                        });
                    }
                    let value = self.type_of_value(&node.value)?;
                    let mut resolved = value;
                    if resolved == VariableType::Unknown {
                        resolved = value_type;
                    }
                    if resolved != value_type
                        && (value_type != VariableType::String
                            || resolved.is_array())
                    {
                        return Err(SemanticError {
                            message: format!(
                                "Expected type '{}', got '{}'.",
                                value_type, value
                            ),
                            index: node.node.start,
                        });
                    }
                }
                DynamicNode::ArrayModify(node) => {
                    let value_type = self.lookup(&node.identifier).ok_or_else(|| SemanticError {
                        message: format!("Variable '{}' does not exist.", node.identifier),
                        index: node.node.start,
                    })?;
                    if !value_type.is_array() {
                        return Err(SemanticError {
                            message: "Invalid non-array variable.".to_string(),
                            index: node.node.start,
                        });
                    }
                    if node.reinforcement_type != VariableType::Unknown
                        && value_type.as_base_type() != node.reinforcement_type
                    {
                        return Err(SemanticError {
                            message: format!(
                                "Got reinforcement type '{}' when expecting type '{}'",
                                node.reinforcement_type, value_type
                            ),
                            index: node.node.start,
                        });
                    }
                    let base_type = value_type.as_base_type();
                    let index_type = self.type_of_value(&node.index)?;
                    if index_type != VariableType::Number {
                        return Err(SemanticError {
                            message: format!(
                                "Expected a numeric index, got type {}",
                                index_type
                            ),
                            index: node.node.start,
                        });
                    }
                    let value = self.type_of_value(&node.value)?;
                    let mut resolved = value;
                    if resolved == VariableType::Unknown {
                        resolved = base_type;
                    }
                    if resolved.is_array() {
                        return Err(SemanticError {
                            message: "Cannot insert an array value".to_string(),
                            index: node.node.start,
                        });
                    }
                    if base_type != resolved {
                        return Err(SemanticError {
                            message: format!(
                                "Expected type '{}', got '{}'.",
                                base_type, value
                            ),
                            index: node.node.start,
                        });
                    }
                }
                DynamicNode::IfStatement(node) => {
                    for branch in &node.conditions {
                        if let Some(condition) = &branch.condition {
                            let branch_type = self.type_of_value(condition)?;
                            if branch_type != VariableType::Boolean {
                                return Err(SemanticError {
                                    message: format!(
                                        "Expected condition to result in type {}, got {}",
                                        VariableType::Boolean, branch_type
                                    ),
                                    index: node.node.start,
                                });
                            }
                        }
                        self.push_scope();
                        match self.validate_statements(&branch.statements)? {
                            Some(returned) => return Ok(Some(returned)),
                            None => {}
                        }
                        self.pop_scope();
                        // Go: después de una rama tomada se hace `break`.
                        // En un análisis estático recorremos todas las ramas.
                    }
                    // Reempujamos en caso de haber salido antes.
                }
                DynamicNode::WhileStatement(node) => {
                    // La condición puede ser un statements-node con condición.
                    let condition = node
                        .condition_stmt
                        .condition
                        .as_ref()
                        .ok_or_else(|| SemanticError {
                            message: "Missing while condition".to_string(),
                            index: node.condition_stmt.statements.node.start,
                        })?;
                    let condition_type = self.type_of_value(condition)?;
                    if condition_type != VariableType::Boolean {
                        return Err(SemanticError {
                            message: format!(
                                "Expected condition to result in type {}, got {}",
                                VariableType::Boolean, condition_type
                            ),
                            index: node.condition_stmt.statements.node.start,
                        });
                    }
                    self.push_scope();
                    match self.validate_statements(&node.condition_stmt.statements)? {
                        Some(returned) => return Ok(Some(returned)),
                        None => {}
                    }
                    self.pop_scope();
                }
                DynamicNode::ForEveryArray(node) => {
                    let value_type = self.lookup(&node.identifier).ok_or_else(|| SemanticError {
                        message: format!("Variable '{}' does not exist.", node.identifier),
                        index: node.base.statements.node.start,
                    })?;
                    if value_type.is_array() {
                        if value_type.as_base_type() != node.base.variable_type {
                            return Err(SemanticError {
                                message: format!(
                                    "Expected loop variable to be type {}, got {}",
                                    value_type.as_base_type(),
                                    node.base.variable_type
                                ),
                                index: node.base.statements.node.start,
                            });
                        }
                    } else if value_type == VariableType::String {
                        if node.base.variable_type != VariableType::Character {
                            return Err(SemanticError {
                                message: format!(
                                    "Expected loop variable to be type {}, got {}",
                                    VariableType::Character,
                                    node.base.variable_type
                                ),
                                index: node.base.statements.node.start,
                            });
                        }
                    } else {
                        return Err(SemanticError {
                            message: format!(
                                "Expected an array variable, got type {}",
                                value_type
                            ),
                            index: node.base.statements.node.start,
                        });
                    }
                    if self.lookup(&node.base.variable_name).is_some() {
                        return Err(SemanticError {
                            message: format!(
                                "Variable '{}' already exists.",
                                node.base.variable_name
                            ),
                            index: node.base.statements.node.start,
                        });
                    }
                    // Nuevo ámbito con la variable de iteración.
                    self.push_scope();
                    self.define(
                        node.base.variable_name.clone(),
                        node.base.variable_type,
                        node.base.statements.node.start,
                    )?;
                    match self.validate_statements(&node.base.statements)? {
                        Some(returned) => {
                            self.pop_scope();
                            return Ok(Some(returned));
                        }
                        None => {}
                    }
                    self.pop_scope();
                }
                DynamicNode::ForEveryRange(node) => {
                    if self.lookup(&node.base.variable_name).is_some() {
                        return Err(SemanticError {
                            message: format!(
                                "Variable '{}' already exists.",
                                node.base.variable_name
                            ),
                            index: node.base.statements.node.start,
                        });
                    }
                    let from_type = self.type_of_value(&node.range_start)?;
                    if from_type != VariableType::Number {
                        return Err(SemanticError {
                            message: format!("Expected a number type, got {}", from_type),
                            index: node.base.statements.node.start,
                        });
                    }
                    let to_type = self.type_of_value(&node.range_end)?;
                    if to_type != VariableType::Number {
                        return Err(SemanticError {
                            message: format!("Expected a number type, got {}", to_type),
                            index: node.base.statements.node.start,
                        });
                    }
                    if node.base.variable_type != VariableType::Number {
                        return Err(SemanticError {
                            message: format!(
                                "Expected loop variable to be a number, got {}",
                                node.base.variable_type
                            ),
                            index: node.base.statements.node.start,
                        });
                    }
                    self.push_scope();
                    self.define(
                        node.base.variable_name.clone(),
                        VariableType::Number,
                        node.base.statements.node.start,
                    )?;
                    match self.validate_statements(&node.base.statements)? {
                        Some(returned) => {
                            self.pop_scope();
                            return Ok(Some(returned));
                        }
                        None => {}
                    }
                    self.pop_scope();
                }
                DynamicNode::UnaryExpression(node) => {
                    match &node.identifier {
                        DynamicNode::Identifier(identifier) => {
                            let value_type = self
                                .lookup(&identifier.identifier)
                                .ok_or_else(|| SemanticError {
                                    message: format!(
                                        "Variable '{}' does not exist.",
                                        identifier.identifier
                                    ),
                                    index: node.node.start,
                                })?;
                            if value_type != VariableType::Number {
                                return Err(SemanticError {
                                    message: format!(
                                        "Expected a number type, got {}.",
                                        value_type
                                    ),
                                    index: node.node.start,
                                });
                            }
                        }
                        DynamicNode::DictionaryIdentifier(identifier) => {
                            let value_type =
                                self.lookup(&identifier.identifier).ok_or_else(|| SemanticError {
                                    message: format!(
                                        "Variable '{}' does not exist.",
                                        identifier.identifier
                                    ),
                                    index: node.node.start,
                                })?;
                            if value_type != VariableType::NumberArray {
                                return Err(SemanticError {
                                    message: format!(
                                        "Expected a number array type for identifier, got {}.",
                                        value_type
                                    ),
                                    index: node.node.start,
                                });
                            }
                            let index_type = self.type_of_value(&identifier.index)?;
                            if index_type != VariableType::Number {
                                return Err(SemanticError {
                                    message: format!(
                                        "Expected a number type for index, got {}.",
                                        index_type
                                    ),
                                    index: node.node.start,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                DynamicNode::FunctionCall(call) => {
                    let signature = self
                        .paragraphs
                        .get(&call.identifier)
                        .ok_or_else(|| SemanticError {
                            message: format!("Paragraph '{}' not found", call.identifier),
                            index: statement.to_node().start,
                        })?
                        .clone();
                    if call.parameters.len() > signature.parameters.len() {
                        return Err(SemanticError {
                            message: format!(
                                "Expected {} parameters, got {}",
                                signature.parameters.len(),
                                call.parameters.len()
                            ),
                            index: statement.to_node().start,
                        });
                    }
                    for (parameter, (name, expected_type)) in
                        call.parameters.iter().zip(signature.parameters.iter())
                    {
                        let actual_type = self.type_of_value(parameter)?;
                        if actual_type != *expected_type {
                            return Err(SemanticError {
                                message: format!(
                                    "Expecting parameter type {}, got {} ({})",
                                    expected_type, actual_type, name
                                ),
                                index: statement.to_node().start,
                            });
                        }
                    }
                }
                DynamicNode::FunctionReturn(node) => {
                    let value_type = self.type_of_value(&node.value)?;
                    return Ok(Some(value_type));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn type_of_value(&self, node: &DynamicNode) -> Result<VariableType, SemanticError> {
        match node {
            DynamicNode::Literal(literal) => Ok(literal.variable.value_type),
            DynamicNode::LiteralDictionary(dictionary) => Ok(dictionary.array_type),
            DynamicNode::Identifier(identifier) => {
                // Variable o párrafo (sin argumentos) que devuelve un valor.
                if let Some(value_type) = self.lookup(&identifier.identifier) {
                    return Ok(value_type);
                }
                if let Some(signature) = self.paragraphs.get(&identifier.identifier) {
                    if signature.parameters.is_empty() {
                        return Ok(signature.return_type);
                    }
                }
                Err(SemanticError {
                    message: format!("Unknown identifier ({})", identifier.identifier),
                    index: identifier.node.start,
                })
            }
            DynamicNode::FunctionCall(call) => {
                let signature = self
                    .paragraphs
                    .get(&call.identifier)
                    .ok_or_else(|| SemanticError {
                        message: format!("Unknown paragraph ({})", call.identifier),
                        index: call.node.start,
                    })?
                    .clone();
                if signature.return_type == VariableType::Unknown {
                    return Err(SemanticError {
                        message: "Tried calling a function that doesn't return a value".to_string(),
                        index: call.node.start,
                    });
                }
                if call.parameters.len() > signature.parameters.len() {
                    return Err(SemanticError {
                        message: format!(
                            "Expected {} parameters, got {}",
                            signature.parameters.len(),
                            call.parameters.len()
                        ),
                        index: call.node.start,
                    });
                }
                for (parameter, (_name, expected_type)) in
                    call.parameters.iter().zip(signature.parameters.iter())
                {
                    let actual_type = self.type_of_value(parameter)?;
                    if actual_type != *expected_type {
                        return Err(SemanticError {
                            message: format!(
                                "Expecting parameter type {}, got {}",
                                expected_type, actual_type
                            ),
                            index: call.node.start,
                        });
                    }
                }
                Ok(signature.return_type)
            }
            DynamicNode::DictionaryIdentifier(identifier) => {
                let value_type = self.lookup(&identifier.identifier).ok_or_else(|| {
                    SemanticError {
                        message: format!("Unknown identifier ({})", identifier.identifier),
                        index: identifier.node.start,
                    }
                })?;
                if !value_type.is_array() && value_type != VariableType::String {
                    return Err(SemanticError {
                        message: format!(
                            "Invalid non-dicionary identifier ({})",
                            identifier.identifier
                        ),
                        index: identifier.node.start,
                    });
                }
                let index_type = self.type_of_value(&identifier.index)?;
                if index_type != VariableType::Number {
                    return Err(SemanticError {
                        message: format!("Expected numeric index, got type {}", index_type),
                        index: identifier.index.to_node().start,
                    });
                }
                if value_type == VariableType::String {
                    Ok(VariableType::Character)
                } else {
                    Ok(value_type.as_base_type())
                }
            }
            DynamicNode::BinaryExpression(expression) => {
                let left = self.type_of_value(&expression.left)?;
                let right = self.type_of_value(&expression.right)?;
                match expression.operator {
                    BinaryExpressionOperator::Add => {
                        if left == VariableType::String || right == VariableType::String {
                            Ok(VariableType::String)
                        } else if left == VariableType::Number
                            && right == VariableType::Number
                        {
                            Ok(VariableType::Number)
                        } else if left == VariableType::Unknown
                            || right == VariableType::Unknown
                        {
                            // Se resuelve por conversión en tiempo de ejecución.
                            Ok(VariableType::Unknown)
                        } else {
                            Err(SemanticError {
                                message: format!(
                                    "Cannot add {:?} and {:?}",
                                    left, right
                                ),
                                index: expression.node.start,
                            })
                        }
                    }
                    BinaryExpressionOperator::Sub
                    | BinaryExpressionOperator::Mul
                    | BinaryExpressionOperator::Div
                    | BinaryExpressionOperator::Mod => {
                        if left != VariableType::Number || right != VariableType::Number {
                            Err(SemanticError {
                                message: format!(
                                    "Expected NUMBER operands, got {:?} and {:?}",
                                    left, right
                                ),
                                index: expression.node.start,
                            })
                        } else {
                            Ok(VariableType::Number)
                        }
                    }
                    BinaryExpressionOperator::And | BinaryExpressionOperator::Or => {
                        if left != VariableType::Boolean || right != VariableType::Boolean {
                            Err(SemanticError {
                                message: format!(
                                    "Expected BOOLEAN operands, got {:?} and {:?}",
                                    left, right
                                ),
                                index: expression.node.start,
                            })
                        } else {
                            Ok(VariableType::Boolean)
                        }
                    }
                    BinaryExpressionOperator::Gte
                    | BinaryExpressionOperator::Lte
                    | BinaryExpressionOperator::Gt
                    | BinaryExpressionOperator::Lt => {
                        // El intérprete compara números.
                        Ok(VariableType::Boolean)
                    }
                    BinaryExpressionOperator::Eq | BinaryExpressionOperator::Neq => {
                        Ok(VariableType::Boolean)
                    }
                    BinaryExpressionOperator::Unknown => {
                        Err(SemanticError {
                            message: "Unsupported binary expression operator".to_string(),
                            index: expression.node.start,
                        })
                    }
                }
            }
            DynamicNode::UnaryExpression(node) => match &node.identifier {
                DynamicNode::Identifier(identifier) => {
                    let value_type = self.lookup(&identifier.identifier).ok_or_else(|| {
                        SemanticError {
                            message: format!("Variable '{}' does not exist.", identifier.identifier),
                            index: node.node.start,
                        }
                    })?;
                    if value_type != VariableType::Number {
                        return Err(SemanticError {
                            message: format!("Expected a number type, got {}.", value_type),
                            index: node.node.start,
                        });
                    }
                    Ok(VariableType::Number)
                }
                DynamicNode::DictionaryIdentifier(identifier) => {
                    let value_type = self
                        .lookup(&identifier.identifier)
                        .ok_or_else(|| SemanticError {
                            message: format!("Variable '{}' does not exist.", identifier.identifier),
                            index: node.node.start,
                        })?;
                    if value_type != VariableType::NumberArray {
                        return Err(SemanticError {
                            message: format!(
                                "Expected a number array type for identifier, got {}.",
                                value_type
                            ),
                            index: node.node.start,
                        });
                    }
                    Ok(VariableType::Number)
                }
                _ => Ok(VariableType::Unknown),
            },
            _ => Ok(VariableType::Unknown),
        }
    }
}

// API pública.
pub fn analyze(report: &ReportNode, source: &str) -> Result<(), SemanticError> {
    let _ = SemanticAnalyzer::new(report, source)?;
    Ok(())
}

#[cfg(test)]
#[path = "semantics_tests.rs"]
mod tests;