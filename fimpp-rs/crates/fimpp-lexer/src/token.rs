use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Unknown,

    Identifier,
    Punctuation,
    NewLine,
    Whitespace,
    EndOfFile,

    CommentParen,
    CommentPostScript,

    String,
    Character,
    Number,
    Boolean,

    Null,

    ReportHeader,
    ReportFooter,

    FunctionHeader,
    FunctionMain,
    FunctionFooter,

    FunctionReturn,
    FunctionParameter,

    Print,
    PrintNewline,
    Prompt,
    FunctionCall,

    Declaration,
    Modify,

    TypeString,
    TypeChar,
    TypeNumber,
    TypeBoolean,

    TypeStringArray,
    TypeNumberArray,
    TypeBooleanArray,

    OperatorEq,
    OperatorNeq,
    OperatorGt,
    OperatorGte,
    OperatorLt,
    OperatorLte,

    UnaryNot,

    OperatorAddInfix,
    OperatorAddPrefix,
    UnaryIncrementPrefix,
    UnaryIncrementPostfix,

    OperatorSubInfix,
    OperatorSubPrefix,
    UnaryDecrementPrefix,
    UnaryDecrementPostfix,

    OperatorMulInfix,
    OperatorMulPrefix,

    OperatorDivInfix,
    OperatorDivPrefix,

    OperatorModInfix,
    OperatorModPrefix,

    KeywordOr,
    KeywordAnd,
    KeywordConst,
    KeywordOf,
    KeywordIn,
    KeywordFrom,
    KeywordTo,
    KeywordThen,
    KeywordStatementEnd,
    KeywordReturn,

    IfClause,
    ElseClause,
    IfEndClause,

    WhileClause,

    ForEveryClause,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::Unknown => "UNKNOWN",
            TokenType::Identifier => "IDENTIFIER",
            TokenType::Punctuation => "PUNCTUATION",
            TokenType::NewLine => "NEWLINE",
            TokenType::Whitespace => "WHITESPACE",
            TokenType::EndOfFile => "EOF",
            TokenType::CommentParen | TokenType::CommentPostScript => "COMMENT",
            TokenType::String => "LITERAL(STRING)",
            TokenType::Character => "LITERAL(CHARACTER)",
            TokenType::Number => "LITERAL(NUMBER)",
            TokenType::Boolean => "LITERAL(BOOL)",
            TokenType::Null => "NULL",
            TokenType::ReportHeader => "REPORT(HEADER)",
            TokenType::ReportFooter => "REPORT(FOOTER)",
            TokenType::FunctionHeader => "FUNCTION(HEADER)",
            TokenType::FunctionMain => "FUNCTION(MAIN)",
            TokenType::FunctionFooter => "FUNCTION(FOOTER)",
            TokenType::FunctionReturn => "FUNCTION(RETURN)",
            TokenType::FunctionParameter => "FUNCTION(PARAMETER)",
            TokenType::Print => "PRINT",
            TokenType::PrintNewline => "PRINT(NEWLINE)",
            TokenType::Prompt => "PROMPT",
            TokenType::FunctionCall => "FUNCTION(CALL)",
            TokenType::Declaration => "VARIABLE(DECLARATION)",
            TokenType::Modify => "VARIABLE(MODIFY)",
            TokenType::TypeString => "TYPE(STRING)",
            TokenType::TypeChar => "TYPE(CHARACTER)",
            TokenType::TypeNumber => "TYPE(NUMBER)",
            TokenType::TypeBoolean => "TYPE(BOOLEAN)",
            TokenType::TypeStringArray => "TYPE(STRING_ARRAY)",
            TokenType::TypeNumberArray => "TYPE(NUMBER_ARRAY)",
            TokenType::TypeBooleanArray => "TYPE(BOOLEAN_ARRAY)",
            TokenType::OperatorEq => "OPERATOR(EQ)",
            TokenType::OperatorNeq => "OPERATOR(NEQ)",
            TokenType::OperatorGt => "OPERATOR(GT)",
            TokenType::OperatorGte => "OPERATOR(GTE)",
            TokenType::OperatorLt => "OPERATOR(LT)",
            TokenType::OperatorLte => "OPERATOR(LTE)",
            TokenType::UnaryNot => "UNARY(NOT)",
            TokenType::OperatorAddInfix => "OPERATOR(ADD_INFIX)",
            TokenType::OperatorAddPrefix => "OPERATOR(ADD_PREFIX)",
            TokenType::UnaryIncrementPrefix => "UNARY(INCREMENT(PREFIX))",
            TokenType::UnaryIncrementPostfix => "UNARY(INCREMENT(POSTFIX))",
            TokenType::OperatorSubInfix => "OPERATOR(SUB_INFIX)",
            TokenType::OperatorSubPrefix => "OPERATOR(SUB_PREFIX)",
            TokenType::UnaryDecrementPrefix => "UNARY(DECREMENT(PREFIX))",
            TokenType::UnaryDecrementPostfix => "UNARY(DECREMENT(POSTFIX))",
            TokenType::OperatorMulInfix => "OPERATOR(MUL_INFIX)",
            TokenType::OperatorMulPrefix => "OPEREATOR(MUL_PREFIX)",
            TokenType::OperatorDivInfix => "OPERATOR(DIV_INFIX)",
            TokenType::OperatorDivPrefix => "OPERATOR(DIV_PREFIX)",
            TokenType::OperatorModInfix => "OPERATOR(MOD_INFIX)",
            TokenType::OperatorModPrefix => "OPERATOR(MOD_PREFIX)",
            TokenType::KeywordOr => "OR",
            TokenType::KeywordAnd => "AND",
            TokenType::KeywordConst => "CONST",
            TokenType::KeywordOf => "OF",
            TokenType::KeywordThen => "THEN",
            TokenType::KeywordIn => "IN",
            TokenType::KeywordFrom => "FROM",
            TokenType::KeywordTo => "TO",
            TokenType::KeywordStatementEnd => "STATEMENT(END)",
            TokenType::KeywordReturn => "RETURN",
            TokenType::IfClause => "IF",
            TokenType::ElseClause => "ELSE",
            TokenType::IfEndClause => "IF(END)",
            TokenType::WhileClause => "WHILE",
            TokenType::ForEveryClause => "FOREVERY",
        }
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub start: usize,
    pub length: usize,
    pub token_type: TokenType,
    pub value: String,
}

impl Token {
    pub fn new(
        start: usize,
        length: usize,
        token_type: TokenType,
        value: impl Into<String>,
    ) -> Self {
        Self {
            start,
            length,
            token_type,
            value: value.into(),
        }
    }

    pub fn append(&mut self, other: &Token) {
        self.value.push_str(&other.value);
        self.length += other.length;
    }
}
