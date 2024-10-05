use crate::types::point::{Point, X, Y};

pub struct FuncValuesCheckIter<Iter: Iterator<Item = Y>> {
    iter: Iter,
    x: X,
}

impl<Iter: Iterator<Item = Y>> FuncValuesCheckIter<Iter> {
    pub fn new(iter: Iter, x: X) -> Self {
        Self {
            iter: iter,
            x: x.trunc(),
        }
    }

    fn invalidate(&mut self) {
        self.x = X::NAN;
    }
}

impl<Iter: Iterator<Item = Y>> Iterator for FuncValuesCheckIter<Iter> {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(y) = self.iter.next() {
            if y.is_nan() || !self.x.is_finite() {
                self.invalidate();
                return None;
            }

            let res = Some(Point::new(self.x, y));
            self.x += 1.0;
            res
        } else {
            None
        }
    }
}
