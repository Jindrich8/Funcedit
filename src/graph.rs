use std::*;

use ops::RangeInclusive;

use crate::{
    functions::function::{
        func_values_check_iter::FuncValuesCheckIter, Func, FuncYValuesIter, StretchY,
        StretchYBounds, StretchYBoundsError,
    },
    shared_op::SharedOp,
    types::{
        bit_set::{self, BitSet},
        point::{X, Y},
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphFuncState {
    Active,
    Inactive,
}

pub struct SelectionValuesIterator<'a> {
    active_funcs: ActiveFuncsIter<'a>,
}

impl<'a> SelectionValuesIterator<'a> {
    pub fn new(funcs: ActiveFuncsIter<'a>) -> Self {
        Self {
            active_funcs: funcs,
        }
    }
}

impl<'a> Iterator for SelectionValuesIterator<'a> {
    type Item = FuncYValuesIter<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(func) = self.active_funcs.next() {
            Some(func.values_selections())
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.active_funcs.size_hint()
    }
}

impl<'a> ExactSizeIterator for SelectionValuesIterator<'a> {}

struct ActiveFuncsIter<'a> {
    active_func_indexes: bit_set::Iter<'a>,
    functions: &'a [Func],
}
impl<'a> ActiveFuncsIter<'a> {
    pub fn new(indexes: bit_set::Iter<'a>, functions: &'a [Func]) -> Self {
        Self {
            active_func_indexes: indexes,
            functions,
        }
    }
}

impl<'a> Iterator for ActiveFuncsIter<'a> {
    type Item = &'a Func;

    fn next(&mut self) -> Option<Self::Item> {
        self.active_func_indexes
            .next()
            .and_then(|fi| Some(&self.functions[fi]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.active_func_indexes.size_hint()
    }
}

impl<'a> DoubleEndedIterator for ActiveFuncsIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.active_func_indexes
            .next_back()
            .and_then(|fi| Some(&self.functions[fi]))
    }
}

impl<'a> ExactSizeIterator for ActiveFuncsIter<'a> {}

struct Functions {
    functions: Vec<Func>,
    active_funcs: BitSet,
}

