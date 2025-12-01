use crate::rjscript::ast::{block::Block, visitor::VisitMut};

pub mod dead_code;

pub fn run_transforms(block: &mut Block) {
    dead_code::DeadCodeStrip.visit_block_mut(block);
}