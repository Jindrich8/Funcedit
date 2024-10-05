pub mod change;

use std::{collections::VecDeque, ops::Add};

use change::PlotBoundsChange;
use egui_plot::PlotBounds;

use crate::{
    history::history_stack::shared_entry::{InOp, OpCombineErr, OpCreateErr},
    ui::ActionId,
    utils::Changeable,
};

use super::{ApplyDataOp, History, OwnedHistoryOp};

impl Changeable<PlotBoundsChange> for PlotBounds {
    fn get_change(&self, new_value: &Self) -> PlotBoundsChange {
        PlotBoundsChange::from_old_new(self, new_value)
    }

    fn change(&mut self, change: &PlotBoundsChange) {
        *self = change.undo_from(&self);
    }

    fn apply_change(&mut self, change: ApplyDataOp<&PlotBoundsChange>) {
        match change {
            ApplyDataOp::Undo(change) => self.change(change),
            ApplyDataOp::Redo(change) => {
                *self = change.redo_from(&self);
            }
        }
    }
}

pub struct PlotBoundsChangeOp<'a, 'b> {
    changes: &'a mut History,
    change: &'b PlotBoundsChange,
}

impl<'a, 'b> PlotBoundsChangeOp<'a, 'b> {
    pub fn new(changes: &'a mut History, change: &'b PlotBoundsChange) -> Self {
        Self { changes, change }
    }

    pub fn has_effect(&self) -> bool {
        self.change.has_effect()
    }
}

impl<'a, 'b> TryInto<OwnedHistoryOp> for PlotBoundsChangeOp<'a, 'b> {
    type Error = OpCreateErr;

    fn try_into(self) -> Result<OwnedHistoryOp, Self::Error> {
        if !self.has_effect() {
            Err(OpCreateErr::OpDoesNotHaveEffect)
        } else {
            self.changes.bounds_changes.push_back(self.change.clone());
            self.changes.op_pushed();
            Ok(OwnedHistoryOp::ChangePlotBounds)
        }
    }
}

impl<'a, 'b> InOp<OwnedHistoryOp, ActionId> for PlotBoundsChangeOp<'a, 'b> {
    fn try_combine(self, owned: &mut OwnedHistoryOp) -> Result<(), OpCombineErr<Self>> {
        match owned {
            OwnedHistoryOp::ChangePlotBounds => {
                if self.has_effect() {
                    *self.changes.bounds_changes.back_mut().unwrap() += self.change;
                    Ok(())
                } else {
                    Err(OpCombineErr::OpDoesNotHaveEffect)
                }
            }
        }
    }

    fn alters_history(&self, id: &ActionId) -> bool {
        *id != ActionId::Conditions
    }
}
