use crate::{eval::Eval, shared_table::*};

pub type TransTable = SharedHashTable<TransTableEntry>;

#[repr(packed)]
#[derive(Default, Clone, Copy)]
pub struct TransTableEntry {
    pub depth: u8,
    pub eval: Eval,
    pub next: chess::ChessMove,
    /// 2-bit node type
    pub flags: u8,
}

impl TransTableEntry {
    pub fn new_flags(nt: NodeType) -> u8 {
        nt as u8
    }

    pub fn node_type(&self) -> NodeType {
        match self.flags & 3 {
            0 => NodeType::Exact,
            1 => NodeType::UpperBound,
            2 => NodeType::LowerBound,
            _ => NodeType::None,
        }
    }
}

unsafe impl bytemuck::NoUninit for TransTableEntry {}

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, bytemuck::NoUninit, PartialEq, Eq)]
pub enum NodeType {
    #[default] Exact, UpperBound, LowerBound, None
}
