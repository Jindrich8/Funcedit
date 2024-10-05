use std::ops::RangeInclusive;

use enumflags2::{bitflags, BitFlags};

use crate::types::point::X;

use super::Stretcher;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum XStretcherFlags {
    Start = 1,
    End = 2,
}

pub struct XStretcher {
    stretch_factor: X,
    add_factor: X,
    flags: BitFlags<XStretcherFlags>,
}

impl XStretcher {
    pub fn flags(&self) -> BitFlags<XStretcherFlags> {
        self.flags
    }

    pub fn new(old: &RangeInclusive<X>, new_range: &RangeInclusive<X>) -> Option<Self> {
        let mut stretcher = Self::new_start(old, *new_range.start());

        if let Some(mut new_stretcher) = Self::new_end(old, *new_range.end()) {
            if let Some(start_stretcher) = stretcher {
                new_stretcher = start_stretcher.combine(new_stretcher);
            }
            stretcher = Some(new_stretcher);
        }
        stretcher
    }

    pub fn new_start(old: &RangeInclusive<X>, new_start: X) -> Option<Self> {
        if !(*old.end() >= new_start) {
            return None;
        }
        if (new_start - *old.start()).abs() < X::EPSILON {
            None
        } else {
            let stretch_factor = (*old.end() - new_start) / (*old.end() - *old.start());
            Some(Self {
                stretch_factor,
                add_factor: *old.end() * (1.0 - stretch_factor),
                flags: XStretcherFlags::Start.into(),
            })
        }
    }

    pub fn new_end(old: &RangeInclusive<X>, new_end: X) -> Option<Self> {
        if !(*old.start() <= new_end) {
            return None;
        }
        if (new_end - *old.end()).abs() < X::EPSILON {
            None
        } else {
            let stretch_factor = (new_end - *old.start()) / (*old.end() - *old.start());
            Some(Self {
                stretch_factor,
                add_factor: *old.start() * (1.0 - stretch_factor),
                flags: XStretcherFlags::End.into(),
            })
        }
    }

    pub fn combine(&self, other: Self) -> Self {
        Self {
            stretch_factor: self.stretch_factor * other.stretch_factor,
            add_factor: self.add_factor * other.stretch_factor + other.add_factor,
            flags: self.flags | other.flags,
        }
    }
}

impl Stretcher<X> for XStretcher {
    fn no_stretch() -> Self {
        Self {
            stretch_factor: 1.0,
            add_factor: 0.0,
            flags: BitFlags::all(),
        }
    }

    fn irreversible(&self) -> bool {
        self.stretch_factor.abs() < X::EPSILON
    }

    fn stretches(&self) -> bool {
        (self.stretch_factor - 1.0).abs() >= X::EPSILON
    }

    fn stretched(&self, x: &X) -> X {
        x * self.stretch_factor + self.add_factor
    }

    fn stretch(&self, x: &mut X) {
        *x = *x * self.stretch_factor + self.add_factor;
    }
}
