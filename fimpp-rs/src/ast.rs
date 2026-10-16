use std::collections::HashMap;
use crate::token::{Token, TokenType};
use crate::variable::{DynamicVariable, VariableType};

// ============================================================================
// BASE NODE & AST CURSOR (spike/node/base.go + spike/ast/ast.go)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Node {
    pub start: usize,
    pub length: usize,
}

impl Node {
    pub fn new(start: usize, length: usize) -> Self {
        Self { start, length }
    }
}

pub struct Ast<'a> {
    pub tokens: &'a [Token],
    pub token_index: usize,
    pub source: &'a str,
}

impl<'a> Ast<'a> {
    pub fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Self {
            tokens,
            token_index: 0,
            source,
        }
    }

    pub fn length(&self) -> usize {
        self.tokens.len()
    }

    pub fn peek_at(&self, index: isize) -> Token {
        if index < 0 || (index as usize) >= self.tokens.len() {
            Token::new(0, 0, TokenType::Unknown, "")
        } else {
            self.tokens[index as usize].clone()
        }
    }

    pub fn start(&self) -> Token {
        self.peek_at(0)
    }

    pub fn end(&self) -> Token {
        self.peek_at(self.length() as isize - 1)
    }

    pub fn peek(&self) -> Token {
        self.peek_at(self.token_index as isize)
    }

    pub fn peek_next(&self) -> Token {
        self.peek_at(self.token_index as isize + 1)
    }

    pub fn peek_previous(&self) -> Token {
        self.peek_at(self.token_index as isize - 1)
    }

    pub fn peek_index(&self) -> usize {
        self.token_index
    }

    pub fn next(&mut self) {
        self.token_index += 1;
    }

    pub fn end_of_file(&self) -> bool {
        self.peek().token_type == TokenType::EndOfFile
    }

    pub fn get_source_text(&self, start: usize, length: usize) -> String {
        self.source[start..start + length].to_string()
    }

    pub fn check_type(&self, token_types: &[TokenType]) -> bool {
        token_types.contains(&self.peek().token_type)
    }

    pub fn check_next_type(&self, token_types: &[TokenType]) -> bool {
        token_types.contains(&self.peek_next().token_type)
    }

    pub fn contains_func<F>(&self, predicate: F) -> bool
    where
        F: Fn(&Token) -> bool,
    {
        for idx in self.peek_index()..self.length() {
            if predicate(&self.tokens[idx]) {
                return true;
            }
        }
        false
    }

    pub fn contains_token(&self, token_type: TokenType) -> bool {
        self.contains_func(|t| t.token_type == token_type)
    }

    pub fn contains_with_stop(&self, token_type: TokenType, stop_tokens: &[TokenType]) -> bool {
        for idx in self.peek_index()..self.length() {
            let current = &self.tokens[idx];
            if stop_tokens.contains(&current.token_type) {
                break;
            }
            if current.token_type == token_type {
                return true;
            }
        }
        false
    }

    pub fn consume(&mut self) -> Token {
        self.next();
        self.peek_previous()
    }

    pub fn consume_func<F>(&mut self, predicate: F, error_message: &str) -> Result<Token, String>
    where
        F: Fn(&Token) -> bool,
    {
        let current = self.peek();
        if !predicate(&current) {
            return Err(format!("{} (en byte {})", error_message, current.start));
        }
        Ok(self.consume())
    }

    pub fn consume_token(
        &mut self,
        token_type: TokenType,
        error_message: &str,
    ) -> Result<Token, String> {
        self.consume_func(|t| t.token_type == token_type, error_message)
    }

    pub fn consume_until_func_match<F>(
        &mut self,
        predicate: F,
        error_message: &str,
    ) -> Result<Vec<Token>, String>
    where
        F: Fn(&Token) -> bool,
    {
        let mut out = Vec::new();
        while self.peek_index() < self.length() {
            let current = self.peek();
            if self.end_of_file() {
                return Err(format!("{} (en byte {})", error_message, current.start));
            }
            if predicate(&current) {
                break;
            }
            out.push(current);
            self.next();
        }
        Ok(out)
    }

    pub fn consume_until_token_match(
        &mut self,
        token_type: TokenType,
        error_message: &str,
    ) -> Result<Vec<Token>, String> {
        self.consume_until_func_match(|t| t.token_type == token_type, error_message)
    }

    pub fn consume_remaining(&mut self) -> Vec<Token> {
        let mut out = Vec::new();
        while self.peek_index() < self.length() {
            out.push(self.peek());
            self.next();
        }
        out
    }
}

// ============================================================================
// DEFINICIÓN DE NODOS DEL AST (spike/nodes/*.go)
// ============================================================================

#[derive(Debug, Clone)]
pub enum DynamicNode {
    Report(ReportNode),
    Function(FunctionNode),
    FunctionCall(FunctionCallNode),
    FunctionReturn(Box<FunctionReturnNode>),
    Identifier(IdentifierNode),
    DictionaryIdentifier(Box<DictionaryIdentifierNode>),
    Literal(LiteralNode),
    LiteralDictionary(LiteralDictionaryNode),
    BinaryExpression(Box<BinaryExpressionNode>),
    UnaryExpression(Box<UnaryExpressionNode>),
    Statements(StatementsNode),
    IfStatement(IfStatementNode),
    WhileStatement(Box<WhileStatementNode>),
    ForEveryArray(ForEveryArrayStatementNode),
    ForEveryRange(Box<ForEveryRangeStatementNode>),
    VariableDeclaration(Box<VariableDeclarationNode>),
    VariableModify(Box<VariableModifyNode>),
    ArrayModify(Box<ArrayModifyNode>),
    Print(Box<PrintNode>),
    Prompt(Box<PromptNode>),
}

