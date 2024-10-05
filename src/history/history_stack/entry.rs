#[derive(Clone, Debug, Default)]
pub struct Entry<OpGroupId: Clone + Default> {
    pub op_end: usize,
    pub id: OpGroupId,
}
