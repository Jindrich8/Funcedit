use egui_plot::PlotBounds;
use std::ops::{Add, AddAssign};

use crate::utils::Change;

#[derive(Debug, Clone, PartialEq)]
pub struct PlotBoundsChange {
    min: [f64; 2],
    max: [f64; 2],
}

impl PlotBoundsChange {
    pub fn from_old_new(old: &PlotBounds, new_value: &PlotBounds) -> Self {
        Self {
            min: Self::subf64x2(&new_value.min(), &old.min()),
            max: Self::subf64x2(&new_value.max(), &old.max()),
        }
    }

    pub fn has_effect(&self) -> bool {
        !Self::is_approx_zero_f64x2(&self.min) || !Self::is_approx_zero_f64x2(&self.max)
    }

    pub fn undo_from(&self, bounds: &PlotBounds) -> PlotBounds {
        self.add(bounds)
    }

    pub fn redo_from(&self, bounds: &PlotBounds) -> PlotBounds {
        PlotBounds::from_min_max(
            PlotBoundsChange::subf64x2(&bounds.min(), &self.min),
            PlotBoundsChange::subf64x2(&bounds.max(), &self.max),
        )
    }

    fn is_approx_zero_f64x2(a: &[f64; 2]) -> bool {
        a[0].abs() < f64::EPSILON && a[1].abs() < f64::EPSILON
    }

    fn not_f64x2(a: &[f64; 2]) -> [f64; 2] {
        [-a[0], -a[1]]
    }

    fn not_assign_f64x2(a: &mut [f64; 2]) {
        a[0] = -a[0];
        a[1] = -a[1];
    }

    fn addf64x2(a: &[f64; 2], b: &[f64; 2]) -> [f64; 2] {
        [a[0] + b[0], a[1] + b[1]]
    }

    fn addf64x2_assign(a: &mut [f64; 2], b: &[f64; 2]) {
        a[0] += b[0];
        a[1] += b[1];
    }

    fn subf64x2(a: &[f64; 2], b: &[f64; 2]) -> [f64; 2] {
        [a[0] - b[0], a[1] - b[1]]
    }

    fn subf64x2_assign(a: &mut [f64; 2], b: &[f64; 2]) {
        a[0] -= b[0];
        a[1] -= b[1];
    }
}

impl Change for PlotBoundsChange {
    fn inverse(&mut self) {
        Self::not_assign_f64x2(&mut self.min);
        Self::not_assign_f64x2(&mut self.max);
    }
}

impl Add<&PlotBounds> for &PlotBoundsChange {
    type Output = PlotBounds;

    fn add(self, rhs: &PlotBounds) -> Self::Output {
        PlotBounds::from_min_max(
            PlotBoundsChange::addf64x2(&rhs.min(), &self.min),
            PlotBoundsChange::addf64x2(&rhs.max(), &self.max),
        )
    }
}

impl Add for PlotBoundsChange {
    type Output = PlotBoundsChange;

    fn add(self, rhs: PlotBoundsChange) -> Self::Output {
        (&self).add(&rhs)
    }
}

impl Add for &PlotBoundsChange {
    type Output = PlotBoundsChange;

    fn add(self, rhs: &PlotBoundsChange) -> Self::Output {
        PlotBoundsChange {
            min: PlotBoundsChange::addf64x2(&self.min, &rhs.min),
            max: PlotBoundsChange::addf64x2(&self.max, &rhs.max),
        }
    }
}

impl AddAssign<&PlotBoundsChange> for PlotBoundsChange {
    fn add_assign(&mut self, rhs: &PlotBoundsChange) {
        Self::addf64x2_assign(&mut self.min, &rhs.min);
        Self::addf64x2_assign(&mut self.max, &rhs.max);
    }
}

impl AddAssign for PlotBoundsChange {
    fn add_assign(&mut self, rhs: Self) {
        self.add_assign(&rhs);
    }
}
