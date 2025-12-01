pub mod lints;
pub mod transforms;

use crate::rjscript::{ast::{block::Block, position::Position, stmt::Stmt}, preprocess::lints::error::LintError};

/// Result of preprocessing a parsed script.
/// If `errors` is non-empty, the caller should log and fail compilation.
pub struct PreprocessResult {
    pub errors: Vec<LintError>,
    pub stmts: Vec<Stmt>,
}

/// Preprocess when you have a Vec<Stmt>.
/// Returns (lint_messages, transformed_stmts).
pub fn preprocess(stmts: Vec<Stmt>) -> PreprocessResult {
    let mut block = Block::new(stmts, Position::UNKNOWN);

    // 2) Transforms (mutating)
    transforms::run_transforms(&mut block);

    let errors = lints::run_lints(&block);

    PreprocessResult {
        errors,
        stmts: std::mem::take(&mut block.stmts),
    }
}
