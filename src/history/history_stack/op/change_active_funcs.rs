use crate::types::bit_set::{self, BitSet};

#[derive(Debug)]
pub struct ChangeActiveFuncs {
    changed: BitSet,
}

impl ChangeActiveFuncs {
    pub fn new<Iter: IntoIterator<Item = usize>>(iter: Iter) -> Option<Self> {
        let set: BitSet = iter.into_iter().into();
        if set.len() < 1 {
            return None;
        }
        Some(Self { changed: set })
    }

    pub fn len(&self) -> usize {
        self.changed.len()
    }

    pub fn toggle(&mut self, change: usize) {
        self.changed.toggle(change);
    }

    pub fn push(&mut self, change: usize) {
        self.changed.insert(change);
    }

    pub fn iter(&self) -> ChangeActiveFuncsIter {
        ChangeActiveFuncsIter::new(self.changed.iter())
    }
}

#[derive(Clone, Debug)]
pub struct ChangeActiveFuncsIter<'a> {
    iter: bit_set::Iter<'a>,
}

impl<'a> ChangeActiveFuncsIter<'a> {
    pub fn new(iter: bit_set::Iter<'a>) -> Self {
        Self { iter }
    }
}

impl<'a> Iterator for ChangeActiveFuncsIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for ChangeActiveFuncsIter<'a> {}

impl<'a> DoubleEndedIterator for ChangeActiveFuncsIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}
