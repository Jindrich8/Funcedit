use std::{
    iter::{self},
    ops::RangeInclusive,
};

use crate::{
    functions::function::{Func, StretchY, StretchYBounds, StretchYBoundsError},
    graph::{Graph, GraphFuncState},
    history::{
        history_stack::{
            entry_builder::{EntryBuilder, EntryBuilderError},
            shared_entry::{
                ApplyGraphOp, ApplyOp, ApplyOtherOp, InOp, OtherOp, OwnedOp, SharedOutOp,
            },
            HistoryError, IsGraphOpNonAltering,
        },
        History,
    },
    shared_op::{Delete, InsertPattern, InsertValues, MoveSelectBy, SharedOp},
    types::point::{X, Y},
};

use super::{ActionBuilderBase, ActiveFuncsIterSharedOp, GraphMutProvider};
pub struct BasicReversibleGraph<
    ActionGroupID: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionGroupID>,
> {
    graph: Graph,
    history: History<ActionGroupID, OpOwned, NonAlteringGraphOpHelper>,
}

impl<
        ActionGroupID: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionGroupID>,
    > BasicReversibleGraph<ActionGroupID, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        graph: Graph,
        history: History<ActionGroupID, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Self {
        Self { graph, history }
    }

    pub fn action<'a: 'b, 'b>(
        &'a mut self,
        id: impl Into<ActionGroupID>,
    ) -> ActionBuilder<'a, 'b, ActionGroupID, OpOwned, NonAlteringGraphOpHelper> {
        ActionBuilder {
            graph: &mut self.graph,
            history: ActionBuilderBase::new(self.history.add_entry(id.into())),
        }
    }

    pub fn history(&mut self) -> &History<ActionGroupID, OpOwned, NonAlteringGraphOpHelper> {
        &self.history
    }

    pub fn history_mut(
        &mut self,
    ) -> &mut History<ActionGroupID, OpOwned, NonAlteringGraphOpHelper> {
        &mut self.history
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn open_action<'a: 'b, 'b>(
        &'a mut self,
        id: impl Into<ActionGroupID>,
    ) -> ActionBuilder<'a, 'b, ActionGroupID, OpOwned, NonAlteringGraphOpHelper> {
        ActionBuilder {
            graph: &mut self.graph,
            history: ActionBuilderBase::new(self.history.open_entry(id.into())),
        }
    }

    pub fn close_action<'a>(&'a mut self, id: impl Into<ActionGroupID>) {
        self.history.close_entry(id.into());
    }
}
impl<
        ActionGroupID: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionGroupID>,
    > BasicReversibleGraph<ActionGroupID, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn undo<OpOut>(&mut self, mut apply_op: impl FnMut(ApplyOtherOp<OpOut>))
    where
        OpOwned: OwnedOp<OpOut>,
    {
        if let Some(undo) = self.history.undo_entry() {
            for op in undo.iter() {
                match op {
                    ApplyOp::Graph(op) => match op {
                        ApplyGraphOp::Undo(o) => self.graph.undo_op(&o),
                        ApplyGraphOp::Redo(o) => self.graph.redo_op(&o),
                    },
                    ApplyOp::Other(op) => apply_op(op),
                }
            }
        }
    }

    pub fn redo<OpOut>(&mut self, mut apply_op: impl FnMut(ApplyOtherOp<OpOut>))
    where
        OpOwned: OwnedOp<OpOut>,
    {
        if let Some(redo) = self.history.redo_entry() {
            for op in redo.iter() {
                match op {
                    ApplyOp::Graph(op) => match op {
                        ApplyGraphOp::Undo(o) => self.graph.undo_op(&o),
                        ApplyGraphOp::Redo(o) => self.graph.redo_op(&o),
                    },
                    ApplyOp::Other(op) => apply_op(op),
                }
            }
        }
    }
}

pub struct ActionBuilder<
    'a,
    'b,
    ActionGroupID: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionGroupID>,
> {
    graph: &'a mut Graph,
    history: ActionBuilderBase<'b, ActionGroupID, OpOwned, NonAlteringGraphOpHelper>,
}
impl<
        'a,
        'b,
        ActionGroupID: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionGroupID>,
    > ActionBuilder<'a, 'b, ActionGroupID, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn delete(&mut self) {
        self.history.delete(&mut self.graph);
    }

    pub fn insert_values<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        at: X,
        points: impl IntoIterator<Item = YExactIter> + Clone,
    ) {
        self.history.insert_values(at, points, &mut self.graph);
    }

    pub fn insert_pattern<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        at: X,
        points: impl IntoIterator<IntoIter = YExactIter, Item = Y> + Clone,
    ) {
        self.history.insert_pattern(at, points, &mut self.graph);
    }

    pub fn other(&mut self, other: impl InOp<OpOwned, ActionGroupID>) {
        self.history.other(other);
    }

    pub fn change_selection(&mut self, new_selection: impl Into<RangeInclusive<X>>) {
        self.history
            .change_selection(new_selection, &mut self.graph);
    }

    pub fn change_func_state(&mut self, index: usize, new_state: GraphFuncState) {
        self.history
            .change_func_state(index, new_state, &mut self.graph);
    }

    pub fn set_func_state_for_all(&mut self, new_state: GraphFuncState) {
        self.history
            .set_func_state_for_all(new_state, &mut self.graph);
    }

    pub fn change_each_active_func_state(
        &mut self,
        retain: impl FnMut(usize, &Func) -> GraphFuncState,
    ) {
        self.history
            .change_each_active_func_state(retain, &mut self.graph);
    }

    pub fn stretch_y_bounds(
        &mut self,
        bounds: &StretchYBounds,
    ) -> Result<StretchY, StretchYBoundsError> {
        self.history.stretch_y_bounds(bounds, &mut self.graph)
    }

    pub fn stretch_y_with_factor(&mut self, stretch: &StretchY) {
        self.history.stretch_y_with_factor(stretch, &mut self.graph);
    }
}