impl DynamicNode {
    pub fn to_node(&self) -> Node {
        match self {
            DynamicNode::Report(n) => n.node,
            DynamicNode::Function(n) => n.node,
            DynamicNode::FunctionCall(n) => n.node,
            DynamicNode::FunctionReturn(n) => n.node,
            DynamicNode::Identifier(n) => n.node,
            DynamicNode::DictionaryIdentifier(n) => n.node,
            DynamicNode::Literal(n) => n.node,
            DynamicNode::LiteralDictionary(n) => n.node,
            DynamicNode::BinaryExpression(n) => n.node,
            DynamicNode::UnaryExpression(n) => n.node,
            DynamicNode::Statements(n) => n.node,
            DynamicNode::IfStatement(n) => n.node,
            DynamicNode::WhileStatement(n) => n.condition_stmt.statements.node,
            DynamicNode::ForEveryArray(n) => n.base.statements.node,
            DynamicNode::ForEveryRange(n) => n.base.statements.node,
            DynamicNode::VariableDeclaration(n) => n.node,
            DynamicNode::VariableModify(n) => n.node,
            DynamicNode::ArrayModify(n) => n.node,
            DynamicNode::Print(n) => n.node,
            DynamicNode::Prompt(n) => n.node,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReportNode {
    pub node: Node,
    pub title: String,
    pub author: String,
    pub body: Vec<DynamicNode>,
}

#[derive(Debug, Clone)]
pub struct FunctionNodeParameter {
    pub name: String,
    pub variable_type: VariableType,
}

#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub node: Node,
    pub name: String,
    pub main: bool,
    pub body: StatementsNode,
    pub parameters: Vec<FunctionNodeParameter>,
    pub return_type: VariableType,
}

#[derive(Debug, Clone)]
pub struct FunctionCallNode {
    pub node: Node,
    pub identifier: String,
    pub parameters: Vec<DynamicNode>,
}

#[derive(Debug, Clone)]
pub struct FunctionReturnNode {
    pub node: Node,
    pub value: DynamicNode,
}

#[derive(Debug, Clone)]
pub struct IdentifierNode {
    pub node: Node,
    pub identifier: String,
}

#[derive(Debug, Clone)]
pub struct DictionaryIdentifierNode {
    pub node: Node,
    pub identifier: String,
    pub index: DynamicNode,
}

#[derive(Debug, Clone)]
pub struct LiteralNode {
    pub node: Node,
    pub variable: DynamicVariable,
}

#[derive(Debug, Clone)]
pub struct LiteralDictionaryNode {
    pub node: Node,
    pub array_type: VariableType,
    pub values: HashMap<usize, DynamicNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryExpressionType {
    Unknown,
    Arithmetic,
    Relational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryExpressionOperator {
    Unknown,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Gte,
    Lte,
    Gt,
    Lt,
    Neq,
    Eq,
}

#[derive(Debug, Clone)]
pub struct BinaryExpressionNode {
    pub node: Node,
    pub left: DynamicNode,
    pub operator: BinaryExpressionOperator,
    pub right: DynamicNode,
    pub binary_type: BinaryExpressionType,
}

#[derive(Debug, Clone)]
pub struct UnaryExpressionNode {
    pub node: Node,
    pub identifier: DynamicNode,
    pub increment: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StatementsNode {
    pub node: Node,
    pub statements: Vec<DynamicNode>,
}

#[derive(Debug, Clone)]
pub struct ConditionStatementNode {
    pub statements: StatementsNode,
    pub condition: Option<DynamicNode>,
}

#[derive(Debug, Clone)]
pub struct IfStatementNode {
    pub node: Node,
    pub conditions: Vec<ConditionStatementNode>,
}

#[derive(Debug, Clone)]
pub struct WhileStatementNode {
    pub condition_stmt: ConditionStatementNode,
}

#[derive(Debug, Clone)]
pub struct ForEveryStatementNode {
    pub statements: StatementsNode,
    pub variable_name: String,
    pub variable_type: VariableType,
}

#[derive(Debug, Clone)]
pub struct ForEveryArrayStatementNode {
    pub base: ForEveryStatementNode,
    pub identifier: String,
}

#[derive(Debug, Clone)]
pub struct ForEveryRangeStatementNode {
    pub base: ForEveryStatementNode,
    pub range_start: DynamicNode,
    pub range_end: DynamicNode,
}

#[derive(Debug, Clone)]
pub struct VariableDeclarationNode {
    pub node: Node,
    pub identifier: String,
    pub constant: bool,
    pub value: DynamicNode,
    pub value_type: VariableType,
}

#[derive(Debug, Clone)]
pub struct VariableModifyNode {
    pub node: Node,
    pub identifier: String,
    pub value: DynamicNode,
    pub reinforcement_type: VariableType,
}

#[derive(Debug, Clone)]
pub struct ArrayModifyNode {
    pub node: Node,
    pub identifier: String,
    pub index: DynamicNode,
    pub value: DynamicNode,
    pub reinforcement_type: VariableType,
}

#[derive(Debug, Clone)]
pub struct PrintNode {
    pub node: Node,
    pub newline: bool,
    pub value: DynamicNode,
}

#[derive(Debug, Clone)]
pub struct PromptNode {
    pub node: Node,
    pub identifier: String,
    pub prompt: DynamicNode,
}

// ============================================================================
// FUNCIONES DE PARSEO (spike/report.go + spike/nodes/*.go)
// ============================================================================

pub fn create_report(tokens: &[Token], source: &str) -> Result<ReportNode, String> {
    let mut ast = Ast::new(tokens, source);
    parse_report_node(&mut ast)
}

pub fn parse_report_node(ast: &mut Ast) -> Result<ReportNode, String> {
    let start_token = ast.consume_token(
        TokenType::ReportHeader,
        &format!("Expected {}", TokenType::ReportHeader),
    )?;

    let name_tokens = ast.consume_until_token_match(
        TokenType::Punctuation,
        &format!("Could not find {}", TokenType::Punctuation),
    )?;

    let first_name = &name_tokens[0];
    let last_name = &name_tokens[name_tokens.len() - 1];
    let title = ast.get_source_text(
        first_name.start,
        last_name.start + last_name.length - first_name.start,
    );

    ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    let mut body = Vec::new();

    loop {
        if ast.check_type(&[TokenType::ReportFooter]) {
            break;
        }
        if ast.check_type(&[TokenType::EndOfFile]) {
            return Err(format!("Could not find {}", TokenType::FunctionFooter));
        }
        if ast.check_type(&[TokenType::NewLine]) {
            ast.consume();
            continue;
        }
        if ast.check_type(&[TokenType::FunctionMain, TokenType::FunctionHeader]) {
            let func = parse_function_node(ast)?;
            body.push(DynamicNode::Function(func));
            continue;
        }
        if ast.check_type(&[TokenType::Declaration]) {
            let decl = parse_variable_declaration_node(ast)?;
            body.push(DynamicNode::VariableDeclaration(Box::new(decl)));
            continue;
        }

        return Err(format!(
            "Unexpected token: {} (en byte {})",
            ast.peek().token_type,
            ast.peek().start
        ));
    }

    ast.consume_token(
        TokenType::ReportFooter,
        &format!("Expected {}", TokenType::ReportFooter),
    )?;

    let author_token = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    if !ast.end_of_file() {
        return Err(format!("Expected {}", TokenType::EndOfFile));
    }

    Ok(ReportNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        title,
        author: author_token.value,
        body,
    })
}

pub fn parse_function_node(ast: &mut Ast) -> Result<FunctionNode, String> {
    let mut main = false;
    let start_token = if ast.check_type(&[TokenType::FunctionMain]) {
        main = true;
        ast.consume_token(
            TokenType::FunctionMain,
            &format!("Expected {}", TokenType::FunctionMain),
        )?
    } else {
        ast.consume_token(
            TokenType::FunctionHeader,
            &format!("Expected {}", TokenType::FunctionHeader),
        )?
    };

    let start_name_token = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    let mut parameters: Vec<FunctionNodeParameter> = Vec::new();
    let mut return_type = VariableType::Unknown;

    loop {
        if ast.check_type(&[TokenType::FunctionParameter]) {
            ast.consume_token(
                TokenType::FunctionParameter,
                &format!("Expected {}", TokenType::FunctionParameter),
            )?;

            if !parameters.is_empty() {
                return Err("Parameter already exists".to_string());
            }

            let mut is_expecting_comma = false;
            loop {
                if ast.peek().token_type == TokenType::FunctionReturn {
                    break;
                }
                if ast.peek().token_type == TokenType::Punctuation {
                    if ast.peek().value == "," {
                        ast.consume_token(TokenType::Punctuation, "Expected ','")?;
                        is_expecting_comma = false;
                        continue;
                    }
                    break;
                }
                if is_expecting_comma {
                    return Err("Expecting parameters separated by comma".to_string());
                }

                let param_type_tok = ast.peek();
                let param_type = VariableType::from_token_type_hint(param_type_tok.token_type);
                if param_type == VariableType::Unknown {
                    return Err("Expected variable type".to_string());
                }
                ast.next();

                let lit_id = ast.consume_token(
                    TokenType::Identifier,
                    &format!("Expected {}", TokenType::Identifier),
                )?;

                if parameters.iter().any(|p| p.name == lit_id.value) {
                    return Err("Parameter already exists".to_string());
                }

                parameters.push(FunctionNodeParameter {
                    name: lit_id.value,
                    variable_type: param_type,
                });

                is_expecting_comma = true;
            }
            continue;
        }

        if ast.check_type(&[TokenType::FunctionReturn]) {
            ast.consume_token(
                TokenType::FunctionReturn,
                &format!("Expected {}", TokenType::FunctionReturn),
            )?;

            if return_type != VariableType::Unknown {
                return Err("Return type already exists".to_string());
            }

            let hint_tok = ast.peek();
            let possible_hint = VariableType::from_token_type_hint(hint_tok.token_type);
            if possible_hint == VariableType::Unknown {
                return Err("Expected variable type hint".to_string());
            }
            return_type = possible_hint;
            ast.next();
            continue;
        }

        break;
    }

    ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    let body = parse_statements_node(ast, &[TokenType::FunctionFooter])?;

    ast.consume_token(
        TokenType::FunctionFooter,
        &format!("Expected {}", TokenType::FunctionFooter),
    )?;

    let end_name_token = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    if start_name_token.value != end_name_token.value {
        return Err(format!(
            "Mismatch method name. Expected '{}', got '{}'",
            start_name_token.value, end_name_token.value
        ));
    }

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(FunctionNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        name: start_name_token.value,
        main,
        body,
        parameters,
        return_type,
    })
}

pub fn parse_function_call_node(ast: &mut Ast) -> Result<FunctionCallNode, String> {
    let start_token = ast.consume_token(
        TokenType::FunctionCall,
        &format!("Expected {}", TokenType::FunctionCall),
    )?;

    let name_token = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    let mut parameters = Vec::new();

    if ast.check_type(&[TokenType::FunctionParameter]) {
        ast.consume_token(
            TokenType::FunctionParameter,
            &format!("Expected {}", TokenType::FunctionParameter),
        )?;

        let mut is_expecting_comma = false;
        loop {
            if ast.peek().token_type == TokenType::Punctuation {
                if ast.peek().value == "," {
                    ast.consume_token(TokenType::Punctuation, "Expected ','")?;
                    is_expecting_comma = false;
                    continue;
                }
                break;
            }

            if is_expecting_comma {
                return Err("Expecting parameters separated by comma".to_string());
            }

            let param_type = VariableType::from_token_type_hint(ast.peek().token_type);
            if param_type != VariableType::Unknown {
                ast.next();
            }

            let val_tokens = ast.consume_until_func_match(
                |t| t.token_type == TokenType::Punctuation,
                &format!("Could not find {}", TokenType::Punctuation),
            )?;

            let val_node = create_value_node(&val_tokens, CreateValueNodeOptions::default())?;
            parameters.push(val_node);
            is_expecting_comma = true;
        }
    }

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(FunctionCallNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        identifier: name_token.value,
        parameters,
    })
}

pub fn parse_function_return_node(ast: &mut Ast) -> Result<FunctionReturnNode, String> {
    let start_token = ast.consume_token(
        TokenType::KeywordReturn,
        &format!("Expected {}", TokenType::KeywordReturn),
    )?;

    let val_tokens = ast.consume_until_token_match(
        TokenType::Punctuation,
        &format!("Could not find {}", TokenType::Punctuation),
    )?;

    let value = create_value_node(&val_tokens, CreateValueNodeOptions::default())?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(FunctionReturnNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        value,
    })
}

pub fn parse_identifier_node(ast: &mut Ast) -> Result<DynamicNode, String> {
    if ast.contains_with_stop(
        TokenType::KeywordOf,
        &[TokenType::EndOfFile, TokenType::Punctuation],
    ) {
        let index_tokens = ast.consume_until_token_match(
            TokenType::KeywordOf,
            &format!("Expected {}", TokenType::KeywordOf),
        )?;
        let index_node = create_value_node(&index_tokens, CreateValueNodeOptions::default())?;

        ast.consume_token(
            TokenType::KeywordOf,
            &format!("Expected {}", TokenType::KeywordOf),
        )?;

        let id_tok = ast.consume_token(
            TokenType::Identifier,
            &format!("Expected {}", TokenType::Identifier),
        )?;

        let start = index_node.to_node().start;
        let length = id_tok.start + id_tok.length - start;

        Ok(DynamicNode::DictionaryIdentifier(Box::new(
            DictionaryIdentifierNode {
                node: Node::new(start, length),
                identifier: id_tok.value,
                index: index_node,
            },
        )))
    } else {
        let id_tok = ast.consume_token(
            TokenType::Identifier,
            &format!("Expected {}", TokenType::Identifier),
        )?;

        Ok(DynamicNode::Identifier(IdentifierNode {
            node: Node::new(id_tok.start, id_tok.length),
            identifier: id_tok.value,
        }))
    }
}

pub fn parse_prefix_unary(ast: &mut Ast) -> Result<UnaryExpressionNode, String> {
    let start_token = ast.consume_func(
        |t| {
            matches!(
                t.token_type,
                TokenType::UnaryIncrementPrefix | TokenType::UnaryDecrementPrefix
            )
        },
        "Expected unary prefix token",
    )?;
    let increment = start_token.token_type == TokenType::UnaryIncrementPrefix;

    let identifier = parse_identifier_node(ast)?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(UnaryExpressionNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        identifier,
        increment,
    })
}

