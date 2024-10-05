use crate::types::point::Point;

pub struct FuncRange<'a> {
    points: &'a [Point],
}

impl<'a> FuncRange<'a> {
    pub fn empty() -> Self {
        Self { points: &[] }
    }

    pub(super) fn new(points: &'a [Point]) -> Self {
        Self { points }
    }

    pub fn points(&self) -> &'a [Point] {
        self.points
    }
}
