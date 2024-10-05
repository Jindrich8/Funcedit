use std::{
    cmp::Ordering,
    mem::swap,
    ops::{Range, RangeInclusive},
};

use enumflags2::BitFlags;

use crate::types::point::{Point, Y};

use super::{BoundIndex, RecomputeStats};

#[derive(Debug, Clone)]
pub struct Stats {
    min_index: usize,
    max_index: usize,
}

impl Stats {
    pub fn new(min_index: usize, max_index: usize) -> Self {
        Self {
            min_index,
            max_index,
        }
    }

    pub fn set_max(&mut self, max_index: usize) {
        self.max_index = max_index;
    }

    pub fn set_min(&mut self, min_index: usize) {
        self.min_index = min_index;
    }

    pub fn set_bound(&mut self, index: &BoundIndex) {
        match index {
            BoundIndex::Max(max_index) => self.set_max(*max_index),
            BoundIndex::Min(min_index) => self.set_min(*min_index),
        }
    }

    pub fn update_with_range(
        &mut self,
        points: &[Point],
        range: &Range<usize>,
        recompute: BitFlags<RecomputeStats>,
    ) {
        let mut range = range.clone();
        if range.end > points.len() {
            range.end = points.len();
        }
        if recompute.is_all() {
            for i in range {
                self.update(i, points);
            }
        } else if recompute.contains(RecomputeStats::Min) {
            for i in range {
                self.update_min(i, points);
            }
        } else if recompute.contains(RecomputeStats::Max) {
            for i in range {
                self.update_max(i, points);
            }
        }
    }

    #[inline(always)]
    pub fn min_max_point_y(
        min_index: &mut usize,
        max_index: &mut usize,
        i: usize,
        points: &[Point],
    ) {
        let y = points[i].y;
        match y.total_cmp(&points[*min_index].y) {
            Ordering::Less => *min_index = i,
            Ordering::Equal => (),
            Ordering::Greater => {
                if y.total_cmp(&points[*max_index].y) == Ordering::Greater {
                    *max_index = i;
                }
            }
        };
    }

    pub fn update_max(&mut self, i: usize, points: &[Point]) {
        let max_index = &mut self.max_index;
        let y = points[i].y;
        if y.total_cmp(&points[*max_index].y) == Ordering::Greater {
            *max_index = i;
        }
    }

    pub fn update_min(&mut self, i: usize, points: &[Point]) {
        let min_index = &mut self.min_index;
        let y = points[i].y;
        if y.total_cmp(&points[*min_index].y) == Ordering::Less {
            *min_index = i;
        }
    }

    pub fn swap_min_max(&mut self, points: &[Point]) -> bool {
        if let (Some(min), Some(max)) = (self.min(points), self.max(points)) {
            if (max - min) < -Y::EPSILON {
                swap(&mut self.min_index, &mut self.max_index);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn update(&mut self, i: usize, points: &[Point]) {
        Self::min_max_point_y(&mut self.min_index, &mut self.max_index, i, points)
    }

    pub fn min_index(&self) -> usize {
        self.min_index
    }

    pub fn max_index(&self) -> usize {
        self.max_index
    }

    pub fn min<'a>(&self, points: &'a [Point]) -> Option<Y> {
        if let Some(point) = points.get(self.min_index) {
            Some(point.y)
        } else {
            None
        }
    }

    pub fn max<'a>(&self, points: &'a [Point]) -> Option<Y> {
        if let Some(point) = points.get(self.max_index) {
            Some(point.y)
        } else {
            None
        }
    }

    pub fn value_range<'a>(&self, points: &'a [Point]) -> Option<RangeInclusive<Y>> {
        if let (Some(Point { y: min, .. }), Some(Point { y: max, .. })) =
            (points.get(self.min_index), points.get(self.max_index))
        {
            Some(RangeInclusive::new(*min, *max))
        } else {
            None
        }
    }
}

impl Into<(usize, usize)> for Stats {
    fn into(self) -> (usize, usize) {
        (self.min_index, self.max_index)
    }
}

impl<'a> Into<(&'a usize, &'a usize)> for &'a Stats {
    fn into(self) -> (&'a usize, &'a usize) {
        (&self.min_index, &self.max_index)
    }
}
