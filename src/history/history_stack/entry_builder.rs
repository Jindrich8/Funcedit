use crate::{shared_op::SharedOp, types::point::Y};

use super::{
    entry::Entry,
    shared_entry::{InOp, OtherOp},
    HistoryStack, IsGraphOpNonAltering, OpenOptions,
};

#[derive(Debug, Clone, PartialEq)]
pub enum EntryBuilderError {
    TooManyOps,
}

pub struct EntryBuilder<
    'a,
    OpGroupId: Clone + Default + PartialEq,
    OpOwned: OtherOp,
    NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
> {
    history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
    id: OpGroupId,
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > EntryBuilder<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn new(
        history: &'a mut HistoryStack<OpGroupId, OpOwned, NonAlteringGraphOpHelper>,
        id: OpGroupId,
    ) -> Self {
        Self { id, history }
    }

    pub fn add_other_op(&mut self, op: impl InOp<OpOwned, OpGroupId>) -> &mut Self {
        self.history.push_other_op(&self.id, op);
        self
    }

    pub fn add_graph_op<YExactIter: ExactSizeIterator<Item = Y> + Clone>(
        &mut self,
        op: SharedOp<
            impl Iterator<Item = usize> + Clone,
            impl Iterator<Item = YExactIter>,
            YExactIter,
        >,
    ) -> &mut Self
    where
        OpOwned: OtherOp,
    {
        self.history.push_graph_op(&self.id, op);
        self
    }
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > Drop for EntryBuilder<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    fn drop(&mut self) {
        self.history.close_new_entry(self.id.clone());
    }
}
