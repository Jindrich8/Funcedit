use crate::types::{
    func_line::{FuncLine, FuncLineErr},
    point::{Point, X, Y},
};

use super::{stats::Stats, Func};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuncBuilderError {
    XGoesBackwards,
    PointIsNotFinite,
}

#[derive(Debug)]
pub struct FuncBuilder {
    points: Vec<Point>,
    stats: Stats,
}

impl FuncBuilder {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            stats: Stats::new(0, 0),
        }
    }

    pub fn add_point(&mut self, point: &Point) -> Result<(), FuncBuilderError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(FuncBuilderError::PointIsNotFinite);
        }
        if let Some(last) = self.points.last() {
            if !(point.x > last.x) {
                return Err(FuncBuilderError::XGoesBackwards);
            }

            let line = match FuncLine::new(last, point) {
                Ok(line) => line,
                Err(e) => {
                    return Err(match e {
                        FuncLineErr::IsNotLine => FuncBuilderError::PointIsNotFinite,
                        FuncLineErr::LineIsNotFunction => FuncBuilderError::XGoesBackwards,
                    })
                }
            };

            let first = last.x.trunc() + 1.0;
            let mut i = 0;

            let add_count = (point.x - first).trunc().max(0.0) as usize + 1;
            if add_count >= 1 {
                if last.x.fract().abs() >= X::EPSILON {
                    let y = line.at(first);
                    let len = self.points.len();
                    self.points[len - 1] = Point::new(first, y);
                    i += 1;
                }

                self.points.reserve(add_count);
                for i in i..add_count {
                    let y = line.at(first);
                    self.add_valid_point(&Point::new(first + i as f64, y));
                }
            }
        } else {
            self.points.push(*point);
        }
        Ok(())
    }

    fn add_valid_point(&mut self, point: &Point) {
        let i = self.points.len();
        self.points.push(*point);
        self.stats.update(i, &self.points);
    }
}

impl Into<Func> for FuncBuilder {
    fn into(mut self) -> Func {
        if let Some(last) = self.points.last() {
            if last.x.fract().abs() < Y::EPSILON {
                self.points.truncate(self.points.len() - 1);
            }
        }
        Func::new_from(self.points)
    }
}
