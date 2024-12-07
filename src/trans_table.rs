use crate::{eval::Eval, shared_table::*};

pub type TransTable = SharedHashTable<TransTableEntry>;

#[repr(C, packed)]
#[derive(Default, Clone, Copy, bytemuck::NoUninit)]
pub struct TransTableEntry {
    pub depth: u8,
    pub eval: Eval,
    pub node_type: NodeType,
}

#[repr(u8)]
#[derive(Default, Clone, Copy, bytemuck::NoUninit, PartialEq, Eq)]
pub enum NodeType {
    #[default] Exact, UpperBound, LowerBound, None
}