pub fn parse_postfix_unary(ast: &mut Ast) -> Result<UnaryExpressionNode, String> {
    let identifier = parse_identifier_node(ast)?;

    let postfix_token = ast.consume_func(
        |t| {
            matches!(
                t.token_type,
                TokenType::UnaryIncrementPostfix | TokenType::UnaryDecrementPostfix
            )
        },
        "Expected unary postfix token",
    )?;
    let increment = postfix_token.token_type == TokenType::UnaryIncrementPostfix;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    let start = identifier.to_node().start;
    Ok(UnaryExpressionNode {
        node: Node::new(start, end_token.start + end_token.length - start),
        identifier,
        increment,
    })
}

pub fn parse_statements_node(
    cur_ast: &mut Ast,
    expected_end_types: &[TokenType],
) -> Result<StatementsNode, String> {
    let mut statements = Vec::new();

    loop {
        if cur_ast.check_type(expected_end_types) {
            break;
        }
        if cur_ast.check_type(&[TokenType::EndOfFile]) {
            return Err(format!("Could not find {}", TokenType::FunctionFooter));
        }
        if cur_ast.check_type(&[TokenType::NewLine, TokenType::Punctuation]) {
            cur_ast.consume();
            continue;
        }

        if cur_ast.check_type(&[TokenType::Print, TokenType::PrintNewline]) {
            let n = parse_print_node(cur_ast)?;
            statements.push(DynamicNode::Print(Box::new(n)));
            continue;
        }
        if cur_ast.check_type(&[TokenType::Prompt]) {
            let n = parse_prompt_node(cur_ast)?;
            statements.push(DynamicNode::Prompt(Box::new(n)));
            continue;
        }
        if cur_ast.check_type(&[TokenType::Declaration]) {
            let n = parse_variable_declaration_node(cur_ast)?;
            statements.push(DynamicNode::VariableDeclaration(Box::new(n)));
            continue;
        }
        if cur_ast.check_type(&[TokenType::Identifier])
            && cur_ast.check_next_type(&[TokenType::Modify])
        {
            let n = parse_variable_modify_node(cur_ast)?;
            statements.push(DynamicNode::VariableModify(Box::new(n)));
            continue;
        }
        if cur_ast.check_type(&[TokenType::FunctionCall]) {
            let n = parse_function_call_node(cur_ast)?;
            statements.push(DynamicNode::FunctionCall(n));
            continue;
        }
        if cur_ast.check_type(&[TokenType::IfClause]) {
            let n = parse_if_statements_node(cur_ast)?;
            statements.push(DynamicNode::IfStatement(n));
            continue;
        }
        if cur_ast.check_type(&[TokenType::WhileClause]) {
            let n = parse_while_statement_node(cur_ast)?;
            statements.push(DynamicNode::WhileStatement(Box::new(n)));
            continue;
        }
        if cur_ast.check_type(&[TokenType::KeywordReturn]) {
            let n = parse_function_return_node(cur_ast)?;
            statements.push(DynamicNode::FunctionReturn(Box::new(n)));
            continue;
        }
        if cur_ast.check_type(&[TokenType::ForEveryClause])
            && cur_ast.contains_with_stop(
            TokenType::KeywordIn,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        )
        {
            let n = parse_for_every_array_statement_node(cur_ast)?;
            statements.push(DynamicNode::ForEveryArray(n));
            continue;
        }
        if cur_ast.check_type(&[TokenType::ForEveryClause])
            && cur_ast.contains_with_stop(
            TokenType::KeywordFrom,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        )
            && cur_ast.contains_with_stop(
            TokenType::KeywordTo,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        )
        {
            let n = parse_for_every_range_statement_node(cur_ast)?;
            statements.push(DynamicNode::ForEveryRange(Box::new(n)));
            continue;
        }
        if cur_ast.contains_with_stop(
            TokenType::KeywordOf,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        ) && cur_ast.contains_with_stop(
            TokenType::OperatorEq,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        ) {
            let n = parse_array_modify_node(cur_ast)?;
            statements.push(DynamicNode::ArrayModify(Box::new(n)));
            continue;
        }
        if cur_ast.contains_with_stop(
            TokenType::UnaryIncrementPrefix,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        ) || cur_ast.contains_with_stop(
            TokenType::UnaryDecrementPrefix,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        ) {
            let n = parse_prefix_unary(cur_ast)?;
            statements.push(DynamicNode::UnaryExpression(Box::new(n)));
            continue;
        }
        if cur_ast.contains_with_stop(
            TokenType::UnaryIncrementPostfix,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        ) || cur_ast.contains_with_stop(
            TokenType::UnaryDecrementPostfix,
            &[TokenType::EndOfFile, TokenType::Punctuation],
        ) {
            let n = parse_postfix_unary(cur_ast)?;
            statements.push(DynamicNode::UnaryExpression(Box::new(n)));
            continue;
        }

        return Err(format!(
            "Unsupported statement token: {} (en byte {})",
            cur_ast.peek().token_type,
            cur_ast.peek().start
        ));
    }

    Ok(StatementsNode {
        node: Node::default(),
        statements,
    })
}

