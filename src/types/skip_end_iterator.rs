use std::mem::swap;

pub struct SkipEndIterator<'a, Iter: Iterator<Item = Item>, Item: Clone> {
    iter: Iter,
    current: &'a mut Item,
}

impl<'a, Iter: Iterator<Item = Item>, Item: Clone> SkipEndIterator<'a, Iter, Item> {
    pub fn new(iter: Iter, current: &'a mut Item) -> Self {
        Self { current, iter }
    }
}

impl<'a, Iter: Iterator<Item = Item>, Item: Clone> Iterator for SkipEndIterator<'a, Iter, Item> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut next) = self.iter.next() {
            swap(self.current, &mut next);
            Some(next)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iter.size_hint();
        (
            min.saturating_sub(1),
            max.and_then(|max| Some(max.saturating_sub(1))),
        )
    }
}
