pub mod basic_reversible_graph;

use std::{
    iter::{self},
    ops::RangeInclusive,
};

use crate::{
    functions::function::{Func, StretchY, StretchYBounds, StretchYBoundsError},
    graph::{Graph, GraphFuncState},
    history::{
        history_stack::{
            entry_builder::EntryBuilder,
            shared_entry::{InOp, OtherOp, OwnedOp},
            IsGraphOpNonAltering,
        },
        History,
    },
    shared_op::{Delete, InsertPattern, InsertValues, MoveSelectBy, SharedOp},
    types::point::{X, Y},
};

pub trait GraphMutProvider {
    fn graph_mut(&mut self) -> &mut Graph;
}

impl GraphMutProvider for Graph {
    fn graph_mut(&mut self) -> &mut Graph {
        self
    }
}

pub type ActiveFuncsIterSharedOp<Iter> =
    SharedOp<Iter, iter::Empty<iter::Empty<Y>>, iter::Empty<Y>>;
pub type NoIterSharedOp = ActiveFuncsIterSharedOp<std::iter::Empty<usize>>;

pub struct ActionBuilderBase<
    'a,
    ActionGroupID: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionGroupID>,
> {
    history: EntryBuilder<'a, ActionGroupID, OpOwned, NonAlteringGraphOpHelper>,
}

impl<
        'a,
        ActionGroupID: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionGroupID>,
    > ActionBuilderBase<'a, ActionGroupID, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        history: EntryBuilder<'a, ActionGroupID, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Self {
        Self { history }
    }

    pub fn delete(&mut self, graph: &mut Graph) {
        self.history
            .add_graph_op(SharedOp::<iter::Empty<usize>, _, _>::Delete(Delete(
                graph.selection_points(),
            )));
        graph.delete();
    }

    pub fn insert_values<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        at: X,
        points: impl IntoIterator<Item = YExactIter> + Clone,
        graph: &mut Graph,
    ) {
        self.history
            .add_graph_op(SharedOp::<iter::Empty<usize>, _, _>::InsertValues(
                InsertValues {
                    x: at,
                    values: points.clone().into_iter(),
                },
            ));
        graph.insert_values(at, points)
    }

    pub fn insert_pattern<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        at: X,
        points: impl IntoIterator<IntoIter = YExactIter, Item = Y> + Clone,
        graph: &mut Graph,
    ) {
        self.history.add_graph_op(
            SharedOp::<iter::Empty<usize>, iter::Empty<YExactIter>, _>::InsertPattern(
                InsertPattern {
                    x: at,
                    values: points.clone().into_iter(),
                },
            ),
        );
        graph.insert_pattern(at, points)
    }

    pub fn other(&mut self, other: impl InOp<OpOwned, ActionGroupID>) {
        self.history.add_other_op(other);
    }

    pub fn change_selection(
        &mut self,
        new_selection: impl Into<RangeInclusive<X>>,
        graph: &mut Graph,
    ) {
        let new_selection = new_selection.into();
        let selection = graph.selection().clone();
        if selection != new_selection {
            graph.change_selection(new_selection.clone());

            self.history
                .add_graph_op(NoIterSharedOp::MoveSelectBy(MoveSelectBy {
                    start_by: new_selection.start() - selection.start(),
                    end_by: new_selection.end() - selection.end(),
                }));
        }
    }

    pub fn change_func_state(
        &mut self,
        index: usize,
        new_state: GraphFuncState,
        graph: &mut Graph,
    ) {
        if let Some(prev_state) = graph.change_func_state(index, new_state) {
            if new_state != prev_state {
                self.history
                    .add_graph_op(ActiveFuncsIterSharedOp::ChangeActiveFuncs(
                        [index].into_iter(),
                    ));
            };
        }
    }

    pub fn set_func_state_for_all(&mut self, new_state: GraphFuncState, graph: &mut Graph) {
        match new_state {
            GraphFuncState::Active => {
                self.history
                    .add_graph_op(ActiveFuncsIterSharedOp::ChangeActiveFuncs(
                        graph.inactive_func_indexes(),
                    ))
            }
            GraphFuncState::Inactive => {
                self.history
                    .add_graph_op(ActiveFuncsIterSharedOp::ChangeActiveFuncs(
                        graph.active_func_indexes(),
                    ))
            }
        };
        graph.set_func_state_for_all(new_state);
    }

    pub fn change_each_active_func_state(
        &mut self,
        mut retain: impl FnMut(usize, &Func) -> GraphFuncState,
        graph: &mut Graph,
    ) {
        graph.change_each_active_func_state(|fi, f| {
            let new_state = retain(fi, f);
            match new_state {
                GraphFuncState::Active => (),
                GraphFuncState::Inactive => {
                    self.history
                        .add_graph_op(ActiveFuncsIterSharedOp::ChangeActiveFuncs([fi].into_iter()));
                }
            }

            new_state
        });
    }

    pub fn stretch_y_bounds(
        &mut self,
        bounds: &StretchYBounds,
        graph: &mut Graph,
    ) -> Result<StretchY, StretchYBoundsError> {
        match graph.stretch_y(&bounds) {
            Ok(stretch) => {
                if stretch.stretches() {
                    self.history
                        .add_graph_op(NoIterSharedOp::StretchY(stretch.clone()));
                }
                Ok(stretch)
            }
            Err(e) => Err(e),
        }
    }

    pub fn stretch_y_with_factor(&mut self, stretch: &StretchY, graph: &mut Graph) {
        if graph.stretch_y_with_factor(stretch) {
            self.history
                .add_graph_op(NoIterSharedOp::StretchY(stretch.clone()));
        }
    }
}
