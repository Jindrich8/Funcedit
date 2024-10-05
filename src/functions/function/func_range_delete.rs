use enumflags2::BitFlags;

use crate::types::point::Point;

use super::{Func, RecomputeStats};

pub struct FuncRangeDelete<'a> {
    func: &'a mut Func,
}
impl<'a> FuncRangeDelete<'a> {
    pub(super) fn new(func: &'a mut Func) -> Self {
        Self { func }
    }

    pub fn points(&self) -> &[Point] {
        &self.func.points_selection()
    }
}
impl<'a> Drop for FuncRangeDelete<'a> {
    fn drop(&mut self) {
        let indexes = &self.func.selection.index_range(&self.func.points);
        let points = &mut self.func.points;
        if indexes.is_empty() || indexes.start >= points.len() || indexes.end <= 0 {
            return;
        }

        let mut recompute = BitFlags::empty();
        if indexes.contains(&self.func.stats.max_index()) {
            recompute |= RecomputeStats::Max;
        }
        if indexes.contains(&self.func.stats.min_index()) {
            recompute |= RecomputeStats::Min;
        }
        let first_removed_x = points[indexes.start].x;
        points.drain(indexes.clone());
        if indexes.end < points.len() {
            let after = points[indexes.start].x;
            let diff = after - first_removed_x;
            for point in points[indexes.start..].iter_mut() {
                point.x -= diff;
            }
        }

        self.func.stats.update_with_range(
            &self.func.points,
            &(0..self.func.points.len()),
            recompute,
        );
        self.func.selection.delete_selection();
    }
}
