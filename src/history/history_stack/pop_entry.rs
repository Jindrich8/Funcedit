use super::{
    shared_entry::{OtherOp, OwnedOp, SharedOutOp},
    HistoryStack, IsGraphOpNonAltering,
};

pub struct PopEntry<
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
    > PopEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
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

    pub fn id(&self) -> &OpGroupId {
        &self.history.entries[self.index].id
    }

    pub fn len(&self) -> usize {
        self.history.get_entry_op_range(self.index).len()
    }
}
impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > PopEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    pub fn iter<OpOut>(&'a self) -> impl Iterator<Item = SharedOutOp<'a, OpOut>>
    where
        OpOwned: OwnedOp<OpOut>,
    {
        let iter = self
            .history
            .ops
            .range(self.history.get_entry_op_range(self.index));
        iter.map(|o| o.get_shared())
    }
}

impl<
        'a,
        OpGroupId: Clone + Default + PartialEq,
        OpOwned: OtherOp,
        NonAlteringGraphOpHelper: IsGraphOpNonAltering<OpGroupId>,
    > Drop for PopEntry<'a, OpGroupId, OpOwned, NonAlteringGraphOpHelper>
{
    fn drop(&mut self) {
        self.history
            .ops
            .drain(self.history.get_entry_op_range(self.index));
        self.history.entries.pop_front();
    }
}