pub fn parse_for_every_array_statement_node(
    ast: &mut Ast,
) -> Result<ForEveryArrayStatementNode, String> {
    let start_token = ast.consume_token(
        TokenType::ForEveryClause,
        &format!("Expected {}", TokenType::ForEveryClause),
    )?;

    let type_token = ast.consume();
    let possible_type = VariableType::from_token_type_hint(type_token.token_type);
    if possible_type == VariableType::Unknown || possible_type.is_array() {
        return Err("Expected non-array variable type".to_string());
    }

    let var_name_tok = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    ast.consume_token(
        TokenType::KeywordIn,
        &format!("Expected {}", TokenType::KeywordIn),
    )?;

    let id_tok = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    let mut statements = parse_statements_node(ast, &[TokenType::KeywordStatementEnd])?;

    ast.consume_token(
        TokenType::KeywordStatementEnd,
        &format!("Expected {}", TokenType::KeywordStatementEnd),
    )?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    statements.node = Node::new(
        start_token.start,
        end_token.start + end_token.length - start_token.start,
    );

    Ok(ForEveryArrayStatementNode {
        base: ForEveryStatementNode {
            statements,
            variable_name: var_name_tok.value,
            variable_type: possible_type,
        },
        identifier: id_tok.value,
    })
}

pub fn parse_for_every_range_statement_node(
    ast: &mut Ast,
) -> Result<ForEveryRangeStatementNode, String> {
    let start_token = ast.consume_token(
        TokenType::ForEveryClause,
        &format!("Expected {}", TokenType::ForEveryClause),
    )?;

    let type_token = ast.consume();
    let possible_type = VariableType::from_token_type_hint(type_token.token_type);
    if possible_type == VariableType::Unknown || possible_type.is_array() {
        return Err("Expected variable type".to_string());
    }

    let id_tok = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    ast.consume_token(
        TokenType::KeywordFrom,
        &format!("Expected {}", TokenType::KeywordFrom),
    )?;

    let from_tokens = ast.consume_until_token_match(TokenType::KeywordTo, "Expected 'to' token")?;
    let range_start = create_value_node(&from_tokens, CreateValueNodeOptions::default())?;

    ast.consume_token(
        TokenType::KeywordTo,
        &format!("Expected {}", TokenType::KeywordTo),
    )?;

    let to_tokens = ast.consume_until_token_match(TokenType::Punctuation, "Expected punctuation")?;
    let range_end = create_value_node(&to_tokens, CreateValueNodeOptions::default())?;

    ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    let mut statements = parse_statements_node(ast, &[TokenType::KeywordStatementEnd])?;

    ast.consume_token(
        TokenType::KeywordStatementEnd,
        &format!("Expected {}", TokenType::KeywordStatementEnd),
    )?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    statements.node = Node::new(
        start_token.start,
        end_token.start + end_token.length - start_token.start,
    );

    Ok(ForEveryRangeStatementNode {
        base: ForEveryStatementNode {
            statements,
            variable_name: id_tok.value,
            variable_type: possible_type,
        },
        range_start,
        range_end,
    })
}

