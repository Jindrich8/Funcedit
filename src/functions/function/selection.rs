use std::ops::{Range, RangeInclusive};

use enumflags2::BitFlags;

use crate::types::point::{Point, X, Y};

use super::{stats::Stats, Func};

#[derive(Debug)]
pub struct Selection {
    stats: Stats,
    value_range: RangeInclusive<X>,
}

impl Selection {
    pub fn new(range: RangeInclusive<X>, stats: Stats) -> Self {
        Self {
            stats,
            value_range: range,
        }
    }

    pub fn start_index(&self, points: &[Point]) -> usize {
        Func::x_to_points_index(points, *self.value_range.start())
    }

    pub fn end_index(&self, points: &[Point]) -> usize {
        (Func::x_to_points_index(points, *self.value_range.end()) + 1).min(points.len())
    }

    pub fn min(&self, points: &[Point]) -> Option<Y> {
        self.stats.min(&points)
    }

    pub fn max(&self, points: &[Point]) -> Option<Y> {
        self.stats.max(&points)
    }

    pub fn value_range(&self, points: &[Point]) -> Option<RangeInclusive<Y>> {
        self.stats.value_range(points)
    }

    pub fn index_range(&self, points: &[Point]) -> Range<usize> {
        self.start_index(points)..self.end_index(points)
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    pub fn stats_mut(&mut self) -> &mut Stats {
        &mut self.stats
    }

    pub fn change_selection(&mut self, new_selection: &RangeInclusive<X>, points: &[Point]) {
        self.value_range = new_selection.clone();
        let new_range = self.index_range(points);
        self.stats = Stats::new(new_range.start, new_range.start);
        if !new_range.is_empty() {
            self.stats
                .update_with_range(&points, &new_range, BitFlags::all());
        }
    }

    pub fn delete_selection(&mut self) {
        self.value_range = *self.value_range.start()..=(*self.value_range.start() - 1.0);
    }
}
