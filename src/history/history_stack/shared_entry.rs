use crate::{shared_op::SharedOp, types::point::Y};
use std::{collections::vec_deque, fmt::Debug, marker::PhantomData};

use super::{
    op::{Op, OutSharedOp},
    HistoryStack, IsGraphOpNonAltering,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCombineErr<Op> {
    OpDoesNotHaveEffect,
    CannotCombine(Op),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCreateErr {
    OpDoesNotHaveEffect,
}

pub trait OtherOp {
    fn order_to_graph_matters<IterChangeActiveFuncs, FuncIter, YExactIter>(
        &self,
        other: &SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>,
    ) -> bool
    where
        IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        FuncIter: Iterator<Item = YExactIter>,
        YExactIter: ExactSizeIterator<Item = Y> + Clone,
    {
        true
    }
}

pub trait OwnedOp<Shared>: OtherOp {
    fn get_shared(&self) -> Shared;
}

pub trait InOp<Owned, ActionId>: Sized + TryInto<Owned, Error = OpCreateErr> {
    fn try_combine(self, owned: &mut Owned) -> Result<(), OpCombineErr<Self>>;

    fn order_matters<'a>(&self, other: OrderMattersOp<'a, Owned>) -> bool {
        true
    }

    fn alters_history(&self, id: &ActionId) -> bool;
}

pub enum SharedInOp<IterChangeActiveFuncs, FuncIter, YExactIter, Other>
where
    IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
    FuncIter: Iterator<Item = YExactIter>,
    YExactIter: ExactSizeIterator<Item = Y> + Clone,
{
    Graph(SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>),
    Other(Other),
}

pub enum OrderMattersOp<'a, Owned> {
    Graph(OutSharedOp<'a>),
    Other(&'a Owned),
}

pub enum SharedOutOp<'a, Other> {
    Graph(OutSharedOp<'a>),
    Other(Other),
}

pub enum ApplyOp<'a, Other> {
    Graph(ApplyGraphOp<'a>),
    Other(ApplyOtherOp<Other>),
}

impl<'a, Other> ApplyOp<'a, Other> {
    pub fn new_undo(op: SharedOutOp<'a, Other>) -> Self {
        match op {
            SharedOutOp::Graph(o) => Self::Graph(ApplyGraphOp::Undo(o)),
            SharedOutOp::Other(o) => Self::Other(ApplyOtherOp::Undo(o)),
        }
    }

    pub fn new_redo(op: SharedOutOp<'a, Other>) -> Self {
        match op {
            SharedOutOp::Graph(o) => Self::Graph(ApplyGraphOp::Redo(o)),
            SharedOutOp::Other(o) => Self::Other(ApplyOtherOp::Redo(o)),
        }
    }
}

#[derive(Debug)]
pub enum ApplyOtherOp<Other> {
    Undo(Other),
    Redo(Other),
}

impl<Other: Clone> ApplyOtherOp<Other> {
    pub fn inversed(&self) -> Self {
        match self {
            Self::Undo(o) => Self::Redo(o.clone()),
            Self::Redo(o) => Self::Undo(o.clone()),
        }
    }
}

#[derive(Debug)]
pub enum ApplyGraphOp<'a> {
    Undo(OutSharedOp<'a>),
    Redo(OutSharedOp<'a>),
}

impl<'a, Other: Debug> Debug for SharedOutOp<'a, Other> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Graph(arg0) => f.debug_tuple("Graph").field(arg0).finish(),
            Self::Other(arg0) => f.debug_tuple("Other").field(arg0).finish(),
        }
    }
}

enum NonAlteringEntryOptions {
    IncludeOps,
    IsNonAlteringEntry,
}

