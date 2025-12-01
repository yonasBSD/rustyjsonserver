use crate::rjscript::{ast::{block::Block, position::Position}, parser::{lexer::token::TokenKind, parser::Parser, stmt::parse_stmt, ParseResult}};

pub fn parse_block(parser: &mut Parser) -> ParseResult<Block> {
    parser.expect_kind(TokenKind::LBrace)?;
    let block_start: Position = parser.last_pos;
    let mut statements = Vec::new();
    while !parser.is_at_end()? {
        // If next is '}', consume and break
        if parser.match_kind(TokenKind::RBrace)? {
            break;
        }
        let st = parse_stmt(parser, false)?;
        statements.push(st);
        let _ = parser.match_kind(TokenKind::Semicolon);
    }
    Ok(Block::new(statements, block_start))
}