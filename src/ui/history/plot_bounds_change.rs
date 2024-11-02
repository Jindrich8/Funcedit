pub mod change;

use std::{collections::VecDeque, ops::Add};

use change::PlotBoundsChange;
use egui_plot::PlotBounds;

use crate::{
    history::history_stack::shared_entry::{InOp, OpCombineErr, OpCreateErr},
    ui::ActionId,
    utils::Changeable,
};

use super::{ApplyDataOp, OwnedHistoryOp};

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

pub struct PlotBoundsChangeOp<'b> {
    change: &'b PlotBoundsChange,
}

impl<'b> PlotBoundsChangeOp<'b> {
    pub fn new(change: &'b PlotBoundsChange) -> Self {
        Self { change }
    }

    pub fn has_effect(&self) -> bool {
        self.change.has_effect()
    }
}

impl<'b> TryInto<OwnedHistoryOp> for PlotBoundsChangeOp<'b> {
    type Error = OpCreateErr;

    fn try_into(self) -> Result<OwnedHistoryOp, Self::Error> {
        if !self.has_effect() {
            Err(OpCreateErr::OpDoesNotHaveEffect)
        } else {
            Ok(OwnedHistoryOp::ChangePlotBounds(self.change.clone()))
        }
    }
}

impl<'b> InOp<OwnedHistoryOp, ActionId> for PlotBoundsChangeOp<'b> {
    fn try_combine(self, owned: &mut OwnedHistoryOp) -> Result<(), OpCombineErr<Self>> {
        match owned {
            OwnedHistoryOp::ChangePlotBounds(change) => {
                if self.has_effect() {
                    *change += self.change;
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