pub fn parse_if_statements_node(cur_ast: &mut Ast) -> Result<IfStatementNode, String> {
    let mut conditions = Vec::new();

    let start_token = cur_ast.consume_token(
        TokenType::IfClause,
        &format!("Expected {}", TokenType::IfClause),
    )?;

    let cond_tokens = cur_ast.consume_until_func_match(
        |t| matches!(t.token_type, TokenType::Punctuation | TokenType::KeywordThen),
        "Expected token for statement condition ending",
    )?;
    let cond_node = create_value_node(&cond_tokens, CreateValueNodeOptions::default())?;

    if cur_ast.check_type(&[TokenType::KeywordThen]) {
        cur_ast.consume_token(TokenType::KeywordThen, "Expected THEN")?;
    }
    cur_ast.consume_func(
        |t| t.token_type == TokenType::Punctuation && t.value == ",",
        "Expected ','",
    )?;

    let statements =
        parse_statements_node(cur_ast, &[TokenType::ElseClause, TokenType::IfEndClause])?;
    conditions.push(ConditionStatementNode {
        statements,
        condition: Some(cond_node),
    });

    let mut has_else_clause = false;

    while cur_ast.check_type(&[TokenType::ElseClause]) {
        cur_ast.consume_token(TokenType::ElseClause, "Expected ELSE")?;

        let cond_tokens = cur_ast.consume_until_func_match(
            |t| matches!(t.token_type, TokenType::Punctuation | TokenType::KeywordThen),
            "Expected token for statement condition ending",
        )?;

        let condition = if !cond_tokens.is_empty() {
            Some(create_value_node(
                &cond_tokens,
                CreateValueNodeOptions::default(),
            )?)
        } else {
            if has_else_clause {
                return Err("Else condition already exists".to_string());
            }
            has_else_clause = true;
            None
        };

        if cur_ast.check_type(&[TokenType::KeywordThen]) {
            cur_ast.consume_token(TokenType::KeywordThen, "Expected THEN")?;
        }
        cur_ast.consume_func(
            |t| t.token_type == TokenType::Punctuation && t.value == ",",
            "Expected ','",
        )?;

        let statements =
            parse_statements_node(cur_ast, &[TokenType::ElseClause, TokenType::IfEndClause])?;
        conditions.push(ConditionStatementNode {
            statements,
            condition,
        });
    }

    cur_ast.consume_token(
        TokenType::IfEndClause,
        &format!("Expected {}", TokenType::IfEndClause),
    )?;

    let end_token = cur_ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(IfStatementNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        conditions,
    })
}

