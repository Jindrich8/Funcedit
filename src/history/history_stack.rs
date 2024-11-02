pub mod drop_entry;
mod entry;
pub mod entry_builder;
mod op;
pub mod pop_entry;
pub mod shared_entry;

use std::{collections::VecDeque, marker::PhantomData, ops::Range, usize};

use entry::Entry;
use entry_builder::EntryBuilder;
use op::Op;
use pop_entry::PopEntry;
use shared_entry::{
    InOp, OpCombineErr, OpCreateErr, OtherOp, OwnedOp, RedoEntry, SharedOutOp, SharedRedoEntry,
    SharedUndoEntry, UndoEntry,
};

use crate::shared_op::SharedOp;
use crate::types::point::Y;
use enumflags2::{bitflags, BitFlags};

#[derive(Debug, Clone, Copy, PartialEq)]
enum OpenOptions {
    None,
    OpenEntry,
    EntryIsOpen,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[bitflags]
enum HistoryFlag {
    ForgetLastEntryIfTooManyEntries,
    ForgetLastEntryIfTooManyOp,
    TreatNonAlteringEntriesAsRegular,
}

impl HistoryFlag {
    pub fn flags_from_options(options: BitFlags<HistoryOption>) -> BitFlags<Self> {
        let mut flags = BitFlags::empty();
        if options.contains(HistoryOption::ForgetLastEntryIfTooManyEntries) {
            flags.insert(HistoryFlag::ForgetLastEntryIfTooManyEntries);
        }
        if options.contains(HistoryOption::ForgetLastEntryIfTooManyOp) {
            flags.insert(HistoryFlag::ForgetLastEntryIfTooManyOp);
        }
        if options.contains(HistoryOption::TreatNonAlteringEntriesAsRegular) {
            flags.insert(HistoryFlag::TreatNonAlteringEntriesAsRegular);
        }
        flags
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[bitflags]
pub enum HistoryOption {
    ForgetLastEntryIfTooManyEntries,
    ForgetLastEntryIfTooManyOp,
    TreatNonAlteringEntriesAsRegular,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryError {
    /// Contains the maximum amount of entries in the history.
    TooManyEntries(usize),
}

pub struct HistoryStack<
    OpGroupId: Clone + Default,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    entries: VecDeque<Entry<OpGroupId>>,
    ops: VecDeque<Op<OpOwned>>,
    len: usize,
    undo_len: usize,
    max_size: usize,
    open_options: OpenOptions,
    flags: BitFlags<HistoryFlag>,
    _marker: PhantomData<NonAlteringGraphOpHelper>,
}

pub trait IsGraphOpNonAltering<OpGroupId> {
    fn graph_op_alters_history<IterChangeActiveFuncs, FuncIter, YExactIter>(
        g_op: &SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>,
        group_id: &OpGroupId,
    ) -> bool
    where
        IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        FuncIter: Iterator<Item = YExactIter>,
        YExactIter: ExactSizeIterator<Item = Y> + Clone;
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            ops: VecDeque::new(),
            len: 0,
            undo_len: 0,
            open_options: OpenOptions::None,
            max_size: usize::MAX,
            flags: BitFlags::empty(),
            _marker: PhantomData,
        }
    }

    pub fn with_max_size(&mut self, max_size: usize) -> &mut Self {
        self.max_size = max_size;
        self
    }

    pub fn with_options(&mut self, options: impl Into<BitFlags<HistoryOption>>) -> &mut Self {
        self.flags = HistoryFlag::flags_from_options(options.into());
        self
    }

    pub fn pop_first(&mut self) -> Option<PopEntry<OpGroupId, OpOwned, NonAlteringGraphOpHelper>> {
        PopEntry::new(0, self)
    }

    pub fn clear(&mut self) {
        self.ops.clear();
        self.entries.clear();
    }

    fn treat_non_altering_entries_as_regular(&self) -> bool {
        self.flags
            .contains(HistoryFlag::TreatNonAlteringEntriesAsRegular)
    }

    fn get_opt_entry_op_end(entry: Option<&Entry<OpGroupId>>) -> usize {
        entry.unwrap_or(&Entry::<OpGroupId>::default()).op_end
    }

    fn get_new_entry_start(&self) -> usize {
        Self::get_opt_entry_op_end(self.entries.get(self.undo_len.saturating_sub(1)))
    }

    fn get_entry_start(&self, index: usize) -> usize {
        if index < 1 {
            0
        } else {
            Self::get_opt_entry_op_end(self.entries.get(index - 1))
        }
    }

