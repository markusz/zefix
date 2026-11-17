use super::Span;

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    Int(i64),
    Name(String),
    Not(Box<Node>),
    Binary {
        op: BinOp,
        lhs: Box<Node>,
        rhs: Box<Node>,
    },
    Call {
        name: String,
    },
    Method(MethodNode),
    Quant(QuantNode),
}

#[derive(Clone, Debug)]
pub struct MethodNode {
    pub recv: Box<Node>,
    pub name: String,
    pub name_span: Span,
    pub args: Vec<Node>,
}

/// `kind(var in domain, body)`
#[derive(Clone, Debug)]
pub struct QuantNode {
    pub kind: QuantKind,
    pub var: String,
    pub var_span: Span,
    pub domain: String,
    pub domain_span: Span,
    pub body: Box<Node>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Cmp(CmpOp),
    Add,
    Sub,
    Union,
    Inter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    pub fn apply(self, a: i64, b: i64) -> bool {
        match self {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantKind {
    Any,
    All,
    NoneOf,
    Num,
}

impl QuantKind {
    pub fn from_keyword(s: &str) -> Option<QuantKind> {
        match s {
            "any" => Some(QuantKind::Any),
            "all" => Some(QuantKind::All),
            "none" => Some(QuantKind::NoneOf),
            "num" => Some(QuantKind::Num),
            _ => None,
        }
    }
}

pub const KEYWORDS: [&str; 8] = ["and", "or", "not", "in", "any", "all", "none", "num"];
