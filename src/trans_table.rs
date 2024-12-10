use crate::{eval::Eval, shared_table::*};

pub type TransTable = SharedHashTable<TransTableEntry>;

#[repr(packed)]
#[derive(Default, Clone, Copy)]
pub struct TransTableEntry {
    pub depth: u8,
    pub eval: Eval,
    pub node_type: NodeType,
    pub next: chess::ChessMove,
}

unsafe impl bytemuck::NoUninit for TransTableEntry {}

#[repr(u8)]
#[derive(Default, Clone, Copy, bytemuck::NoUninit, PartialEq, Eq)]
pub enum NodeType {
    #[default] Exact, UpperBound, LowerBound, None
}
