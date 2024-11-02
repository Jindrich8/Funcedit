pub mod change_active_funcs;

use std::{convert::Infallible, iter::Copied, marker::PhantomData, slice::Iter};

use change_active_funcs::{ChangeActiveFuncs, ChangeActiveFuncsIter};

use crate::{
    shared_op::{Delete, InsertPattern, MoveSelectBy, SharedOp, StretchY},
    types::point::{Point, X, Y},
};

use super::shared_entry::{
    OpCombineErr, OpCreateErr, OrderMattersOp, OtherOp, OwnedOp, SharedOutOp,
};

pub(super) struct InsertOpArgs {
    pub points: Box<[Point]>,
    pub splits: Box<[usize]>,
}

impl InsertOpArgs {
    pub fn iter<'a>(&'a self) -> impl IntoIterator<Item = &'a [Point]> {
        let points = &*self.points;
        let mut i = 0;
        self.splits.iter().map(move |mid| {
            let index = i;
            i = *mid;
            &points[index..*mid]
        })
    }
}

#[derive(Debug, Clone)]
pub struct FuncIter<'a> {
    iter: std::slice::Iter<'a, Box<[Y]>>,
}

impl<'a> Iterator for FuncIter<'a> {
    type Item = Copied<std::slice::Iter<'a, Y>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(values) = self.iter.next() {
            Some(values.iter().copied())
        } else {
            None
        }
    }
}

pub type OutSharedOp<'a> = SharedOp<ChangeActiveFuncsIter<'a>, FuncIter<'a>, Copied<Iter<'a, Y>>>;

#[derive(Debug)]
pub(super) enum Op<OtherOp> {
    Delete(Box<[Box<[Y]>]>),
    StretchY(StretchY),
    InsertValues(X, Box<[Box<[Y]>]>),
    InsertPattern(X, Box<[Y]>),
    MoveSelectBy(f64, f64),
    ChangeActiveFuncs(ChangeActiveFuncs),
    Other(OtherOp),
}

impl<'a, OpOther> Op<OpOther> {
    pub fn try_combine<
        IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        YExactIter: ExactSizeIterator<Item = Y> + Clone,
        FuncIter: Iterator<Item = YExactIter>,
    >(
        &mut self,
        shared: SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>,
    ) -> Result<(), OpCombineErr<SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>>> {
        match (self, &shared) {
            (Self::StretchY(op), SharedOp::StretchY(shared_stretch)) => {
                if op.flags == shared_stretch.flags {
                    op.factor *= shared_stretch.factor;
                } else if (op.factor - shared_stretch.factor).abs() < Y::EPSILON {
                    op.flags |= shared_stretch.flags;
                } else {
                    return Err(OpCombineErr::CannotCombine(shared));
                }
            }
            (Self::MoveSelectBy(op_start, op_end), SharedOp::MoveSelectBy(shared)) => {
                *op_start += shared.start_by;
                *op_end += shared.end_by;
            }
            (Self::ChangeActiveFuncs(op), SharedOp::ChangeActiveFuncs(shared)) => {
                for i in shared.clone() {
                    op.toggle(i);
                }
                if op.len() < 1 {
                    return Err(OpCombineErr::OpDoesNotHaveEffect);
                }
            }
            _ => return Err(OpCombineErr::CannotCombine(shared)),
        }
        Ok(())
    }

    pub fn order_matters<
        IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        YExactIter: ExactSizeIterator<Item = Y> + Clone,
        FuncIter: Iterator<Item = YExactIter>,
    >(
        &self,
        other: &SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>,
    ) -> bool
    where
        OpOther: OtherOp,
    {
        match self.get_order_matters_op() {
            OrderMattersOp::Other(owned) => owned.order_to_graph_matters(other),
            OrderMattersOp::Graph(g) => other.order_matters(g),
        }
    }
}

