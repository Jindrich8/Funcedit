use eframe::egui::Color32;

use crate::{
    graph::GraphFuncState,
    history::history_stack::{shared_entry::OtherOp, IsGraphOpNonAltering},
    reversible_graph::basic_reversible_graph::BasicReversibleGraph,
};

use super::{LegendEntries, LegendEntry};

pub trait LegendActionId: Clone + PartialEq + Default {
    fn change_active_funcs() -> Self;
}

pub struct SimpleLegendEntry {
    pub name: String,
    pub color: Color32,
    pub hovered: bool,
}

impl SimpleLegendEntry {
    pub fn new(name: String, color: Color32, hovered: bool) -> Self {
        Self {
            name,
            color,
            hovered,
        }
    }
}

pub struct SimpleLegendEntries<
    'a,
    ActionId: LegendActionId,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionId>,
> {
    graph: &'a mut BasicReversibleGraph<ActionId, OpOwned, NonAlteringGraphOpHelper>,
    entries: &'a mut [SimpleLegendEntry],
}

impl<
        'a,
        ActionId: LegendActionId,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionId>,
    > SimpleLegendEntries<'a, ActionId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        graph: &'a mut BasicReversibleGraph<ActionId, OpOwned, NonAlteringGraphOpHelper>,
        entries: &'a mut [SimpleLegendEntry],
    ) -> Self {
        Self { graph, entries }
    }
}

impl<
        'a,
        ActionId: LegendActionId,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<ActionId>,
    > LegendEntries for SimpleLegendEntries<'a, ActionId, OpOwned, NonAlteringGraphOpHelper>
{
    fn len(&self) -> usize {
        self.graph.graph().len()
    }

    fn get_name(&self, index: usize) -> Option<String> {
        self.entries.get(index).and_then(|e| Some(e.name.clone()))
    }

    fn get_color(&self, index: usize) -> Option<Color32> {
        self.entries.get(index).and_then(|e| Some(e.color))
    }

    fn get_hovered(&self, index: usize) -> Option<bool> {
        self.entries.get(index).and_then(|e| Some(e.hovered))
    }

    fn get_checked(&self, index: usize) -> Option<bool> {
        self.graph
            .graph()
            .get_func_state(index)
            .and_then(|s| Some(s == GraphFuncState::Active))
    }

    fn get_entry(&self, index: usize) -> Option<LegendEntry> {
        if let (Some(entry), Some(state)) = (
            self.entries.get(index),
            self.graph.graph().get_func_state(index),
        ) {
            Some(LegendEntry {
                name: entry.name.clone(),
                color: entry.color,
                checked: state == GraphFuncState::Active,
                hovered: entry.hovered,
            })
        } else {
            None
        }
    }

    fn iter_checked(&self) -> impl Iterator<Item = usize> {
        self.graph.graph().active_functions_index().map(|(_, i)| i)
    }

    fn iter_unchecked(&self) -> impl Iterator<Item = usize> {
        self.graph
            .graph()
            .inactive_functions_index()
            .map(|(_, i)| i)
    }

    fn set_hovered(&mut self, index: usize, hovered: bool) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.hovered = hovered;
        }
    }

    fn set_checked(&mut self, index: usize, checked: bool) {
        {
            self.graph
                .open_action(ActionId::change_active_funcs())
                .change_func_state(
                    index,
                    if checked {
                        GraphFuncState::Active
                    } else {
                        GraphFuncState::Inactive
                    },
                );
        }
    }

    fn check_all(&mut self) {
        {
            let mut binding = self.graph.open_action(ActionId::change_active_funcs());
            {
                binding.set_func_state_for_all(GraphFuncState::Active);
            }
        }
    }

    fn uncheck_all(&mut self) {
        self.graph
            .open_action(ActionId::change_active_funcs())
            .set_func_state_for_all(GraphFuncState::Inactive)
    }
}
