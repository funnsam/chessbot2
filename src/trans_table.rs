use crate::{eval::Eval, node::NodeType, shared_table::*};

pub type TransTable = SharedHashTable<TransTableEntry>;

#[repr(packed)]
#[derive(Default, Clone, Copy)]
pub struct TransTableEntry {
    pub depth: u8,
    pub eval: Eval,
    pub next: dychess::chess_move::Move,
    /// 2-bit node type
    pub flags: u8,
}

impl TransTableEntry {
    pub fn new_flags(nt: NodeType) -> u8 {
        nt as u8
    }

    pub fn node_type(&self) -> NodeType {
        match self.flags & 3 {
            0 => NodeType::Pv,
            1 => NodeType::All,
            2 => NodeType::Cut,
            _ => NodeType::None,
        }
    }
}

unsafe impl bytemuck::NoUninit for TransTableEntry {}
