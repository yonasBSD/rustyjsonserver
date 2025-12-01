pub mod error;
pub mod must_return;
pub mod type_assign;
pub mod req_type_guard;
pub mod definite_assign;
pub mod req_imutability;
pub mod declarations;
pub mod unknown_calls;
pub mod util;

use crate::rjscript::{ast::block::Block, preprocess::lints::error::LintError};

/// Returns a flat list of error strings (empty if OK).
pub fn run_lints(block: &Block) -> Vec<LintError> {
    let mut errs = Vec::new();

    errs.extend(must_return::run(block));
    errs.extend(type_assign::run(block));
    errs.extend(req_imutability::run(block));
    errs.extend(req_type_guard::run(block));
    errs.extend(definite_assign::run(block));
    errs.extend(declarations::run(block));
    errs.extend(unknown_calls::run(block));

    errs.sort();
    errs
}
