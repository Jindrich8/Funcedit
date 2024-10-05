use crate::types::{
    func_line::FuncLine,
    point::{Point, X, Y},
};

pub struct FuncCheckIter<Iter: Iterator<Item = Point>> {
    iter: Iter,
    last: Point,
}

impl<Iter: Iterator<Item = Point>> FuncCheckIter<Iter> {
    pub fn new(iter: Iter) -> Self {
        Self {
            iter,
            last: Point::new(X::NEG_INFINITY, Y::NAN),
        }
    }

    fn invalidate(&mut self) {
        self.last.x = X::INFINITY;
    }
}

impl<Iter: Iterator<Item = Point>> Iterator for FuncCheckIter<Iter> {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(point) = self.iter.next() {
            if !(point.x > self.last.x) || point.y.is_nan() {
                self.invalidate();
                return None;
            }
            if self.last.y.is_nan() {
                self.last = point;
                return Some(point);
            }
            let line = match FuncLine::new(&self.last, &point) {
                Ok(line) => line,
                Err(e) => {
                    self.invalidate();
                    return None;
                }
            };
            let x = self.last.x.trunc() + 1.0;
            let y = line.at(x);
            Some(Point::new(x, y))
        } else {
            None
        }
    }
}