impl<'a, OtherOp> Op<OtherOp> {
    pub fn get_order_matters_op(&'a self) -> OrderMattersOp<'a, OtherOp> {
        fn g<'a, OtherOp>(op: OutSharedOp<'a>) -> OrderMattersOp<'a, OtherOp> {
            OrderMattersOp::Graph(op)
        }
        match self {
            Op::Delete(points) => {
                let op = SharedOp::Delete(Delete(FuncIter {
                    iter: points.iter(),
                }));
                g(op)
            }
            Op::StretchY(f) => g(SharedOp::StretchY(f.clone())),
            Op::InsertValues(x, values) => {
                g(SharedOp::InsertValues(crate::shared_op::InsertValues {
                    x: *x,
                    values: FuncIter {
                        iter: values.iter(),
                    },
                }))
            }
            Self::InsertPattern(x, pattern) => g(SharedOp::InsertPattern(InsertPattern {
                x: *x,
                values: pattern.iter().copied(),
            })),
            Op::MoveSelectBy(start, end) => g(SharedOp::MoveSelectBy(MoveSelectBy {
                start_by: *start,
                end_by: *end,
            })),
            Op::ChangeActiveFuncs(change) => g(SharedOp::ChangeActiveFuncs(change.iter())),
            Op::Other(op) => OrderMattersOp::Other(op),
        }
    }

    pub fn get_shared<SharedOtherOp>(&'a self) -> SharedOutOp<'a, SharedOtherOp>
    where
        OtherOp: OwnedOp< SharedOtherOp>,
    {
        match self.get_order_matters_op() {
            OrderMattersOp::Graph(g) => SharedOutOp::Graph(g),
            OrderMattersOp::Other(o) => SharedOutOp::Other(o.get_shared()),
        }
    }

    pub fn get_shared_opt<SharedOtherOp>(
        op: Option<&'a Self>,
    ) -> Option<SharedOutOp<'a, SharedOtherOp>>
    where
        OtherOp: OwnedOp< SharedOtherOp>,
    {
        if let Some(op) = op {
            Some(op.get_shared())
        } else {
            None
        }
    }
}

impl<
        'a,
        IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        YExactIter: ExactSizeIterator<Item = Y> + Clone,
        FuncIter: Iterator<Item = YExactIter>,
        OtherOp,
    > TryFrom<SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>> for Op<OtherOp>
{
    type Error = OpCreateErr;
    fn try_from(
        op: SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>,
    ) -> Result<Self, OpCreateErr> {
        let res = match op {
            SharedOp::Delete(iter) => {
                let del = iter
                    .0
                    .into_iter()
                    .map(|vals| vals.into_iter().collect::<Vec<_>>().into_boxed_slice())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                if del.len() < 1 {
                    return Err(OpCreateErr::OpDoesNotHaveEffect);
                }
                Self::Delete(del)
            }
            SharedOp::StretchY(factor) => {
                if !factor.stretches() {
                    return Err(OpCreateErr::OpDoesNotHaveEffect);
                }
                Self::StretchY(factor)
            }
            SharedOp::InsertPattern(pattern) => {
                let insert = pattern
                    .values
                    .into_iter()
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                if insert.len() < 1 {
                    return Err(OpCreateErr::OpDoesNotHaveEffect);
                }
                Self::InsertPattern(pattern.x, insert)
            }
            SharedOp::InsertValues(points) => {
                let insert = points
                    .values
                    .into_iter()
                    .map(|vals| vals.into_iter().collect::<Vec<_>>().into_boxed_slice())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                if insert.len() < 1 {
                    return Err(OpCreateErr::OpDoesNotHaveEffect);
                }
                Self::InsertValues(points.x, insert)
            }
            SharedOp::MoveSelectBy(move_by) => {
                if !move_by.is_move() {
                    return Err(OpCreateErr::OpDoesNotHaveEffect);
                }
                Self::MoveSelectBy(move_by.start_by, move_by.end_by)
            }
            SharedOp::ChangeActiveFuncs(change) => {
                if let Some(change) = ChangeActiveFuncs::new(change) {
                    Self::ChangeActiveFuncs(change)
                } else {
                    return Err(OpCreateErr::OpDoesNotHaveEffect);
                }
            }
        };
        Ok(res)
    }
}
