use crate::types::{
    func_line::FuncLine,
    point::{Point, X, Y},
};
use enumflags2::{bitflags, BitFlags};

use super::Stretcher;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum YStretcherFlags {
    Bottom = 1,
    Top = 2,
}

/// Stretches distance of point's y component from y component of point on given line with same x component as given point
pub struct YStretcher {
    x_factor: X,
    y_factor: Y,
    add_factor: Y,
    flags: BitFlags<YStretcherFlags>,
}

impl YStretcher {
    /// start - together with end point define line  
    /// factor - the factor by which is stretched distance between y components
    pub fn new(line: &FuncLine, factor: Y, flags: BitFlags<YStretcherFlags>) -> Option<Self> {
        if flags.is_empty() || factor.is_sign_positive() && (factor - 1.0).abs() < Y::EPSILON {
            return None;
        }
        let mfactor = 1.0 - factor;
        Some(Self {
            x_factor: line.k() * mfactor,
            y_factor: factor,
            add_factor: line.q() * mfactor,
            flags,
        })
    }

    pub fn flags(&self) -> BitFlags<YStretcherFlags> {
        self.flags
    }
}

impl Stretcher<Point> for YStretcher {
    fn no_stretch() -> Self {
        Self {
            x_factor: 0.0,
            y_factor: 1.0,
            add_factor: 0.0,
            flags: BitFlags::all(),
        }
    }

    fn irreversible(&self) -> bool {
        self.y_factor.abs() < Y::EPSILON
    }

    fn stretches(&self) -> bool {
        (self.y_factor - 1.0).abs() >= Y::EPSILON
    }

    fn stretched(&self, item: &Point) -> Point {
        let mut point = item.clone();
        self.stretch(&mut point);
        point
    }

    fn stretch(&self, item: &mut Point) {
        item.y = item.y.mul_add(
            self.y_factor,
            item.x.mul_add(self.x_factor, self.add_factor),
        );
    }
}