    fn get_entry_op_range(&self, index: usize) -> Range<usize> {
        Self::get_entry_start(&self, index)..self.entries[index].op_end
    }

    fn get_last_entry_start(&self) -> usize {
        Self::get_entry_start(&self, self.undo_len.saturating_sub(1))
    }

    fn get_last_entry_op_range(&self) -> Range<usize> {
        Self::get_last_entry_start(&self)..self.get_new_entry_start()
    }

    pub fn undo_len(&self) -> usize {
        self.undo_len
    }

    pub fn redo_len(&self) -> usize {
        self.len().saturating_sub(self.undo_len())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns index of pushed op or None if op was not actually pushed into history.
    fn push_other_op(
        &mut self,
        group_id: &OpGroupId,
        mut op: impl InOp<OpOwned, OpGroupId>,
    ) -> Option<usize> {
        if op.alters_history(group_id) {
            self.no_redo();
        } else {
            println!("");
        }
        let mut start = Self::get_opt_entry_op_end(self.entries.back());
        let end = self.ops.len();
        if start >= end && self.open_options == OpenOptions::EntryIsOpen {
            if let Some(last_entry) = self.entries.back() {
                if last_entry.id == *group_id {
                    // There needs to be entries.len(), because that is the only reliable way to get last entry
                    // in case of non altering entries.
                    start = self.get_entry_start(self.entries.len() - 1);
                }
            }
        }

        if start < end {
            for owned_op in self.ops.range_mut(start..end).rev() {
                match owned_op {
                    Op::Other(other) => match op.try_combine(other) {
                        Ok(_) => return None,
                        Err(e) => match e {
                            OpCombineErr::OpDoesNotHaveEffect => return None,
                            OpCombineErr::CannotCombine(v) => {
                                op = v;
                                if op.order_matters(owned_op.get_order_matters_op()) {
                                    break;
                                }
                            }
                        },
                    },
                    _ => (),
                };
            }
        }

        match op.try_into() {
            Ok(op) => {
                let index = self.ops.len();
                self.ops.push_back(Op::Other(op));
                Some(index)
            }
            Err(e) => match e {
                OpCreateErr::OpDoesNotHaveEffect => None,
            },
        }
    }

    /// Returns index of pushed op or None if op was not actually pushed into history.
    fn push_graph_op<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        group_id: &OpGroupId,
        mut op: SharedOp<
            impl Iterator<Item = usize> + Clone,
            impl Iterator<Item = YExactIter>,
            YExactIter,
        >,
    ) -> Option<usize>
    where
        OpOwned: OtherOp,
    {
        if NonAlteringGraphOpHelper::graph_op_alters_history(&op, group_id) {
            self.no_redo();
        } else {
            println!("");
        }
        let mut start = Self::get_opt_entry_op_end(self.entries.back());
        let end = self.ops.len();
        if start >= end && self.open_options == OpenOptions::EntryIsOpen {
            if let Some(last_entry) = self.entries.back() {
                if last_entry.id == *group_id {
                    // There needs to be entries.len(), because that is the only reliable way to get last entry
                    // in case of non altering entries.
                    start = self.get_entry_start(self.entries.len() - 1);
                }
            }
        }
        if start < end {
            for owned_op in self.ops.range_mut(start..end).rev() {
                match owned_op.try_combine(op) {
                    Ok(_) => return None,
                    Err(e) => match e {
                        OpCombineErr::OpDoesNotHaveEffect => return None,
                        OpCombineErr::CannotCombine(v) => {
                            op = v;
                            if owned_op.order_matters(&op) {
                                break;
                            }
                        }
                    },
                };
            }
        }

        match op.try_into() {
            Ok(op) => {
                let index = self.ops.len();
                self.ops.push_back(op);
                Some(index)
            }
            Err(e) => match e {
                OpCreateErr::OpDoesNotHaveEffect => None,
            },
        }
    }

    fn close_new_entry(&mut self, id: OpGroupId) {
        let op_end = self.ops.len();
        if op_end > self.entries.back().unwrap_or(&Default::default()).op_end {
            let is_non_altering = self.undo_len < self.len;
            if !is_non_altering && self.open_options == OpenOptions::EntryIsOpen {
                if let Some(last) = self.entries.back_mut() {
                    if last.id == id {
                        last.op_end = op_end;
                        return;
                    }
                }
            }
            self.entries.push_back(Entry { op_end, id });
            if !is_non_altering {
                self.undo_len += 1;
                self.len += 1;
            }
        }
    }