impl<'a> Functions {
    pub fn active_funcs_len(&self) -> usize {
        self.active_funcs.len()
    }

    pub fn iter_active(&'a self) -> ActiveFuncsIter {
        ActiveFuncsIter::new(self.active_funcs.iter(), &self.functions)
    }

    pub fn iter_inactive(&'a self) -> impl Iterator<Item = &'a Func> {
        self.iter_inactive_index().map(|(f, _)| f)
    }

    pub fn iter_active_index(&'a self) -> impl ExactSizeIterator<Item = (&'a Func, usize)> {
        self.active_funcs.iter().map(|fi| (&self.functions[fi], fi))
    }

    pub fn iter_inactive_index(&'a self) -> impl Iterator<Item = (&'a Func, usize)> {
        self.functions.iter().enumerate().filter_map(|(fi, f)| {
            if self.active_funcs.contains(fi) {
                Some((f, fi))
            } else {
                None
            }
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Func> {
        self.functions.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.functions.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Func> {
        self.functions.iter()
    }

    pub fn set_func_state_for_all(&mut self, new_state: GraphFuncState) {
        match new_state {
            GraphFuncState::Active => self.active_funcs.insert_range(0..self.len()),
            GraphFuncState::Inactive => self.active_funcs.remove_range(0..self.len()),
        }
    }

    pub fn retain_active(&'a mut self, mut retain: impl FnMut(usize, &Func) -> GraphFuncState) {
        self.active_funcs.retain(|fi| {
            let f = &self.functions[fi];
            match retain(fi, f) {
                GraphFuncState::Active => true,
                GraphFuncState::Inactive => false,
            }
        });
    }

    pub fn for_each_active_mut(&'a mut self, mut apply: impl FnMut(&mut Func)) {
        let functions = self.functions.as_mut_slice();
        for fi in self.active_funcs.iter() {
            apply(&mut functions[fi]);
        }
    }

    pub fn for_each_active_ok_mut<T, E>(
        &'a mut self,
        mut apply: impl FnMut(&mut Func) -> Result<T, E>,
    ) -> Result<(), E> {
        let functions = self.functions.as_mut_slice();
        for fi in self.active_funcs.iter() {
            apply(&mut functions[fi])?;
        }
        Ok(())
    }

    pub fn iter_inactive_mut(&'a mut self) -> impl Iterator<Item = &'a mut Func> {
        self.functions.iter_mut().enumerate().filter_map(|(fi, f)| {
            if self.active_funcs.contains(fi) {
                Some(f)
            } else {
                None
            }
        })
    }

    pub fn change_func_state(
        &mut self,
        index: usize,
        new_state: GraphFuncState,
    ) -> Option<GraphFuncState> {
        if index >= self.len() {
            return None;
        }

        let was_active = match new_state {
            GraphFuncState::Active => self.active_funcs.insert(index) == false,
            GraphFuncState::Inactive => self.active_funcs.remove(index),
        };
        Some(if was_active {
            GraphFuncState::Active
        } else {
            GraphFuncState::Inactive
        })
    }

    pub fn get_func_state(&self, index: usize) -> Option<GraphFuncState> {
        if index >= self.len() {
            return None;
        }
        let is_active = self.active_funcs.contains(index);
        let state = if is_active {
            GraphFuncState::Active
        } else {
            GraphFuncState::Inactive
        };
        Some(state)
    }
}

pub struct Graph {
    functions: Functions,
    selection: RangeInclusive<X>,
}

impl<'a> Graph {
    pub fn new(mut functions: Vec<Func>) -> Self {
        let selection = functions
            .iter()
            .filter_map(|f| f.points().first())
            .map(|p| p.x)
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0)
            ..=functions
                .iter()
                .filter_map(|f| f.points().last())
                .map(|p| p.x)
                .max_by(|a, b| a.total_cmp(b))
                .unwrap_or(0.0);

        functions
            .iter_mut()
            .for_each(|f| f.change_selection(&selection));
        Self {
            selection,
            functions: Functions {
                active_funcs: (0..functions.len()).into_iter().into(),
                functions,
            },
        }
    }
    pub fn selection(&self) -> &RangeInclusive<X> {
        &self.selection
    }

    pub fn selection_points(&self) -> SelectionValuesIterator {
        SelectionValuesIterator::new(self.functions.iter_active())
    }

    pub fn change_selection(&mut self, selection: RangeInclusive<X>) {
        self.selection = selection;
        self.functions
            .iter_mut()
            .for_each(|f| f.change_selection(&self.selection));
    }

    pub fn move_selection_by(&mut self, add: RangeInclusive<X>) {
        let selection = &self.selection;
        let new_selection = (add.start() + selection.start())..=(add.end() + selection.end());
        self.change_selection(new_selection);
    }

    pub fn active_func_indexes(&'a self) -> impl ExactSizeIterator<Item = usize> + Clone + 'a {
        self.functions.active_funcs.iter()
    }

    pub fn inactive_func_indexes(&'a self) -> impl ExactSizeIterator<Item = usize> + Clone + 'a {
        self.functions.active_funcs.iter()
    }

    pub fn active_functions(&'a self) -> impl ExactSizeIterator<Item = &'a Func> {
        self.functions.iter_active()
    }

    pub fn inactive_functions_index(&'a self) -> impl Iterator<Item = (&'a Func, usize)> {
        self.functions.iter_inactive_index()
    }

    pub fn active_functions_index(&'a self) -> impl Iterator<Item = (&'a Func, usize)> {
        self.functions.iter_active_index()
    }

    pub fn inactive_functions(&'a self) -> impl Iterator<Item = &'a Func> {
        self.functions.iter_inactive()
    }

    pub fn functions(&'a self) -> impl Iterator<Item = &'a Func> {
        self.functions.iter()
    }

    pub fn len(&self) -> usize {
        self.functions.len()
    }

    pub fn max_x(&self) -> Option<X> {
        self.functions
            .iter_active()
            .filter_map(|f| f.points().last())
            .map(|p| p.x)
            .max_by(|a, b| a.total_cmp(b))
    }

    pub fn min_x(&self) -> Option<X> {
        self.functions
            .iter_active()
            .filter_map(|f| f.points().first())
            .map(|p| p.y)
            .min_by(|a, b| a.total_cmp(b))
    }

    pub fn active_funcs_len(&self) -> usize {
        self.functions.active_funcs_len()
    }

    pub fn change_func_state(
        &mut self,
        index: usize,
        new_state: GraphFuncState,
    ) -> Option<GraphFuncState> {
        self.functions.change_func_state(index, new_state)
    }

    pub fn get_func_state(&self, index: usize) -> Option<GraphFuncState> {
        self.functions.get_func_state(index)
    }

    pub fn insert_values<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        at: X,
        values: impl IntoIterator<Item = YExactIter>,
    ) {
        for func_values in values {
            self.functions.for_each_active_mut(|f| {
                f.insert_values(FuncValuesCheckIter::new(func_values.clone(), at).into());
            });
        }
    }

    pub fn insert_pattern<Iter: IntoIterator<Item = Y>>(&mut self, at: X, values: Iter) {
        let functions = self.functions.functions.as_mut_slice();

        let mut active_funcs = self.functions.active_funcs.iter();

        if let Some(fi) = active_funcs.next() {
            let mut index = fi + 1;
            let (f_slice, mut funcs) = functions.split_at_mut(index);
            let mut range = f_slice
                .last_mut()
                .unwrap()
                .insert_pattern(FuncValuesCheckIter::new(values.into_iter(), at).into());
            for fi in active_funcs {
                let (f_slice, functions) = funcs.split_at_mut(fi + 1 - index);
                funcs = functions;
                index += fi + 1;
                range = f_slice.last_mut().unwrap().insert_pattern(range.into());
            }
        }
    }

    pub fn undo_op<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        op: &SharedOp<
            impl Iterator<Item = usize> + Clone,
            impl Iterator<Item = YExactIter> + Clone,
            YExactIter,
        >,
    ) {
        match op {
            SharedOp::Delete(points) => {
                self.insert_values(*self.selection.start(), points.0.clone());
            }
            SharedOp::StretchY(stretch) => {
                self.stretch_y_with_factor(&StretchY {
                    factor: 1.0 / stretch.factor,
                    flags: stretch.flags,
                });
            }
            SharedOp::InsertValues(points) => {
                let len = points.values.clone().map(|f_vals| f_vals.len()).max();
                if let Some(len) = len {
                    let new_selection = points.x..=(points.x + len as X);
                    let selection = self.selection.clone();
                    self.change_selection(new_selection);
                    self.delete();
                    self.change_selection(selection);
                }
            }
            SharedOp::InsertPattern(points) => {
                let len = points.values.len();
                let new_selection = points.x..=(points.x + len as X);
                let selection = self.selection.clone();
                self.change_selection(new_selection);
                self.delete();
                self.change_selection(selection);
            }
            SharedOp::MoveSelectBy(move_by) => {
                self.change_selection(move_by.negated().move_selection(&self.selection));
            }
            SharedOp::ChangeActiveFuncs(change) => {
                let funcs = &mut self.functions.active_funcs;
                for fi in change.clone().into_iter() {
                    funcs.toggle(fi);
                }
            }
        }
    }

    pub fn redo_op<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        op: &SharedOp<
            impl Iterator<Item = usize> + Clone,
            impl Iterator<Item = YExactIter> + Clone,
            YExactIter,
        >,
    ) {
        match op {
            SharedOp::Delete(_) => {
                self.delete();
            }
            SharedOp::StretchY(stretch) => {
                self.stretch_y_with_factor(stretch);
            }
            SharedOp::InsertValues(points) => {
                self.insert_values(points.x, points.values.clone());
            }
            SharedOp::InsertPattern(points) => {
                self.insert_pattern(points.x, points.values.clone());
            }
            SharedOp::MoveSelectBy(move_by) => {
                self.change_selection(move_by.move_selection(&self.selection));
            }
            SharedOp::ChangeActiveFuncs(change) => {
                let funcs = &mut self.functions.active_funcs;
                for fi in change.clone().into_iter() {
                    funcs.toggle(fi);
                }
            }
        }
    }

    pub fn delete(&mut self) {
        self.functions.for_each_active_mut(|f| {
            f.delete();
        });
    }

    ///Return whether any function was modified
    pub fn stretch_y_with_factor(&mut self, stretch: &StretchY) -> bool {
        let mut stretched = false;
        if stretch.stretches() {
            self.functions.for_each_active_mut(|f| {
                let modified = f.stretch_y_with_factor(stretch);
                if !stretched && modified {
                    stretched = true;
                }
            });
        }
        stretched
    }

    pub fn stretch_y(&mut self, bounds: &StretchYBounds) -> Result<StretchY, StretchYBoundsError> {
        let mut factor = Y::INFINITY;
        let flags = bounds.flags();
        if flags.is_empty() || bounds.is_empty() {
            return Ok(StretchY::no_stretch());
        }
        self.functions.for_each_active_ok_mut(|f| {
            let new_factor = f.min_y_stretch_factor_for_bounds(bounds)?;
            if (new_factor - 1.0).abs() < (factor - 1.0).abs() {
                factor = new_factor;
            }
            Ok(())
        })?;

        if let Some(stretch) = StretchY::new(factor, flags) {
            self.functions.for_each_active_mut(|f| {
                f.stretch_y_with_factor(&stretch);
            });
            Ok(stretch)
        } else {
            Ok(StretchY::no_stretch())
        }
    }

    pub fn change_each_active_func_state(
        &mut self,
        mut apply: impl FnMut(usize, &Func) -> GraphFuncState,
    ) {
        self.functions.active_funcs.retain(|fi| {
            let f = &self.functions.functions[fi];
            match apply(fi, f) {
                GraphFuncState::Active => true,
                GraphFuncState::Inactive => false,
            }
        })
    }

    pub fn set_func_state_for_all(&mut self, new_state: GraphFuncState) {
        self.functions.set_func_state_for_all(new_state);
    }

    pub fn global_min(&self) -> Option<Y> {
        self.active_functions()
            .filter_map(|f| f.min())
            .min_by(|a, b| a.total_cmp(b))
    }

    pub fn global_max(&self) -> Option<Y> {
        self.active_functions()
            .filter_map(|f| f.max())
            .max_by(|a, b| a.total_cmp(b))
    }

    pub fn min(&self) -> Option<Y> {
        self.active_functions()
            .filter_map(|f| f.selection_min())
            .min_by(|a, b| a.total_cmp(b))
    }

    pub fn max(&self) -> Option<Y> {
        self.active_functions()
            .filter_map(|f| f.selection_max())
            .max_by(|a, b| a.total_cmp(b))
    }

    pub fn value_range(&self) -> Option<RangeInclusive<Y>> {
        self.active_functions()
            .filter_map(|f| f.selection_value_range())
            .fold(None, |a, r| {
                if let Some(a) = a {
                    Some((a.start().min(*r.start())..=a.end().max(*r.end())).into())
                } else {
                    Some(r)
                }
            })
    }
}
