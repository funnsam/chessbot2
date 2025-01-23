#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, bytemuck::NoUninit, PartialEq, Eq)]
pub enum NodeType {
    #[default]
    #[doc(alias = "Exact")]
    Pv,
    #[doc(alias = "UpperBound")]
    All,
    #[doc(alias = "LowerBound")]
    Cut,
    None,
}

pub trait Node {
    type Zw: Node;

    const NODE: NodeType;
    const PV: bool;
}

pub struct Pv;
impl Node for Pv {
    type Zw = Cut;

    const NODE: NodeType = NodeType::Pv;
    const PV: bool = true;
}

pub struct Cut;
impl Node for Cut {
    type Zw = All;

    const NODE: NodeType = NodeType::Cut;
    const PV: bool = false;
}

pub struct All;
impl Node for All {
    type Zw = Cut;

    const NODE: NodeType = NodeType::All;
    const PV: bool = false;
}