struct BaseSharedEntry<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    index: usize,
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > BaseSharedEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        index: usize,
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Option<Self> {
        if history.entries.get(index).is_some() {
            Some(Self { history, index })
        } else {
            None
        }
    }

    pub fn get_id(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> &OpGroupId {
        &history.entries[index].id
    }

    pub fn id(&self) -> &OpGroupId {
        Self::get_id(&self.history, self.index)
    }

    pub fn get_is_non_altering(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> bool {
        index >= history.len
    }

    fn is_non_altering(&self) -> bool {
        Self::get_is_non_altering(&self.history, self.index)
    }

    pub fn get_include_non_altering_ops(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> bool {
        // index is always < usize::MAX, because number of entries is limited to usize::MAX,
        // so this should always work as |undo_len - index| <= 1
        (history.undo_len.wrapping_sub(index)) <= 1
            && history.treat_non_altering_entries_as_regular()
    }

    fn include_non_altering_ops(&self) -> bool {
        Self::get_include_non_altering_ops(&self.history, self.index)
    }

    pub fn get_len(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> usize {
        let mut len = history.get_entry_op_range(index).len();
        if Self::get_include_non_altering_ops(history, index) {
            len += history.non_atering_ops_count();
        }
        len
    }

    pub fn len(&self) -> usize {
        Self::get_len(&self.history, self.index)
    }

    pub fn get_op_range_iter(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> vec_deque::Iter<'a, Op<OpOwned>> {
        history.ops.range(history.get_entry_op_range(index))
    }

    fn op_range_iter(&self) -> vec_deque::Iter<'a, Op<OpOwned>> {
        Self::get_op_range_iter(&self.history, self.index)
    }

    pub fn get_iter<OpOut, const UNDO: bool>(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        let is_non_altering = Self::get_is_non_altering(&history, index);
        if is_non_altering {
            SharedEntryIter::<_, _, UNDO>::new(
                Self::get_op_range_iter(&history, index),
                history.ops.range(0..0),
            )
        } else {
            let non_altering_range = history.non_altering_op_range();
            SharedEntryIter::new(
                history.ops.range(non_altering_range),
                Self::get_op_range_iter(&history, index),
            )
        }
    }

    fn iter<OpOut, const UNDO: bool>(&'a self) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        Self::get_iter::<OpOut, UNDO>(&self.history, self.index)
    }

    pub fn get_undo_iter<OpOut>(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        Self::get_iter::<OpOut, true>(&history, index)
    }

    pub fn undo_iter<OpOut>(&'a self) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        Self::get_undo_iter(&self.history, self.index)
    }

    pub fn get_redo_iter<OpOut>(
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        index: usize,
    ) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        Self::get_iter::<OpOut, false>(&history, index)
    }

    pub fn redo_iter<OpOut>(&'a self) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        Self::get_redo_iter(&self.history, self.index)
    }
}

pub struct SharedEntryIter<'a, OpOwned: 'a, OpOut, const UNDO: bool = false>
where
    OpOwned: OwnedOp<OpOut> + 'a,
{
    non_altering: vec_deque::Iter<'a, Op<OpOwned>>,
    next: vec_deque::Iter<'a, Op<OpOwned>>,
    _markder: PhantomData<OpOut>,
}

impl<'a, OpOwned, OpOut, const UNDO: bool> SharedEntryIter<'a, OpOwned, OpOut, UNDO>
where
    OpOwned: OwnedOp<OpOut> + 'a,
{
    pub(self) fn new(
        non_altering: vec_deque::Iter<'a, Op<OpOwned>>,
        next: vec_deque::Iter<'a, Op<OpOwned>>,
    ) -> Self {
        Self {
            non_altering,
            next,
            _markder: PhantomData,
        }
    }
}

impl<'a, OpOwned, OpOut, const UNDO: bool> Iterator for SharedEntryIter<'a, OpOwned, OpOut, UNDO>
where
    OpOwned: OwnedOp<OpOut> + 'a,
{
    type Item = ApplyOp<'a, OpOut>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(op) = self.non_altering.next() {
            Some(ApplyOp::new_undo(op.get_shared()))
        } else if let Some(op) = self.next.next() {
            let shared = op.get_shared();
            Some(if UNDO {
                ApplyOp::new_undo(shared)
            } else {
                ApplyOp::new_redo(shared)
            })
        } else {
            None
        }
    }
}

pub struct SharedUndoEntry<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    entry: BaseSharedEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > SharedUndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        index: usize,
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Option<Self> {
        if let Some(entry) = BaseSharedEntry::new(index, history) {
            Some(Self { entry })
        } else {
            None
        }
    }

    pub fn id(&self) -> &OpGroupId {
        &self.entry.id()
    }

    pub fn len(&self) -> usize {
        self.entry.len()
    }
}
impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > SharedUndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn iter<OpOut>(&'a self) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        self.entry.undo_iter()
    }
}

