use crate::rjscript::{
    ast::{
        block::Block,
        expr::ExprKind,
        literal::Literal,
        node::Located,
        stmt::{Stmt, StmtKind},
    }, parser::{
        block::parse_block, errors::ParseError, expr::parse_expr, lexer::token::TokenKind,
        parser::Parser, ParseResult,
    }, semantics::{methods::builtin_names_set, types::VarType}
};

fn parse_function_decl(parser: &mut Parser) -> ParseResult<Stmt> {
    parser.advance()?;
    let ident = parser.consume_ident()?;

    if builtin_names_set().contains(ident.as_str()) {
        return Err(ParseError::General(
            format!(
                "{} is a reserved function name",
                ident
            ),
            parser.last_pos,
        ));
    }

    parser.expect_kind(TokenKind::LParen)?;
    let mut params = Vec::new();
    if !parser.match_kind(TokenKind::RParen)? {
        loop {
            let name = parser.consume_ident()?;
            parser.expect_kind(TokenKind::Colon)?;
            let ty: VarType = parser.parse_assignment_type()?;
            params.push((name, ty));
            if parser.match_kind(TokenKind::RParen)? {
                break;
            }
            parser.expect_kind(TokenKind::Comma)?;
        }
    }
    // must have return type annotation
    parser.expect_kind(TokenKind::Colon)?;
    let return_type = parser.parse_assignment_type()?;
    let body = parse_block(parser)?;
    Ok(Located::new(
        StmtKind::FunctionDecl {
            ident,
            params,
            return_type,
            body,
        },
        parser.last_pos,
    ))
}

fn parse_switch_stmt(parser: &mut Parser) -> ParseResult<Stmt> {
    let switch_start_pos = parser.last_pos;

    parser.advance()?; // consume 'switch'
    parser.expect_kind(TokenKind::LParen)?;
    let discr = parse_expr(parser)?;
    parser.expect_kind(TokenKind::RParen)?;
    parser.expect_kind(TokenKind::LBrace)?;

    let mut cases = Vec::new();
    let mut default = None;

    while !parser.match_kind(TokenKind::RBrace)? {
        if parser.match_kind(TokenKind::Case)? {
            let case_expr = parse_expr(parser)?;
            parser.expect_kind(TokenKind::Colon)?;
            // collect statements until next case/default/}
            let mut stmts = Vec::new();
            while !matches!(
                parser.peek_kind()?,
                TokenKind::Case | TokenKind::Default | TokenKind::RBrace
            ) {
                stmts.push(parse_stmt(parser, false)?);
            }
            cases.push((case_expr, Block::new(stmts, switch_start_pos)));
        } else if parser.match_kind(TokenKind::Default)? {
            parser.expect_kind(TokenKind::Colon)?;
            let mut stmts = Vec::new();
            while !matches!(parser.peek_kind()?, TokenKind::Case | TokenKind::RBrace) {
                stmts.push(parse_stmt(parser, false)?);
            }
            default = Some(Block::new(stmts, switch_start_pos));
        } else {
            return Err(ParseError::General(
                format!(
                    "Expected 'case' or 'default', found {:?}",
                    parser.peek_kind()?
                ),
                parser.last_pos,
            ));
        }
    }

    Ok(Located::new(
        StmtKind::Switch {
            condition: discr,
            cases,
            default,
        },
        switch_start_pos,
    ))
}

/// Parses an `if (…) { … } [ else if (…) { … } ]* [ else { … } ]` chain
fn parse_if_stmt(parser: &mut Parser) -> ParseResult<Stmt> {
    // the initial `if`
    parser.expect_kind(TokenKind::If)?; // consume 'if'
    let if_start_pos = parser.last_pos;

    parser.expect_kind(TokenKind::LParen)?;
    let cond = parse_expr(parser)?;
    parser.expect_kind(TokenKind::RParen)?;
    let then_block = parse_block(parser)?;

    // check for `else`
    if parser.match_kind(TokenKind::Else)? {
        if *parser.peek_kind()? == TokenKind::If {
            // else‑if: recursively parse the nested `if`
            let nested = parse_if_stmt(parser)?;
            let else_block = Block::new(vec![nested], if_start_pos);
            Ok(Located::new(
                StmtKind::IfElse {
                    condition: cond,
                    then_block,
                    else_block: Some(else_block),
                },
                parser.last_pos,
            ))
        } else {
            // final else
            let else_block = parse_block(parser)?;
            Ok(Located::new(
                StmtKind::IfElse {
                    condition: cond,
                    then_block,
                    else_block: Some(else_block),
                },
                parser.last_pos,
            ))
        }
    } else {
        // no else
        Ok(Located::new(
            StmtKind::IfElse {
                condition: cond,
                then_block,
                else_block: None,
            },
            parser.last_pos,
        ))
    }
}

