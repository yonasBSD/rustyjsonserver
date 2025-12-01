use crate::rjscript::ast::position::Position;


#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Single-character tokens
    Plus,      // '+'
    Minus,     // '-'
    Star,      // '*'
    Slash,     // '/'
    Percent,   // '%'
    LParen,    // '('
    RParen,    // ')'
    LBrace,    // '{'
    RBrace,    // '}'
    LBracket,  // '['
    RBracket,  // ']'
    Colon,     // ':'
    Semicolon, // ';'
    Dot,       // '.'
    Comma,     // ','

    // One- or two-character tokens
    Eq,     // '='
    EqEq,   // '=='
    BangEq, // '!='
    Lt,     // '<'
    LtEq,   // '<='
    Gt,     // '>'
    GtEq,   // '>='
    AndAnd, // '&&'
    OrOr,   // '||'

    // Literals
    Number(f64),
    String(String), // "...."
    Template(String), // '`${var}`'
    Bool(bool),
    Undefined,

    //Types
    NumberType,
    BoolType,
    StringType,
    VecType,
    ObjType,
    AnyType,
    UndefinedType,

    // --- identifiers & keywords ---
    Ident(String),
    Let,    // 'let'
    Return, // 'return'
    If,     // 'if'
    Else,   // 'else'
    Switch,   // 'switch'
    Case,   // 'case'
    Default,   // 'default'
    For,   // 'for'
    Break, // 'break'
    Continue, // 'continue'
    Func,   // 'for'
    Req,    // 'req'
    Body,   // 'body'
    Params, // 'params'
    Query,  // 'query'
    Headers,  // 'headers'

    /// End‐of‐input sentinel
    EOF,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: Position,
}