pub struct SharedRedoEntry<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    entry: BaseSharedEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > SharedRedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        index: usize,
        history: &'a HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Option<Self> {
        if let Some(entry) = BaseSharedEntry::new(index, history) {
            Some(Self { entry })
        } else {
            None
        }
    }

    pub fn id(&self) -> &OpGroupId {
        &self.entry.id()
    }

    pub fn len(&self) -> usize {
        self.entry.len()
    }
}
impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > SharedRedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn iter<OpOut>(&'a self) -> impl Iterator<Item = ApplyOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        self.entry.redo_iter()
    }
}

trait SpecializedEntry<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
>
{
    fn history_and_index(
        &self,
    ) -> (
        &HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        usize,
    );

    fn id<'b>(&'b self) -> &OpGroupId
    where
        NonAlteringGraphOpHelper: 'b,
        OpOwned: 'b,
    {
        let (history, index) = self.history_and_index();
        BaseSharedEntry::get_id(history, index)
    }

    fn len(&self) -> usize {
        let (history, index) = self.history_and_index();
        BaseSharedEntry::get_len(history, index)
    }

    fn is_non_altering(&self) -> bool {
        let (history, index) = self.history_and_index();
        BaseSharedEntry::get_is_non_altering(history, index)
    }

    fn include_non_altering_ops(&self) -> bool {
        let (history, index) = self.history_and_index();
        BaseSharedEntry::get_include_non_altering_ops(history, index)
    }

    fn op_range_iter<'b>(&'b self) -> vec_deque::Iter<'b, Op<OpOwned>>
    where
        NonAlteringGraphOpHelper: 'b,
        OpOwned: 'b,
        OpGroupId: 'b,
    {
        let (history, index) = self.history_and_index();
        BaseSharedEntry::get_op_range_iter(history, index)
    }

    fn undo_iter<'b, OpOut>(&'b self) -> impl Iterator<Item = ApplyOp<'b, OpOut>>
    where
        OpOwned: OwnedOp<OpOut> + 'b,
        NonAlteringGraphOpHelper: 'b,
        OpGroupId: 'b,
    {
        let (history, index) = self.history_and_index();
        BaseSharedEntry::get_undo_iter(history, index)
    }

    fn redo_iter<'b, OpOut>(&'b self) -> impl Iterator<Item = ApplyOp<'b, OpOut>>
    where
        OpOwned: OwnedOp<OpOut> + 'b,
        NonAlteringGraphOpHelper: 'b,
        OpGroupId: 'b,
    {
        let (history, index) = self.history_and_index();
        BaseSharedEntry::get_redo_iter(history, index)
    }
}

struct BaseEntry<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    index: usize,
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > BaseEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        index: usize,
        history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Option<Self> {
        if history.entries.get(index).is_some() {
            Some(Self { history, index })
        } else {
            None
        }
    }
}
impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > SpecializedEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
    for BaseEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    fn history_and_index(
        &self,
    ) -> (
        &HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        usize,
    ) {
        (self.history, self.index)
    }
}
pub struct UndoEntry<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    index: usize,
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > UndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        index: usize,
        history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Option<Self> {
        if history.entries.get(index).is_some() {
            Some(Self { history, index })
        } else {
            None
        }
    }
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > SpecializedEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
    for UndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    fn history_and_index(
        &self,
    ) -> (
        &HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        usize,
    ) {
        (self.history, self.index)
    }
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > Drop for UndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    fn drop(&mut self) {
        self.history.no_non_altering_entries();
    }
}

pub struct RedoEntry<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    index: usize,
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > RedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        index: usize,
        history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    ) -> Option<Self> {
        if history.entries.get(index).is_some() {
            Some(Self { history, index })
        } else {
            None
        }
    }
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > SpecializedEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
    for RedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    fn history_and_index(
        &self,
    ) -> (
        &HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        usize,
    ) {
        (self.history, self.index)
    }
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > Drop for RedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    fn drop(&mut self) {
        self.history.no_non_altering_entries();
    }
}