pub fn parse_stmt(parser: &mut Parser, is_top_level: bool) -> ParseResult<Stmt> {
    match parser.peek_kind()? {
        TokenKind::Let => {
            // let x = <expr>
            parser.advance()?; // consume 'let'
            let name = parser.consume_ident()?;
            parser.expect_kind(TokenKind::Colon)?;
            let var_type = parser.parse_assignment_type()?;
            let initializer = if parser.match_kind(TokenKind::Eq)? {
                let e = parse_expr(parser)?;
                parser.expect_kind(TokenKind::Semicolon)?;
                Some(e)
            } else {
                parser.expect_kind(TokenKind::Semicolon)?;
                None
            };
            Ok(Located::new(
                StmtKind::Let {
                    name,
                    ty: var_type,
                    init: initializer,
                },
                parser.last_pos,
            ))
        }

        TokenKind::Return => {
            // return <expr> / return <expr>, <expr>
            // consume 'return'
            parser.advance()?;
            let first = parse_expr(parser)?;
            if parser.match_kind(TokenKind::Comma)? {
                let second = parse_expr(parser)?;
                parser.expect_kind(TokenKind::Semicolon)?;
                Ok(Located::new(
                    StmtKind::ReturnStatus {
                        status: first,
                        value: second,
                    },
                    parser.last_pos,
                ))
            } else {
                parser.expect_kind(TokenKind::Semicolon)?;
                Ok(Located::new(StmtKind::Return(first), parser.last_pos))
            }
        }

        TokenKind::If => parse_if_stmt(parser),
        TokenKind::Switch => parse_switch_stmt(parser),
        TokenKind::For => {
            parser.advance()?; // eat 'for'
            parser.expect_kind(TokenKind::LParen)?;

            // — initializer —
            let init = if parser.match_kind(TokenKind::Semicolon)? {
                None
            } else {
                // let-stmt or expr-stmt
                let stmt = if *parser.peek_kind()? == TokenKind::Let {
                    let s = parse_stmt(parser, false)?;
                    s
                } else {
                    let e = parse_expr(parser)?;
                    parser.expect_kind(TokenKind::Semicolon)?;
                    Located::new(StmtKind::ExprStmt(e), parser.last_pos)
                };
                Some(Box::new(stmt))
            };

            // — condition —
            let condition = if parser.match_kind(TokenKind::Semicolon)? {
                // default to true
                Located::new(ExprKind::Literal(Literal::Bool(true)), parser.last_pos)
            } else {
                let cond = parse_expr(parser)?;
                parser.expect_kind(TokenKind::Semicolon)?;
                cond
            };

            // — increment —
            let increment = if parser.match_kind(TokenKind::RParen)? {
                None
            } else {
                let inc = parse_expr(parser)?;
                parser.expect_kind(TokenKind::RParen)?;
                Some(inc)
            };

            // — body: either a block or a single stmt —
            let body = if *parser.peek_kind()? == TokenKind::LBrace {
                parse_block(parser)?
            } else {
                let s = parse_stmt(parser, false)?;
                Block::new(vec![s], parser.last_pos)
            };

            Ok(Located::new(
                StmtKind::For {
                    init,
                    condition,
                    increment,
                    body,
                },
                parser.last_pos,
            ))
        }
        TokenKind::Func => {
            // funcs only on toplevel
            if is_top_level {
                parse_function_decl(parser)
            } else {
                Err(ParseError::General(
                    format!("Functions can only be declared at top level"),
                    parser.last_pos,
                ))
            }
        }

        TokenKind::Break => {
            parser.advance()?;
            parser.expect_kind(TokenKind::Semicolon)?;
            Ok(Located::new(StmtKind::Break, parser.last_pos))
        }

        TokenKind::Continue => {
            parser.advance()?;
            parser.expect_kind(TokenKind::Semicolon)?;
            Ok(Located::new(StmtKind::Continue, parser.last_pos))
        }

        // Otherwise, it must be an expression‐statement (e.g. starting with '(' or a literal)
        _ => {
            let e = parse_expr(parser)?;
            parser.expect_kind(TokenKind::Semicolon)?;
            Ok(Located::new(StmtKind::ExprStmt(e), parser.last_pos))
        }
    }
}
