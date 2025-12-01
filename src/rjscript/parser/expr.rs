use crate::rjscript::ast::binop::BinOp;
use crate::rjscript::ast::expr::{Expr, ExprKind, TemplatePart};
use crate::rjscript::ast::literal::Literal;
use crate::rjscript::ast::node::Located;
use crate::rjscript::parser::errors::ParseError;
use crate::rjscript::parser::lexer::token::TokenKind;
use crate::rjscript::parser::parser::Parser;
use crate::rjscript::parser::ParseResult;

/// Operator precedences, from lowest to highest.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Assignment = 1,
    LogicalOr,
    LogicalAnd,
    Equality,
    Comparison,
    Term,
    Factor,
    Prefix, // unary operators
    Call,   // member, index, call
}

impl Precedence {
    fn next(self) -> Precedence {
        use Precedence::*;
        match self {
            Assignment => LogicalOr,
            LogicalOr => LogicalAnd,
            LogicalAnd => Equality,
            Equality => Comparison,
            Comparison => Term,
            Term => Factor,
            Factor => Prefix,
            Prefix => Call,
            Call => Call,
        }
    }
}

/// Return the binary-operator precedence
fn infix_precedence(tok: &TokenKind) -> Option<Precedence> {
    use TokenKind::*;
    Some(match tok {
        OrOr => Precedence::LogicalOr,
        AndAnd => Precedence::LogicalAnd,
        EqEq | BangEq => Precedence::Equality,
        Lt | LtEq | Gt | GtEq => Precedence::Comparison,
        Plus | Minus => Precedence::Term,
        Star | Slash | Percent => Precedence::Factor,
        Eq => Precedence::Assignment,
        _ => return None,
    })
}

/// Only `=` is right-associative in our grammar.
fn is_right_associative(tok: &TokenKind) -> bool {
    matches!(tok, TokenKind::Eq)
}

/// Parse a prefix expression: literals, variables, arrays, parens, unary minus, or `req.xxx` fields.
fn parse_prefix(parser: &mut Parser) -> ParseResult<Expr> {
    use TokenKind::*;
    // Otherwise, match standard prefix forms (including anonymous functions)
    match parser.peek_kind()?.clone() {
        Req => {
            parser.advance()?;
            parser.parse_req_access(parser.last_pos)
        }
        Number(n) => {
            parser.advance()?;
            Ok(Located::new(
                ExprKind::Literal(Literal::Number(n)),
                parser.last_pos,
            ))
        }
        // String literal
        String(s) => {
            parser.advance()?;
            Ok(Located::new(
                ExprKind::Literal(Literal::String(s)),
                parser.last_pos,
            ))
        }
        // Boolean literal
        Bool(b) => {
            parser.advance()?;
            Ok(Located::new(
                ExprKind::Literal(Literal::Bool(b)),
                parser.last_pos,
            ))
        }
        TokenKind::Template(raw) => {
            parser.advance()?; // consume the Template token
            let mut parts = Vec::new();
            let mut rest = raw.as_str();
            while let Some(idx) = rest.find("${") {
                // literal text before ${
                if idx > 0 {
                    parts.push(TemplatePart::Text(rest[..idx].to_string()));
                }
                // drop the `${`
                rest = &rest[idx + 2..];
                // find the closing `}`
                let end = rest.find('}').ok_or_else(|| {
                    ParseError::General("Unclosed ${ in template".into(), parser.last_pos)
                })?;
                let expr_src = &rest[..end];
                // parse that sub‐expression by re-lexing
                let mut subp = Parser::new(expr_src)?;
                let expr = parse_expr(&mut subp)?;
                parts.push(TemplatePart::Expr(expr));
                // advance past `}`
                rest = &rest[end + 1..];
            }
            // any trailing text
            if !rest.is_empty() {
                parts.push(TemplatePart::Text(rest.to_string()));
            }
            return Ok(Located::new(ExprKind::Template(parts), parser.last_pos));
        }
        // Undefined literal
        TokenKind::Undefined => {
            parser.advance()?;
            Ok(Located::new(
                ExprKind::Literal(Literal::Undefined),
                parser.last_pos,
            ))
        }
        // Array literal
        LBracket => {
            parser.advance()?;
            let mut elems = Vec::new();
            if !parser.match_kind(RBracket)? {
                loop {
                    elems.push(parse_expr(parser)?);
                    if parser.match_kind(RBracket)? {
                        break;
                    }
                    parser.expect_kind(Comma)?;
                }
            }
            Ok(Located::new(ExprKind::Array(elems), parser.last_pos))
        }
        // Object literal
        LBrace => {
            parser.advance()?; // consume '{'
            let mut fields = Vec::new();
            if !parser.match_kind(TokenKind::RBrace)? {
                loop {
                    // Parse key: either bare identifier or string literal
                    let key = match parser.peek_kind()?.clone() {
                        TokenKind::Ident(_) => parser.consume_ident()?,
                        TokenKind::String(s) => {
                            parser.advance()?;
                            s
                        }
                        _ => return Err(ParseError::ExpectedExpression(parser.last_pos)),
                    };
                    parser.expect_kind(TokenKind::Colon)?;
                    let value = parse_expr(parser)?;
                    fields.push((key, value));
                    if parser.match_kind(TokenKind::RBrace)? {
                        break;
                    }
                    parser.expect_kind(TokenKind::Comma)?;
                }
            }
            Ok(Located::new(
                ExprKind::ObjectLiteral { fields },
                parser.last_pos,
            ))
        }
        TokenKind::BoolType
        | TokenKind::NumberType
        | TokenKind::StringType
        | TokenKind::ObjType
        | TokenKind::VecType
        | TokenKind::AnyType
        | TokenKind::UndefinedType => {
            let ty = parser.parse_type()?;
            Ok(Located::new(ExprKind::TypeLiteral(ty), parser.last_pos))
        }
        // Variable or identifier
        Ident(_) => {
            let name = parser.consume_ident()?;
            Ok(Located::new(ExprKind::Ident(name), parser.last_pos))
        }
        // Grouping
        LParen => {
            parser.advance()?;
            let expr = parse_expr(parser)?;
            parser.expect_kind(RParen)?;
            Ok(expr)
        }
        // Unary minus: desugar to 0 - rhs
        Minus => {
            parser.advance()?;
            let rhs = parse_precedence(parser, Precedence::Prefix)?;
            Ok(Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Sub,
                    left: Box::new(Located::new(
                        ExprKind::Literal(Literal::Number(0.0)),
                        parser.last_pos,
                    )),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ))
        }
        // No match
        _ => Err(ParseError::ExpectedExpression(parser.last_pos)),
    }
}