    pub fn close_entry(&mut self, id: OpGroupId) {
        if self.open_options != OpenOptions::None && self.entries.back().is_some_and(|e| e.id == id)
        {
            self.open_options = OpenOptions::None;
        }
    }

    fn no_redo(&mut self) {
        if self.undo_len < self.len {
            self.open_options = OpenOptions::None;
            if self.len < self.entries.len() {
                // Len is guaranteed to be >= 1, because undo_len, which is usize, is smaller.
                /*
                self.entries:
                | undo entries | redo entries | non altering entries |
                               ^              ^                      ^
                          self.undo_len    self.len           self.entries.len()
                 */
                let op_redo_end = self.entries[self.len - 1].op_end;
                self.entries.drain(self.undo_len..self.len);
                self.ops.drain(self.get_new_entry_start()..op_redo_end);
            } else {
                self.entries.truncate(self.undo_len);
                self.ops.truncate(self.get_new_entry_start());
            }
            self.len = self.entries.len();
            self.undo_len = self.len;
        }
    }

    fn no_non_altering_entries(&mut self) {
        if self.len < self.entries.len() {
            self.entries.truncate(self.len);
            self.ops
                .truncate(self.get_entry_start(self.len.saturating_sub(1)));
        }
    }

    fn non_altering_op_range(&self) -> Range<usize> {
        if self.len >= 1 {
            self.entries[self.len - 1].op_end..Self::get_opt_entry_op_end(self.entries.back())
        } else {
            0..0
        }
    }

    pub fn undo(
        &'a mut self,
    ) -> Option<SharedUndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>> {
        self.open_options = OpenOptions::None;
        self.undo_len = self.undo_len.saturating_sub(1);
        SharedUndoEntry::new(self.undo_len, self)
    }

    pub fn redo(
        &'a mut self,
    ) -> Option<SharedRedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>> {
        if self.undo_len < self.len() {
            self.undo_len += 1;
            self.open_options = OpenOptions::None;
            return SharedRedoEntry::new(self.undo_len - 1, self);
        }
        None
    }

    pub fn undo_iter(
        &'a self,
    ) -> impl Iterator<Item = SharedUndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>>
    {
        let range = 0..self.undo_len;
        let non_altering_range = if self.treat_non_altering_entries_as_regular() {
            self.len..self.entries.len()
        } else {
            0..0
        };

        non_altering_range
            .into_iter()
            .rev()
            .chain(range.into_iter().rev())
            .map(|i| SharedUndoEntry::new(i, self).unwrap())
    }

    pub fn redo_iter(
        &'a self,
    ) -> impl Iterator<Item = SharedRedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>>
    {
        let range = self.undo_len..self.len;
        let non_altering_range = if self.treat_non_altering_entries_as_regular() {
            self.len..self.entries.len()
        } else {
            0..0
        };

        non_altering_range
            .into_iter()
            .rev()
            .chain(range.into_iter())
            .map(|i| SharedRedoEntry::new(i, self).unwrap())
    }

    fn non_altering_entries_iter(
        &'a self,
    ) -> impl DoubleEndedIterator<Item = SharedUndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>>
    {
        (self.len..self.entries.len())
            .into_iter()
            .map(|i| SharedUndoEntry::new(i, self).unwrap())
    }

    fn non_atering_ops_count(&self) -> usize {
        if self.len >= 1 {
            self.ops.len() - self.entries[self.len - 1].op_end
        } else {
            0
        }
    }

    pub fn push_entry(
        &mut self,
        id: OpGroupId,
    ) -> EntryBuilder<OpGroupId, OpOwned, NonAlteringGraphOpHelper> {
        self.open_options = OpenOptions::None;
        self.build_new_entry(id)
    }

    fn build_new_entry(
        &mut self,
        id: OpGroupId,
    ) -> EntryBuilder<OpGroupId, OpOwned, NonAlteringGraphOpHelper> {
        if self.entries.len() >= self.max_size {
            self.pop_first();
            if self.entries.len() >= self.max_size {
                unreachable!("Too many entries!\nThis should never happen!");
            }
        }
        EntryBuilder::new(self, id)
    }

    pub fn open_entry(
        &mut self,
        id: OpGroupId,
    ) -> EntryBuilder<OpGroupId, OpOwned, NonAlteringGraphOpHelper> {
        self.open_options = if self.open_options == OpenOptions::None {
            OpenOptions::OpenEntry
        } else {
            OpenOptions::EntryIsOpen
        };
        if self.open_options == OpenOptions::EntryIsOpen {
            EntryBuilder::new(self, id)
        } else {
            self.build_new_entry(id)
        }
    }
}
