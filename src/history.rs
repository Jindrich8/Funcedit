pub mod history_stack;

use enumflags2::BitFlags;
use history_stack::{
    entry_builder::EntryBuilder,
    pop_entry::PopEntry,
    shared_entry::{OtherOp, SharedRedoEntry, SharedUndoEntry},
    HistoryError, HistoryOption, HistoryStack, IsGraphOpNonAltering,
};

pub struct History<
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    stack: HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
}

impl<
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > History<OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new() -> Self {
        Self {
            stack: HistoryStack::new(),
        }
    }

    pub fn with_max_size(&mut self, max_size: usize) -> &mut Self {
        self.stack.with_max_size(max_size);
        self
    }

    pub fn with_options(&mut self, options: impl Into<BitFlags<HistoryOption>>) -> &mut Self {
        self.stack.with_options(options);
        self
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn add_entry(
        &mut self,
        id: OpGroupId,
    ) -> EntryBuilder<OpGroupId, OpOwned, NonAlteringGraphOpHelper> {
        self.stack.push_entry(id)
    }

    pub fn open_entry(
        &mut self,
        id: OpGroupId,
    ) -> EntryBuilder<OpGroupId, OpOwned, NonAlteringGraphOpHelper> {
        self.stack.open_entry(id)
    }

    pub fn close_entry(&mut self, id: OpGroupId) {
        self.stack.close_entry(id)
    }

    pub fn undo_entry<'a>(
        &'a mut self,
    ) -> Option<SharedUndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>> {
        self.stack.undo()
    }

    pub fn redo_entry<'a>(
        &'a mut self,
    ) -> Option<SharedRedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>> {
        self.stack.redo()
    }

    pub fn undo_len(&self) -> usize {
        self.stack.undo_len()
    }

    pub fn redo_len(&self) -> usize {
        self.stack.redo_len()
    }

    pub fn undo_iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = SharedUndoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>>
    {
        self.stack.undo_iter()
    }

    pub fn redo_iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = SharedRedoEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>>
    {
        self.stack.redo_iter()
    }

    pub fn pop_first(&mut self) -> Option<PopEntry<OpGroupId, OpOwned, NonAlteringGraphOpHelper>> {
        self.stack.pop_first()
    }
}
