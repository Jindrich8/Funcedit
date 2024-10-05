use std::ops::{Add, RangeInclusive};

use crate::{
    history::history_stack::shared_entry::InOp,
    types::point::{X, Y},
};

#[derive(Debug)]
pub enum SharedOp<
    IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
    FuncIter: Iterator<Item = YExactIter>,
    YExactIter: ExactSizeIterator<Item = Y> + Clone,
> {
    Delete(Delete<FuncIter, YExactIter>),
    StretchY(StretchY),
    InsertValues(InsertValues<FuncIter, YExactIter>),
    InsertPattern(InsertPattern<YExactIter>),
    MoveSelectBy(MoveSelectBy),
    ChangeActiveFuncs(IterChangeActiveFuncs),
}

impl<
        IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        FuncIter: Iterator<Item = YExactIter>,
        YExactIter: ExactSizeIterator<Item = Y> + Clone,
    > SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>
{
    pub fn order_matters<OtherIterChangeActiveFuncs, OtherFuncIter, OtherYExactIter>(
        &self,
        other: SharedOp<OtherIterChangeActiveFuncs, OtherFuncIter, OtherYExactIter>,
    ) -> bool
    where
        OtherIterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        OtherFuncIter: Iterator<Item = OtherYExactIter>,
        OtherYExactIter: ExactSizeIterator<Item = Y> + Clone,
    {
        matches!(
            (self, other),
            (
                Self::ChangeActiveFuncs(_) | Self::MoveSelectBy(_),
                SharedOp::ChangeActiveFuncs(_) | SharedOp::MoveSelectBy(_),
            )
        ) != true
    }
}

pub type StretchY = crate::functions::function::StretchY;

#[derive(Debug)]
pub struct Delete<
    Iter: IntoIterator<Item = YExactIter>,
    YExactIter: ExactSizeIterator<Item = Y> + Clone,
>(pub Iter);

#[derive(Debug)]
pub struct InsertValues<
    Iter: IntoIterator<Item = YExactIter>,
    YExactIter: ExactSizeIterator<Item = Y> + Clone,
> {
    pub x: X,
    pub values: Iter,
}

#[derive(Debug)]
pub struct InsertPattern<Iter: IntoIterator<Item = Y>> {
    pub x: X,
    pub values: Iter,
}

#[derive(Debug)]
pub struct MoveSelectBy {
    pub start_by: X,
    pub end_by: X,
}

impl MoveSelectBy {
    pub fn move_selection(&self, selection: &RangeInclusive<X>) -> RangeInclusive<X> {
        RangeInclusive::new(
            selection.start() + self.start_by,
            selection.end() + self.end_by,
        )
    }

    pub fn is_move(&self) -> bool {
        self.start_by.abs() >= X::EPSILON || self.end_by.abs() >= X::EPSILON
    }

    pub fn negated(&self) -> Self {
        Self {
            start_by: -self.start_by,
            end_by: -self.end_by,
        }
    }
}
