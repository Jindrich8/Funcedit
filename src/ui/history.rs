pub mod plot_bounds_change;

use std::{
    collections::{vec_deque, VecDeque},
    fmt::Debug,
};

use plot_bounds_change::{change::PlotBoundsChange, PlotBoundsChangeOp};

use crate::{
    history::history_stack::shared_entry::{ApplyOtherOp, OtherOp, OwnedOp},
    utils::Change,
};

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedHistoryOp {
    ChangePlotBounds(PlotBoundsChange),
}

impl OtherOp for OwnedHistoryOp {}

impl OwnedOp<SharedHistoryOp> for OwnedHistoryOp {
    fn get_shared(&self) -> SharedHistoryOp {
        match self {
            Self::ChangePlotBounds(u) => SharedHistoryOp::ChangePlotBounds(u.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SharedHistoryOp {
    ChangePlotBounds(PlotBoundsChange),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SharedDataOp<'a> {
    ChangePlotBounds(ApplyDataOp<&'a PlotBoundsChange>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApplyDataOp<Other: Debug + Clone + PartialEq> {
    Undo(Other),
    Redo(Other),
}

impl<Other: Debug + Clone + PartialEq> ApplyDataOp<Other> {
    pub fn new<'a, FromOther: 'a>(
        op: &'a ApplyOtherOp<FromOther>,
        map: impl FnOnce(&'a FromOther) -> Other + 'a,
    ) -> Self {
        match op {
            ApplyOtherOp::Undo(op) => Self::Undo(map(op)),
            ApplyOtherOp::Redo(op) => Self::Redo(map(op)),
        }
    }

    pub fn new_opt<'a, FromOther>(
        op: &'a ApplyOtherOp<FromOther>,
        map: impl FnOnce(&'a FromOther) -> Option<Other> + 'a,
    ) -> Option<Self> {
        match op {
            ApplyOtherOp::Undo(op) => map(op).and_then(|op| Some(Self::Undo(op))),
            ApplyOtherOp::Redo(op) => map(op).and_then(|op| Some(Self::Redo(op))),
        }
    }
}