fn parse_precedence(parser: &mut Parser, min_prec: Precedence) -> ParseResult<Expr> {
    // 1) Parse the left-hand side via prefix
    let mut left = parse_prefix(parser)?;

    // 2) Handle postfix operators (calls, indexing, member)
    loop {
        if parser.match_kind(TokenKind::Dot)? {
            let prop = parser.consume_ident()?;
            left = Located::new(
                ExprKind::Member {
                    object: Box::new(left),
                    property: prop,
                },
                parser.last_pos,
            );
            continue;
        }
        if parser.match_kind(TokenKind::LBracket)? {
            let idx = parse_expr(parser)?;
            parser.expect_kind(TokenKind::RBracket)?;
            left = Located::new(
                ExprKind::Index {
                    object: Box::new(left),
                    index: Box::new(idx),
                },
                parser.last_pos,
            );
            continue;
        }
        if parser.match_kind(TokenKind::LParen)? {
            let mut args = Vec::new();
            if !parser.match_kind(TokenKind::RParen)? {
                loop {
                    args.push(parse_expr(parser)?);
                    if parser.match_kind(TokenKind::RParen)? {
                        break;
                    }
                    parser.expect_kind(TokenKind::Comma)?;
                }
            }
            left = Located::new(
                ExprKind::Call {
                    callee: Box::new(left),
                    args,
                },
                parser.last_pos,
            );
            continue;
        }
        break;
    }

    // 3) Handle binary and assignment operators
    loop {
        let next = parser.peek_kind()?.clone();
        let prec = match infix_precedence(&next) {
            Some(p) if p >= min_prec => p,
            _ => break,
        };

        // Consume operator
        let op = parser.advance()?.kind;

        // Determine binding power for RHS
        let rhs_min = if is_right_associative(&op) {
            prec
        } else {
            prec.next()
        };
        let rhs = parse_precedence(parser, rhs_min)?;

        // Combine LHS and RHS
        left = match op {
            TokenKind::Eq => {
                // Move out of `left` to inspect the kind safely
                let (kind, _old_pos) = {
                    let Located { kind, pos } = left;
                    (kind, pos)
                };
                match kind {
                    ExprKind::Ident(name) => Located::new(
                        ExprKind::AssignVar {
                            name,
                            value: Box::new(rhs),
                        },
                        parser.last_pos,
                    ),
                    ExprKind::Member { object, property } => Located::new(
                        ExprKind::AssignMember {
                            object,
                            property,
                            value: Box::new(rhs),
                        },
                        parser.last_pos,
                    ),
                    ExprKind::Index { object, index } => Located::new(
                        ExprKind::AssignIndex {
                            object,
                            index,
                            value: Box::new(rhs),
                        },
                        parser.last_pos,
                    ),
                    ExprKind::RequestField(_) => {
                        return Err(ParseError::General(
                            "Cannot mutate request fields".into(),
                            parser.last_pos,
                        ))
                    }
                    _ => return Err(ParseError::InvalidAssignmentTarget(parser.last_pos)),
                }
            }
            TokenKind::Plus => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Add,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::Minus => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Sub,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::Star => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Mul,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::Slash => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Div,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::Percent => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Rem,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::EqEq => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Eq,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::BangEq => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Ne,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::Lt => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Lt,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::LtEq => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Le,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::Gt => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Gt,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::GtEq => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Ge,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::AndAnd => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::And,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            TokenKind::OrOr => Located::new(
                ExprKind::BinaryOp {
                    op: BinOp::Or,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                parser.last_pos,
            ),
            _ => unreachable!(),
        };
    }

    Ok(left)
}

pub fn parse_expr(parser: &mut Parser) -> ParseResult<Expr> {
    parse_precedence(parser, Precedence::Assignment)
}
