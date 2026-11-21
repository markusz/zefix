//! Type-checked, compiled form of a query. Card sets are pre-resolved to bitmasks.

use super::ast::CmpOp;
use crate::cards::Cards;
use crate::game::Seat;

/// Maximum quantifier nesting depth.
pub const MAX_VARS: usize = 8;

#[derive(Clone, Debug)]
pub enum SetExpr {
    Const(Cards),
    /// Cards of the suit bound to this variable slot.
    SuitVar(u8),
    Union(Box<SetExpr>, Box<SetExpr>),
    Inter(Box<SetExpr>, Box<SetExpr>),
    Diff(Box<SetExpr>, Box<SetExpr>),
}

#[derive(Clone, Copy, Debug)]
pub enum PlayerRef {
    Seat(Seat),
    Partner,
    Var(u8),
}

#[derive(Clone, Debug)]
pub enum IntExpr {
    Const(i64),
    Count(PlayerRef, SetExpr),
    Add(Box<IntExpr>, Box<IntExpr>),
    Sub(Box<IntExpr>, Box<IntExpr>),
    Num(Box<Quant>),
}

#[derive(Clone, Debug)]
pub enum BoolExpr {
    And(Box<BoolExpr>, Box<BoolExpr>),
    Or(Box<BoolExpr>, Box<BoolExpr>),
    Not(Box<BoolExpr>),
    Cmp(CmpOp, IntExpr, IntExpr),
    HasAll(PlayerRef, SetExpr),
    HasAny(PlayerRef, SetExpr),
    Quant(BoolQuant, Box<Quant>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolQuant {
    Any,
    All,
    NoneOf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    Others,
    Opponents,
    Players,
    Farben,
    Suits,
}

#[derive(Clone, Debug)]
pub struct Quant {
    pub slot: u8,
    pub domain: Domain,
    pub body: BoolExpr,
}
