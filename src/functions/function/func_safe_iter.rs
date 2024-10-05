use crate::types::point::Point;

use super::func_range::FuncRange;

pub struct FuncSafeIter<'a, Iter: IntoIterator<Item = &'a Point>> {
    iter: Iter,
}

impl<'a, Iter: IntoIterator<Item = &'a Point>> IntoIterator for FuncSafeIter<'a, Iter> {
    fn into_iter(self) -> <Iter as IntoIterator>::IntoIter {
        self.iter.into_iter()
    }

    type Item = Iter::Item;

    type IntoIter = Iter::IntoIter;
}

impl<'a> From<FuncRange<'a>> for FuncSafeIter<'a, &'a [Point]> {
    fn from(value: FuncRange<'a>) -> Self {
        Self {
            iter: value.points(),
        }
    }
}