pub fn parse_while_statement_node(cur_ast: &mut Ast) -> Result<WhileStatementNode, String> {
    let start_token = cur_ast.consume_token(
        TokenType::WhileClause,
        &format!("Expected {}", TokenType::WhileClause),
    )?;

    let cond_tokens = cur_ast.consume_until_func_match(
        |t| matches!(t.token_type, TokenType::Punctuation | TokenType::KeywordThen),
        "Expected token for statement condition ending",
    )?;
    let cond_node = create_value_node(&cond_tokens, CreateValueNodeOptions::default())?;

    if cur_ast.check_type(&[TokenType::KeywordThen]) {
        cur_ast.consume();
    }

    cur_ast.consume_func(
        |t| t.token_type == TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    let mut statements = parse_statements_node(cur_ast, &[TokenType::KeywordStatementEnd])?;

    cur_ast.consume_token(
        TokenType::KeywordStatementEnd,
        &format!("Expected {}", TokenType::KeywordStatementEnd),
    )?;

    let end_token = cur_ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    statements.node = Node::new(
        start_token.start,
        end_token.start + end_token.length - start_token.start,
    );

    Ok(WhileStatementNode {
        condition_stmt: ConditionStatementNode {
            statements,
            condition: Some(cond_node),
        },
    })
}

pub fn consume_until_punctuation(ast: &mut Ast, is_array: bool) -> Result<Vec<Token>, String> {
    if is_array
        || ast.contains_with_stop(
        TokenType::FunctionParameter,
        &[TokenType::EndOfFile, TokenType::Punctuation],
    )
    {
        ast.consume_until_func_match(
            |t| t.token_type == TokenType::Punctuation && t.value != ",",
            &format!("Could not find {}", TokenType::Punctuation),
        )
    } else {
        ast.consume_until_token_match(
            TokenType::Punctuation,
            &format!("Could not find {}", TokenType::Punctuation),
        )
    }
}

pub fn parse_variable_declaration_node(
    ast: &mut Ast,
) -> Result<VariableDeclarationNode, String> {
    let start_token = ast.consume_token(
        TokenType::Declaration,
        &format!("Expected {}", TokenType::Declaration),
    )?;

    let identifier = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    ast.consume_token(
        TokenType::OperatorEq,
        &format!("Expected {}", TokenType::OperatorEq),
    )?;

    let mut constant = false;
    if ast.peek().token_type == TokenType::KeywordConst {
        constant = true;
        ast.next();
    }

    let type_token = ast.peek();
    let value_type = VariableType::from_token_type_hint(type_token.token_type);
    if value_type == VariableType::Unknown {
        return Err("Expected variable type hint".to_string());
    }
    ast.next();

    let value_tokens = consume_until_punctuation(ast, value_type.is_array())?;

    let value = create_value_node(
        &value_tokens,
        CreateValueNodeOptions {
            possible_null_type: Some(value_type),
            into_array: value_type.is_array(),
        },
    )?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(VariableDeclarationNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        identifier: identifier.value,
        constant,
        value,
        value_type,
    })
}

pub fn parse_variable_modify_node(ast: &mut Ast) -> Result<VariableModifyNode, String> {
    let start_token = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    ast.consume_token(
        TokenType::Modify,
        &format!("Expected {}", TokenType::Modify),
    )?;

    let possible_type = VariableType::from_token_type_hint(ast.peek().token_type);
    if possible_type != VariableType::Unknown && !possible_type.is_array() {
        ast.next();
    }

    let value_tokens = consume_until_punctuation(ast, false)?;
    let value = create_value_node(&value_tokens, CreateValueNodeOptions::default())?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(VariableModifyNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        identifier: start_token.value,
        value,
        reinforcement_type: VariableType::Unknown,
    })
}

pub fn parse_array_modify_node(ast: &mut Ast) -> Result<ArrayModifyNode, String> {
    let index_tokens = ast.consume_until_token_match(
        TokenType::KeywordOf,
        &format!("Could not find {}", TokenType::KeywordOf),
    )?;
    let index = create_value_node(&index_tokens, CreateValueNodeOptions::default())?;

    ast.consume_token(
        TokenType::KeywordOf,
        &format!("Expected {}", TokenType::KeywordOf),
    )?;

    let id_token = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    ast.consume_token(
        TokenType::OperatorEq,
        &format!("Expected {}", TokenType::OperatorEq),
    )?;

    let possible_type = VariableType::from_token_type_hint(ast.peek().token_type);
    if possible_type != VariableType::Unknown && !possible_type.is_array() {
        ast.next();
    }

    let value_tokens = ast.consume_until_token_match(
        TokenType::Punctuation,
        &format!("Could not find {}", TokenType::Punctuation),
    )?;
    let value = create_value_node(&value_tokens, CreateValueNodeOptions::default())?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    let start = index.to_node().start;
    Ok(ArrayModifyNode {
        node: Node::new(start, end_token.start + end_token.length - start),
        identifier: id_token.value,
        index,
        value,
        reinforcement_type: VariableType::Unknown,
    })
}

pub fn parse_print_node(ast: &mut Ast) -> Result<PrintNode, String> {
    let start_token = ast.consume_func(
        |t| matches!(t.token_type, TokenType::Print | TokenType::PrintNewline),
        "Expected print token",
    )?;
    let newline = start_token.token_type == TokenType::PrintNewline;

    let value_tokens = consume_until_punctuation(ast, false)?;
    let value = create_value_node(&value_tokens, CreateValueNodeOptions::default())?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(PrintNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        newline,
        value,
    })
}

pub fn parse_prompt_node(ast: &mut Ast) -> Result<PromptNode, String> {
    let start_token = ast.consume_token(
        TokenType::Prompt,
        &format!("Expected {}", TokenType::Prompt),
    )?;

    let identifier = ast.consume_token(
        TokenType::Identifier,
        &format!("Expected {}", TokenType::Identifier),
    )?;

    ast.consume_func(
        |t| t.token_type == TokenType::Punctuation && t.value == ":",
        "Expected ':'",
    )?;

    let value_tokens = ast.consume_until_token_match(
        TokenType::Punctuation,
        &format!("Could not find {}", TokenType::Punctuation),
    )?;
    let prompt = create_value_node(&value_tokens, CreateValueNodeOptions::default())?;

    let end_token = ast.consume_token(
        TokenType::Punctuation,
        &format!("Expected {}", TokenType::Punctuation),
    )?;

    Ok(PromptNode {
        node: Node::new(
            start_token.start,
            end_token.start + end_token.length - start_token.start,
        ),
        identifier: identifier.value,
        prompt,
    })
}

