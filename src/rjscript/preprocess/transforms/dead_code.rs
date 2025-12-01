use crate::rjscript::ast::{
    block::Block,
    stmt::{Stmt, StmtKind},
    visitor::{walk_stmt_mut, VisitMut},
};

pub struct DeadCodeStrip;

impl VisitMut for DeadCodeStrip {
    fn visit_block_mut(&mut self, b: &mut Block) {
        // First recurse inside statements
        for s in &mut b.stmts {
            walk_stmt_mut(self, s);
        }

        // does this statement guarantee a return/termination?
        fn stmt_terminates(s: &Stmt) -> bool {
            fn block_must_return(b: &Block) -> bool {
                if b.stmts.is_empty() {
                    return false;
                }
                for (i, s) in b.stmts.iter().enumerate() {
                    let last = i == b.stmts.len() - 1;
                    match &s.kind {
                        StmtKind::Return(_) | StmtKind::ReturnStatus { .. } => return true,
                        StmtKind::IfElse {
                            then_block,
                            else_block,
                            ..
                        } => {
                            let then_r = block_must_return(then_block);
                            let else_r =
                                else_block.as_ref().map(block_must_return).unwrap_or(false);
                            if then_r && else_r {
                                return true;
                            }
                            if !last {
                                continue;
                            } else {
                                return false;
                            }
                        }
                        StmtKind::Switch { cases, default, .. } => {
                            let all_cases_return =
                                cases.iter().all(|(_, blk)| block_must_return(blk));
                            let default_returns =
                                default.as_ref().map(block_must_return).unwrap_or(false);
                            if all_cases_return && default_returns {
                                return true;
                            }
                            if !last {
                                continue;
                            } else {
                                return false;
                            }
                        }
                        StmtKind::For { .. } => {
                            if !last {
                                continue;
                            } else {
                                return false;
                            }
                        }
                        _ => {
                            if !last {
                                continue;
                            } else {
                                return false;
                            }
                        }
                    }
                }
                false
            }

            match &s.kind {
                StmtKind::Return(_) | StmtKind::ReturnStatus { .. } => true,
                StmtKind::IfElse {
                    then_block,
                    else_block,
                    ..
                } => {
                    block_must_return(then_block)
                        && else_block.as_ref().map(block_must_return).unwrap_or(false)
                }
                StmtKind::Switch { cases, default, .. } => {
                    let all_cases_return = cases.iter().all(|(_, blk)| block_must_return(blk));
                    let default_returns = default.as_ref().map(block_must_return).unwrap_or(false);
                    all_cases_return && default_returns
                }
                // Break/Continue only terminate a loop body; we keep prior behavior and
                // treat them as terminating within the current block (same as before).
                StmtKind::Break | StmtKind::Continue => true,
                _ => false,
            }
        }

        // drop statements after the first terminating one
        let mut out = Vec::with_capacity(b.stmts.len());
        let mut live = true;
        for s in std::mem::take(&mut b.stmts) {
            if live {
                let terminates = stmt_terminates(&s);
                out.push(s);
                if terminates {
                    live = false;
                }
            }
        }
        b.stmts = out;
    }
}
