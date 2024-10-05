pub mod hist_store;
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

pub struct History {
    bounds_changes: VecDeque<PlotBoundsChange>,
}

impl History {
    pub fn new() -> Self {
        Self {
            bounds_changes: VecDeque::new(),
        }
    }

    pub fn bounds_change<'a>(&'a mut self, change: &'a PlotBoundsChange) -> PlotBoundsChangeOp {
        PlotBoundsChangeOp::new(self, change)
    }

    pub fn len(&self) -> usize {
        self.bounds_changes.len()
    }

    fn op_pushed(&mut self) {
        self.bounds_changes.drain(self.undo_len..(self.len() - 1));
    }

    pub fn undo(&mut self, op: ApplyOtherOp<SharedHistoryOp>) -> Option<SharedDataOp> {
        match self.undo_len.checked_sub(1) {
            Some(new_len) => {
                self.undo_len = new_len;
                self.get_data_op(op, new_len)
            }
            None => None,
        }
    }

    pub fn redo(&mut self, op: ApplyOtherOp<SharedHistoryOp>) -> Option<SharedDataOp> {
        if self.undo_len < self.len() {
            let index = self.undo_len;
            self.undo_len += 1;
            self.get_data_op(op, index)
        } else {
            None
        }
    }

    fn get_data_op(&self, op: ApplyOtherOp<SharedHistoryOp>, index: usize) -> Option<SharedDataOp> {
        let map = |op| match op {
            ApplyOtherOp::Undo(SharedHistoryOp::ChangePlotBounds) => self
                .bounds_changes
                .get(index)
                .and_then(|change| Some(SharedDataOp::ChangePlotBounds(ApplyDataOp::Undo(change)))),
            ApplyOtherOp::Redo(SharedHistoryOp::ChangePlotBounds) => self
                .bounds_changes
                .get(index)
                .and_then(|change| Some(SharedDataOp::ChangePlotBounds(ApplyDataOp::Redo(change)))),
        };

        map(op)
    }

    pub fn iter<'a>(&'a self) -> HistoryIter<'a> {
        HistoryIter {
            undo_len: self.undo_len,
            history: self,
        }
    }
}

pub struct HistoryIter<'a> {
    history: &'a History,
    undo_len: usize,
}

impl<'a> HistoryIter<'a> {
    pub fn next_redo(&mut self, op: ApplyOtherOp<SharedHistoryOp>) -> Option<SharedDataOp> {
        if self.undo_len < self.history.len() {
            let undo_len = self.undo_len;
            self.undo_len += 1;
            self.history.get_data_op(op, undo_len)
        } else {
            None
        }
    }

    pub fn next_undo(&mut self, op: ApplyOtherOp<SharedHistoryOp>) -> Option<SharedDataOp> {
        if self.undo_len > 0 {
            self.undo_len -= 1;
            self.history.get_data_op(op, self.undo_len)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedHistoryOp {
    ChangePlotBounds(usize),
}

impl OtherOp for OwnedHistoryOp {}

impl OwnedOp<SharedHistoryOp> for OwnedHistoryOp {
    fn get_shared(&self) -> SharedHistoryOp {
        match self {
            Self::ChangePlotBounds(u) => SharedHistoryOp::ChangePlotBounds(*u),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SharedHistoryOp {
    ChangePlotBounds(usize),
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
    pub fn new<FromOther>(
        op: ApplyOtherOp<FromOther>,
        map: impl FnOnce(FromOther) -> Other,
    ) -> Self {
        match op {
            ApplyOtherOp::Undo(op) => Self::Undo(map(op)),
            ApplyOtherOp::Redo(op) => Self::Redo(map(op)),
        }
    }

    pub fn new_opt<FromOther>(
        op: ApplyOtherOp<FromOther>,
        map: impl FnOnce(FromOther) -> Option<Other>,
    ) -> Option<Self> {
        match op {
            ApplyOtherOp::Undo(op) => map(op).and_then(|op| Some(Self::Undo(op))),
            ApplyOtherOp::Redo(op) => map(op).and_then(|op| Some(Self::Redo(op))),
        }
    }
}