// ============================================================================
// EXPRESIONES Y VALORES (spike/nodes/misc_value.go + expr_binary.go)
// ============================================================================

#[derive(Default, Clone, Copy)]
pub struct CreateValueNodeOptions {
    pub possible_null_type: Option<VariableType>,
    pub into_array: bool,
}

fn wrap_as_dictionary_node(
    n: DynamicNode,
    array_type: VariableType,
    start: usize,
    length: usize,
) -> DynamicNode {
    let mut values = HashMap::new();
    values.insert(1, n);

    DynamicNode::LiteralDictionary(LiteralDictionaryNode {
        node: Node::new(start, length),
        array_type,
        values,
    })
}

pub fn create_value_node(
    tokens: &[Token],
    options: CreateValueNodeOptions,
) -> Result<DynamicNode, String> {
    let mut temp_ast = Ast::new(tokens, "");

    if temp_ast.length() == 0 {
        if let Some(null_type) = options.possible_null_type {
            if null_type.is_array() {
                return Ok(DynamicNode::Literal(LiteralNode {
                    node: Node::new(0, 0),
                    variable: DynamicVariable::new_dictionary(null_type),
                }));
            }

            let default_val = null_type
                .get_default_value()
                .expect("AST::create_value_node called with unknown possible_null_type");

            return Ok(DynamicNode::Literal(LiteralNode {
                node: Node::new(0, 0),
                variable: DynamicVariable::from_value_type(&default_val, null_type),
            }));
        }
        panic!("AST::create_value_node called without any tokens");
    }

    if temp_ast.length() == 1 {
        if temp_ast.peek().value == "nothing" && options.possible_null_type.is_none() {
            let t = temp_ast.peek();
            return Ok(DynamicNode::Literal(LiteralNode {
                node: Node::new(t.start, t.length),
                variable: DynamicVariable::new_unknown(),
            }));
        }

        if temp_ast.check_type(&[TokenType::Identifier]) {
            let t = temp_ast.consume();
            let node = DynamicNode::Identifier(IdentifierNode {
                node: Node::new(t.start, t.length),
                identifier: t.value,
            });

            if let Some(null_type) = options.possible_null_type {
                if null_type.is_array() {
                    return Ok(wrap_as_dictionary_node(node, null_type, t.start, t.length));
                }
            }
            return Ok(node);
        }

        let t = temp_ast.consume();
        let default_type = VariableType::from_token_type(t.token_type);
        if default_type != VariableType::Unknown {
            let lit = DynamicNode::Literal(LiteralNode {
                node: Node::new(t.start, t.length),
                variable: DynamicVariable::from_value_type(&t.value, default_type),
            });

            if let Some(null_type) = options.possible_null_type {
                if null_type.is_array() {
                    return Ok(wrap_as_dictionary_node(lit, null_type, t.start, t.length));
                }
            }
            return Ok(lit);
        }

        if t.token_type == TokenType::Null {
            if let Some(null_type) = options.possible_null_type {
                if null_type.is_array() {
                    return Ok(DynamicNode::Literal(LiteralNode {
                        node: Node::new(t.start, t.length),
                        variable: DynamicVariable::new_dictionary(null_type),
                    }));
                }
                let default_val = null_type
                    .get_default_value()
                    .expect("Literal null called with no possible default value");
                return Ok(DynamicNode::Literal(LiteralNode {
                    node: Node::new(t.start, t.length),
                    variable: DynamicVariable::from_value_type(&default_val, null_type),
                }));
            }
            return Ok(DynamicNode::Literal(LiteralNode {
                node: Node::new(0, 0),
                variable: DynamicVariable::new_unknown(),
            }));
        }
    }

    if temp_ast.length() == 2 {
        let possible_type_tok = temp_ast.peek();
        let possible_type = VariableType::from_token_type_hint(possible_type_tok.token_type);
        if possible_type != VariableType::Unknown
            && !possible_type.is_array()
            && temp_ast.peek_next().token_type == TokenType::Null
        {
            let null_tok = temp_ast.peek_next();
            let default_val = possible_type
                .get_default_value()
                .expect("Literal null called with no possible default value");
            return Ok(DynamicNode::Literal(LiteralNode {
                node: Node::new(
                    possible_type_tok.start,
                    null_tok.start + null_tok.length - possible_type_tok.start,
                ),
                variable: DynamicVariable::from_value_type(&default_val, possible_type),
            }));
        }
    }

    if temp_ast.length() > 1 {
        // FunctionCallNode dentro de expresión
        if temp_ast.peek().token_type == TokenType::Identifier
            && temp_ast.peek_next().token_type == TokenType::FunctionParameter
        {
            let id = temp_ast.consume();
            temp_ast.consume();

            let mut parameters = Vec::new();
            let mut is_expecting_comma = false;

            while temp_ast.peek_index() < temp_ast.length() {
                if temp_ast.check_type(&[TokenType::Punctuation]) {
                    if temp_ast.peek().value == "," {
                        temp_ast.consume();
                        is_expecting_comma = false;
                        continue;
                    }
                    break;
                }
                if is_expecting_comma {
                    return Err("Expecting parameters separated by comma".to_string());
                }

                let param_type = VariableType::from_token_type_hint(temp_ast.peek().token_type);
                if param_type != VariableType::Unknown {
                    temp_ast.next();
                }

                let val_tokens = temp_ast.consume_until_func_match(
                    |t| t.token_type == TokenType::Punctuation,
                    "Could not find punctuation",
                )?;
                let val_node = create_value_node(&val_tokens, CreateValueNodeOptions::default())?;
                parameters.push(val_node);
                is_expecting_comma = true;
            }

            let end = temp_ast.end();
            return Ok(DynamicNode::FunctionCall(FunctionCallNode {
                node: Node::new(id.start, end.start + end.length - id.start),
                identifier: id.value,
                parameters,
            }));
        }

        let boolean_expressions = [
            (
                TokenType::KeywordAnd,
                BinaryExpressionOperator::And,
                BinaryExpressionType::Relational,
            ),
            (
                TokenType::KeywordOr,
                BinaryExpressionOperator::Or,
                BinaryExpressionType::Relational,
            ),
            (
                TokenType::OperatorGte,
                BinaryExpressionOperator::Gte,
                BinaryExpressionType::Relational,
            ),
            (
                TokenType::OperatorLte,
                BinaryExpressionOperator::Lte,
                BinaryExpressionType::Relational,
            ),
            (
                TokenType::OperatorGt,
                BinaryExpressionOperator::Gt,
                BinaryExpressionType::Relational,
            ),
            (
                TokenType::OperatorLt,
                BinaryExpressionOperator::Lt,
                BinaryExpressionType::Relational,
            ),
            (
                TokenType::OperatorNeq,
                BinaryExpressionOperator::Neq,
                BinaryExpressionType::Relational,
            ),
            (
                TokenType::OperatorEq,
                BinaryExpressionOperator::Eq,
                BinaryExpressionType::Relational,
            ),
        ];

        for (tok_type, op, bin_type) in boolean_expressions {
            if let Some(expr) = create_expression(tokens, tok_type, op, bin_type)? {
                return Ok(DynamicNode::BinaryExpression(Box::new(expr)));
            }
        }

        // DictionaryIdentifierNode
        if temp_ast.contains_token(TokenType::KeywordOf)
            && temp_ast.end().token_type != TokenType::KeywordOf
        {
            let index_tokens = temp_ast.consume_until_token_match(TokenType::KeywordOf, "")?;
            temp_ast.consume();
            let id_tokens = temp_ast.consume_remaining();

            if index_tokens.is_empty() {
                return Err("Expected dictionary identifier index".to_string());
            }
            let index = create_value_node(&index_tokens, CreateValueNodeOptions::default())?;

            if id_tokens.len() != 1 || id_tokens[0].token_type != TokenType::Identifier {
                return Err("Expected dictionary identifier".to_string());
            }

            let start_tok = temp_ast.start();
            let end_tok = temp_ast.end();

            return Ok(DynamicNode::DictionaryIdentifier(Box::new(
                DictionaryIdentifierNode {
                    node: Node::new(
                        start_tok.start,
                        end_tok.start + end_tok.length - start_tok.start,
                    ),
                    identifier: id_tokens[0].value.clone(),
                    index,
                },
            )));
        }

        let arithmetic_expressions = [
            (
                TokenType::OperatorSubInfix,
                BinaryExpressionOperator::Sub,
                BinaryExpressionType::Arithmetic,
            ),
            (
                TokenType::OperatorAddInfix,
                BinaryExpressionOperator::Add,
                BinaryExpressionType::Arithmetic,
            ),
            (
                TokenType::OperatorDivInfix,
                BinaryExpressionOperator::Div,
                BinaryExpressionType::Arithmetic,
            ),
            (
                TokenType::OperatorMulInfix,
                BinaryExpressionOperator::Mul,
                BinaryExpressionType::Arithmetic,
            ),
            (
                TokenType::OperatorModInfix,
                BinaryExpressionOperator::Mod,
                BinaryExpressionType::Arithmetic,
            ),
        ];

        for (tok_type, op, bin_type) in arithmetic_expressions {
            if let Some(expr) = create_expression(tokens, tok_type, op, bin_type)? {
                return Ok(DynamicNode::BinaryExpression(Box::new(expr)));
            }
        }

        // LiteralDictionaryNode (ej: "a", "b", "c")
        if temp_ast.contains_func(|t| t.token_type == TokenType::Punctuation && t.value == ",") {
            let null_type = options
                .possible_null_type
                .expect("LiteralDictionaryNode without possible_null_type");
            if !null_type.is_array() {
                panic!("LiteralDictionaryNode with non-array possible_null_type");
            }

            let base_type = null_type.as_base_type();
            let mut values = HashMap::new();
            let mut current_index = 1;

            while temp_ast.peek_index() < temp_ast.length() {
                if current_index != 1 {
                    temp_ast.consume_func(
                        |t| t.token_type == TokenType::Punctuation && t.value == ",",
                        "Expected ','",
                    )?;
                }

                let ts = temp_ast.consume_until_func_match(
                    |t| t.token_type == TokenType::Punctuation && t.value == ",",
                    "",
                )?;

                if ts.is_empty() {
                    break;
                }

                let val = create_value_node(
                    &ts,
                    CreateValueNodeOptions {
                        possible_null_type: Some(base_type),
                        into_array: false,
                    },
                )?;

                values.insert(current_index, val);
                current_index += 1;
            }

            let start_tok = temp_ast.start();
            let end_tok = temp_ast.end();

            return Ok(DynamicNode::LiteralDictionary(LiteralDictionaryNode {
                node: Node::new(
                    start_tok.start,
                    end_tok.start + end_tok.length - start_tok.start,
                ),
                array_type: null_type,
                values,
            }));
        }
    }

    let unknown_str: String = tokens.iter().map(|t| t.value.as_str()).collect();
    Err(format!("Encountered unknown value token: '{}'", unknown_str))
}

pub fn create_expression(
    tokens: &[Token],
    token_type: TokenType,
    operator: BinaryExpressionOperator,
    binary_type: BinaryExpressionType,
) -> Result<Option<BinaryExpressionNode>, String> {
    let Some(index) = tokens.iter().position(|t| t.token_type == token_type) else {
        return Ok(None);
    };

    let left_node = create_value_node(&tokens[..index], CreateValueNodeOptions::default())?;
    let right_node = create_value_node(&tokens[index + 1..], CreateValueNodeOptions::default())?;

    let left_span = left_node.to_node();
    let right_span = right_node.to_node();

    Ok(Some(BinaryExpressionNode {
        node: Node::new(
            left_span.start,
            right_span.start + right_span.length - left_span.start,
        ),
        left: left_node,
        operator,
        right: right_node,
        binary_type,
    }))
}

#[cfg(test)]
#[path = "ast_tests.rs"]
mod tests;
