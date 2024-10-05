pub mod func_builder;
pub mod func_check_iter;
pub mod func_range;
pub mod func_range_delete;
pub mod func_safe_copy_iter;
pub mod func_safe_iter;
pub mod func_values_check_iter;
pub mod selection;
pub mod stats;

use egui_plot::{Line, PlotPoints};
use enumflags2::{bitflags, BitFlags};
use func_range::FuncRange;
use func_safe_copy_iter::FuncSafeCopyIter;
use selection::Selection;
use stats::Stats;
use std::{
    ops::{Range, RangeInclusive},
    usize,
};

use func_range_delete::FuncRangeDelete;

use crate::{
    stretchers::y_stretcher::YStretcherFlags,
    types::{
        func_line::FuncLine,
        point::{vector, Point, X, Y},
        skip_end_iterator::SkipEndIterator,
    },
};

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RecomputeStats {
    Min = 1,
    Max = 2,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuncInsertMode {
    Pattern,
    Values,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StretchType {
    Expand,
    Shrink,
    None,
}

#[derive(Debug, Clone)]
pub struct StretchY {
    pub factor: Y,
    pub flags: BitFlags<YStretcherFlags>,
}

impl StretchY {
    fn is_not_stretch(factor: Y, flags: BitFlags<YStretcherFlags>) -> bool {
        flags.is_empty()
            || !factor.is_finite()
            || factor.is_sign_positive() && (factor - 1.0).abs() < Y::EPSILON
    }

    pub fn stretch_type(&self) -> StretchType {
        if self.flags.is_empty() {
            return StretchType::None;
        }
        let diff = self.factor.abs() - 1.0;
        if !diff.is_finite() || diff.abs() < Y::EPSILON {
            return StretchType::None;
        }
        if diff.is_sign_positive() {
            StretchType::Expand
        } else {
            StretchType::Shrink
        }
    }

    pub fn stretches(&self) -> bool {
        !Self::is_not_stretch(self.factor, self.flags)
    }

    pub fn no_stretch() -> Self {
        Self {
            factor: 1.0,
            flags: BitFlags::empty(),
        }
    }

    pub fn new(factor: Y, flags: BitFlags<YStretcherFlags>) -> Option<Self> {
        if Self::is_not_stretch(factor, flags) {
            return None;
        }
        Some(Self { factor, flags })
    }
}

pub enum StretchYBoundsError {
    BoundsOutOfRange,
    Unstretchable,
}

pub struct StretchYBounds {
    min: Y,
    max: Y,
}

impl StretchYBounds {
    pub fn empty() -> Self {
        Self {
            min: Y::NAN,
            max: Y::NAN,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min.is_nan() && self.max.is_nan()
    }

    pub fn new(min: Y, max: Y) -> Self {
        Self { min, max }
    }

    pub fn new_top(max: Y) -> Self {
        Self { min: Y::NAN, max }
    }

    pub fn new_bottom(min: Y) -> Self {
        Self { min, max: Y::NAN }
    }

    pub fn new_both_by_max(max: Y) -> Self {
        Self {
            min: Y::NEG_INFINITY,
            max,
        }
    }

    pub fn new_both_by_min(min: Y) -> Self {
        Self {
            min,
            max: Y::INFINITY,
        }
    }

    pub fn flags(&self) -> BitFlags<YStretcherFlags> {
        let mut flags = BitFlags::empty();
        if !self.min.is_nan() {
            flags |= YStretcherFlags::Bottom;
        }
        if !self.max.is_nan() {
            flags |= YStretcherFlags::Top;
        }
        flags
    }

    pub fn min(&self) -> Y {
        self.min
    }

    pub fn max(&self) -> Y {
        self.max
    }

    pub fn set_min(&mut self, min: Y) {
        self.min = min;
    }

    pub fn set_max(&mut self, max: Y) {
        self.max = max;
    }
}

enum BoundIndex {
    Max(usize),
    Min(usize),
}

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum NewBoundSearchRangeFlag {
    IncludeBoundIndex,
    IncludeBoundPoints,
}

enum StretchYBound {
    Max(Y),
    Min(Y),
}
#[derive(Debug, Clone)]
pub struct FuncYValuesIter<'a> {
    points: std::slice::Iter<'a, Point>,
}

impl<'a> FuncYValuesIter<'a> {
    pub fn new(points: std::slice::Iter<'a, Point>) -> Self {
        Self { points }
    }
}

impl<'a> Iterator for FuncYValuesIter<'a> {
    type Item = Y;

    fn next(&mut self) -> Option<Self::Item> {
        self.points.next().and_then(|p| Some(p.y))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.points.size_hint()
    }
}

impl<'a> ExactSizeIterator for FuncYValuesIter<'a> {}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
enum FuncFlags {
    InvalidSelectionStats,
}

pub struct Func {
    points: Vec<Point>,
    selection: Selection,
    stats: Stats,
    flags: BitFlags<FuncFlags>,
}

pub enum FuncError {
    XGoesBackwards,
}

impl Func {
    pub fn new_from(points: Vec<Point>) -> Self {
        let mut stats = Stats::new(0, 0);
        stats.update_with_range(&points, &(0..points.len()), BitFlags::all());

        Self {
            selection: Selection::new((0.0..=-1.0).into(), Stats::new(0, 0)),
            stats,
            points,
            flags: FuncFlags::InvalidSelectionStats.into(),
        }
    }

    pub fn values(&self) -> FuncYValuesIter {
        FuncYValuesIter::new(self.points().iter())
    }

    pub fn values_selections(&self) -> FuncYValuesIter {
        FuncYValuesIter::new(self.points_selection().iter())
    }

    pub fn points_selection(&self) -> &[Point] {
        &self.points[self.selection_index_range()]
    }

    pub fn value_range(&self) -> Option<RangeInclusive<Y>> {
        self.stats.value_range(&self.points)
    }

    pub fn min(&self) -> Option<Y> {
        self.stats.min(&self.points)
    }

    pub fn max(&self) -> Option<Y> {
        self.stats.max(&self.points)
    }

    pub fn points(&self) -> &[Point] {
        &self.points
    }

    pub fn selection_min(&self) -> Option<Y> {
        self.selection.min(&self.points)
    }

    pub fn selection_max(&self) -> Option<Y> {
        self.selection.max(&self.points)
    }

    pub fn selection_value_range(&self) -> Option<RangeInclusive<Y>> {
        self.selection.value_range(&self.points)
    }

    pub fn new() -> Self {
        Self::new_from(Vec::new())
    }

    pub fn change_selection(&mut self, new_selection: &RangeInclusive<X>) {
        self.selection.change_selection(new_selection, &self.points);
    }

    fn update_stats_with_selection(&mut self) {
        self.stats
            .update_max(self.selection.stats().max_index(), &self.points);
        self.stats
            .update_min(self.selection.stats().min_index(), &self.points);
    }

    pub fn line(&self) -> Line {
        let points = PlotPoints::Owned(self.points.clone());
        let line = Line::new(points);
        line
    }

    pub fn delete<'a>(&'a mut self) -> FuncRangeDelete<'a> {
        FuncRangeDelete::new(self)
    }

    pub fn index_of(&self, x: X) -> usize {
        self.x_to_index(x)
    }

    fn insert<I: IntoIterator<Item = Point>>(&mut self, points: I) -> FuncRange {
        let mut points = points.into_iter();
        if let Some(first) = points.next() {
            let x = first.x;
            let index = self.x_to_index(x);
            let count = self.points.len();
            if count < 1 {
                self.points.extend(points);
            } else {
                self.points
                    .splice(index..index, [first].into_iter().chain(points));
                let inserted = self.points.len() - count;
                if index == 0 || index == count {
                    let add = if index == 0 {
                        let last_inserted_x = self.points[index + inserted - 1].x;
                        let after = self.points[index + inserted].x;
                        after - last_inserted_x - 1.0
                    } else {
                        let first_inserted_x = self.points[index].x;
                        let before = self.points[index - 1].x;
                        before - first_inserted_x + 1.0
                    };
                    if add.abs() >= X::EPSILON {
                        self.points[index..index + inserted]
                            .iter_mut()
                            .for_each(|p| p.x += add);
                    }
                }
                if inserted > 0 {
                    let last_inserted_x = self.points[index + inserted - 1].x;
                    self.points[index + inserted..].iter_mut().for_each(|p| {
                        p.x += last_inserted_x - x;
                    });
                }
            }

            let inserted = self.points.len() - count;
            let inserted_range = index..(index + inserted);
            self.stats
                .update_with_range(&self.points, &inserted_range, BitFlags::all());
            self.selection.stats_mut().update_with_range(
                &self.points,
                &inserted_range,
                BitFlags::all(),
            );
            FuncRange::new(&self.points[inserted_range])
        } else {
            FuncRange::new(&[])
        }
    }

    pub fn insert_values<I: IntoIterator<Item = Point>>(
        &mut self,
        points: FuncSafeCopyIter<I>,
    ) -> FuncRange {
        self.insert(points)
    }

    pub fn insert_pattern<I: IntoIterator<Item = Point>>(
        &mut self,
        points: FuncSafeCopyIter<I>,
    ) -> FuncRange {
        let mut points = points.into_iter();
        if let Some(first) = points.next() {
            let x = first.x;
            let index = self.x_to_index(x);
            let count = self.points.len();
            if count < 1 {
                self.points.extend(points);
            } else {
                let y_add = if index == 0 {
                    let mut last = first;
                    let y_after = self.points[0].y;
                    let points = SkipEndIterator::new(points, &mut last);
                    self.points.splice(index..index, points);
                    y_after - last.y
                } else {
                    let y_before = self.points[index - 1].y;
                    self.points.splice(index..index, points);
                    y_before - first.y
                };

                let inserted = self.points.len() - count;
                if index == 0 || index == count {
                    let add = if index == 0 {
                        let last_inserted_x = self.points[index + inserted - 1].x;
                        let after = self.points[index + inserted].x;
                        after - last_inserted_x - 1.0
                    } else {
                        let first_inserted_x = self.points[index].x;
                        let before = self.points[index - 1].x;
                        before - first_inserted_x + 1.0
                    };
                    if add.abs() >= X::EPSILON {
                        self.points[index..index + inserted]
                            .iter_mut()
                            .for_each(|p| p.x += add);
                    }
                }

                if inserted > 0 {
                    if y_add.abs() >= Y::EPSILON {
                        self.points[index..(index + inserted)]
                            .iter_mut()
                            .for_each(|p| {
                                p.y += y_add;
                            });
                    }

                    let last_inserted_x = self.points[index + inserted - 1].x;

                    self.points[index + inserted..].iter_mut().for_each(|p| {
                        p.x += last_inserted_x - x;
                    });
                }
            }

            let inserted = self.points.len() - count;
            let inserted_range = index..(index + inserted);
            self.stats
                .update_with_range(&self.points, &inserted_range, BitFlags::all());
            self.selection.stats_mut().update_with_range(
                &self.points,
                &inserted_range,
                BitFlags::all(),
            );
            FuncRange::new(&self.points[inserted_range])
        } else {
            FuncRange::new(&[])
        }
    }

    pub fn min_y_stretch_factor_for_bounds(
        &self,
        bounds: &StretchYBounds,
    ) -> Result<Y, StretchYBoundsError> {
        match self.min_y_stretch_factor_and_index_for_bounds(bounds) {
            Ok((factor, _)) => Ok(factor),
            Err(e) => Err(e),
        }
    }

    fn min_y_stretch_factor_and_index_for_bounds(
        &self,
        bounds: &StretchYBounds,
    ) -> Result<(Y, BoundIndex), StretchYBoundsError> {
        if !self.is_selection_stretchable() {
            return Err(StretchYBoundsError::Unstretchable);
        }
        let sel_max = self.selection_max();
        let sel_min = self.selection_min();

        if let (Some(sel_max), Some(sel_min)) = (sel_max, sel_min) {
            if (sel_max - sel_min) < Y::EPSILON {
                return Err(StretchYBoundsError::Unstretchable);
            }
        } else {
            return Err(StretchYBoundsError::Unstretchable);
        }

        let flags = bounds.flags();
        let min = bounds.min();
        let max = bounds.max();
        let mut bound_index = BoundIndex::Min(0);
        let factor = if flags.is_all() {
            let mut factor: Y = Y::INFINITY;
            if min != Y::NEG_INFINITY {
                let min_index;
                (factor, min_index) =
                    self.find_min_y_stretch_factor_and_index_for_bound(&StretchYBound::Min(min))?;
                bound_index = BoundIndex::Min(min_index);
            }
            if max != Y::INFINITY {
                let (factor_max, max_index) =
                    self.find_min_y_stretch_factor_and_index_for_bound(&StretchYBound::Max(max))?;
                if (factor_max - 1.0).abs() < (factor - 1.0).abs() {
                    factor = factor_max;
                    bound_index = BoundIndex::Max(max_index);
                }
            }
            if !factor.is_finite() {
                return Err(StretchYBoundsError::BoundsOutOfRange);
            }
            factor
        } else if flags.contains(YStretcherFlags::Top) {
            let (factor, index) =
                self.find_min_y_stretch_factor_and_index_for_bound(&StretchYBound::Max(max))?;
            bound_index = BoundIndex::Max(index);
            factor
        } else if flags.contains(YStretcherFlags::Bottom) {
            let (factor, index) =
                self.find_min_y_stretch_factor_and_index_for_bound(&StretchYBound::Min(min))?;
            bound_index = BoundIndex::Min(index);
            factor
        } else {
            1.0
        };
        Ok((factor, bound_index))
    }

    fn new_bound_search_range(
        &self,
        bound_index: usize,
        k: Y,
        stretches_up: bool,
        flags: impl Into<BitFlags<NewBoundSearchRangeFlag>>,
    ) -> Range<usize> {
        let flags = flags.into();
        let bound_index_add = if flags.contains(NewBoundSearchRangeFlag::IncludeBoundIndex) {
            0
        } else {
            1
        };
        let bound_points_add = if flags.contains(NewBoundSearchRangeFlag::IncludeBoundPoints) {
            0
        } else {
            1
        };
        let range = self.selection_index_range();
        let search_range = {
            let left_range = (range.start + bound_points_add)..(bound_index + 1 - bound_index_add);
            let right_range = (bound_index + bound_index_add)..(range.end - bound_points_add);

            // If maximum is one of the bounding points, we pick range without it
            if left_range.is_empty() {
                left_range.end..right_range.end
            } else if right_range.is_empty() {
                left_range.start..right_range.start
            } else if k.is_sign_positive() == stretches_up {
                left_range
            } else {
                right_range
            }
        };
        search_range
    }

    fn find_min_y_stretch_factor_and_index_for_bound(
        &self,
        bound: &StretchYBound,
    ) -> Result<(Y, usize), StretchYBoundsError> {
        let range = self.selection_index_range();
        if !self.is_selection_stretchable() {
            return Err(StretchYBoundsError::Unstretchable);
        }
        let first = &self.points[range.start];
        let last = &self.points[range.end - 1];
        let line = match self.y_stretch_line() {
            Some(line) => line,
            None => return Err(StretchYBoundsError::BoundsOutOfRange),
        };
        let k = line.k();
        let q = line.q();
        let factor = match bound {
            StretchYBound::Max(max) => {
                // Cannot stretch to infinity or nan, and also cannot stretch maximum to anything smaller
                // than maximum of bounds point y components, because these bound points have distance from line 0
                // and 0 multiplied by any finite number results into 0.
                if !max.is_finite() || (max - first.y.max(last.y)) < -Y::EPSILON {
                    return Err(StretchYBoundsError::BoundsOutOfRange);
                }
                let max_index = self.selection.stats().max_index();
                let sel_max = match self.selection_max() {
                    Some(sel_max) => sel_max,
                    None => return Err(StretchYBoundsError::BoundsOutOfRange),
                };
                // > 0 - stretching out
                // < 0 - contraction
                let diff_max = max - sel_max;

                // If  current maximum is same as new maximum, no stretch is needed, so factor is 1.0
                if diff_max.abs() < Y::EPSILON {
                    return Ok((1.0, max_index));
                }
                let is_stretch = diff_max.is_sign_positive();
                // If line is parallel to x-axis,
                // then point with maximum y is going to have maximum y after any stretching.
                if k.abs() < Y::EPSILON {
                    ((max - q) / (self.points[max_index].y - q), max_index)
                } else {
                    // If line is not parallel to x-axis, then after some stretching there may be different point,
                    // whose y component is maximum
                    let search_range = self.new_bound_search_range(
                        max_index,
                        k,
                        is_stretch,
                        NewBoundSearchRangeFlag::IncludeBoundIndex,
                    );
                    let points = &self.points[search_range];
                    if is_stretch {
                        match Self::find_min_y_stretch_factor_and_index(
                            *max,
                            &line,
                            points
                                .iter()
                                .enumerate()
                                .filter(|(i, p)| (p.y - line.at(p.x)) >= Y::EPSILON),
                        ) {
                            None => return Err(StretchYBoundsError::Unstretchable),
                            Some(factor) => factor,
                        }
                    } else {
                        match Self::find_min_y_stretch_factor_and_index(
                            *max,
                            &line,
                            points
                                .iter()
                                .enumerate()
                                .filter(|(i, p)| (p.y - max) > -Y::EPSILON),
                        ) {
                            None => return Err(StretchYBoundsError::Unstretchable),
                            Some(factor) => factor,
                        }
                    }
                }
            }
            StretchYBound::Min(min) => {
                if !min.is_finite() || (min - first.y.min(last.y)) >= Y::EPSILON {
                    return Err(StretchYBoundsError::BoundsOutOfRange);
                }
                let min_index = self.selection.stats().min_index();
                let sel_min = match self.selection_min() {
                    Some(sel_min) => sel_min,
                    None => return Err(StretchYBoundsError::Unstretchable),
                };
                // > 0 - stretching out
                // < 0 - contraction
                let diff_min = sel_min - min;

                if diff_min.abs() < Y::EPSILON {
                    return Ok((1.0, min_index));
                }
                let is_stretch = diff_min.is_sign_positive();

                if k.abs() < Y::EPSILON {
                    ((min - q) / (self.points[min_index].y - q), min_index)
                } else {
                    let search_range = self.new_bound_search_range(
                        min_index,
                        k,
                        !is_stretch,
                        NewBoundSearchRangeFlag::IncludeBoundIndex,
                    );
                    let points = &self.points[search_range];
                    if is_stretch {
                        match Self::find_min_y_stretch_factor_and_index(
                            *min,
                            &line,
                            points
                                .iter()
                                .enumerate()
                                .filter(|(i, p)| (line.at(p.x) - p.y) >= Y::EPSILON),
                        ) {
                            None => return Err(StretchYBoundsError::Unstretchable),
                            Some(factor) => factor,
                        }
                    } else {
                        match Self::find_min_y_stretch_factor_and_index(
                            *min,
                            &line,
                            points
                                .iter()
                                .enumerate()
                                .filter(|(i, p)| (min - p.y) > -Y::EPSILON),
                        ) {
                            None => return Err(StretchYBoundsError::Unstretchable),
                            Some(factor) => factor,
                        }
                    }
                }
            }
        };
        Ok(factor)
    }

    /// Returns whether points were modified
    pub fn stretch_y(&mut self, bounds: &StretchYBounds) -> Result<bool, StretchYBoundsError> {
        let (factor, index) = self.min_y_stretch_factor_and_index_for_bounds(bounds)?;
        let flags = bounds.flags();
        if let Some(stretch) = StretchY::new(factor, flags) {
            let is_stretch = stretch.stretch_type() == StretchType::Expand;
            if let Some(line) = self.y_stretch_line() {
                if self.stretch_y_with_factor_and_line_no_stats_update(&stretch, &line) {
                    self.selection.stats_mut().set_bound(&index);
                    if flags.is_all() {
                        let (search_index, recompute) = match index {
                            BoundIndex::Max(_) => {
                                (self.selection.stats().min_index(), RecomputeStats::Min)
                            }
                            BoundIndex::Min(_) => {
                                (self.selection.stats().max_index(), RecomputeStats::Max)
                            }
                        };
                        let stretches_up = is_stretch == (recompute == RecomputeStats::Max);
                        let search_range = self.new_bound_search_range(
                            search_index,
                            line.k(),
                            stretches_up,
                            NewBoundSearchRangeFlag::IncludeBoundPoints,
                        );
                        self.selection.stats_mut().update_with_range(
                            &self.points,
                            &search_range,
                            recompute.into(),
                        );
                    }
                    self.update_stats_with_selection();
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn find_min_y_stretch_factor_and_index<'a, Iter: IntoIterator<Item = (usize, &'a Point)>>(
        m: f64,
        line: &FuncLine,
        iter: Iter,
    ) -> Option<(f64, usize)> {
        dbg!(iter
            .into_iter()
            .map(|(i, p)| {
                let ly = line.at(p.x);

                (i, (m - ly) / (p.y - ly))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b)))
        .and_then(|(i, f)| Some((f, i)))
    }

    fn stretch_y_points_with_factor<'a, Iter: IntoIterator<Item = &'a mut Point>>(
        points: Iter,
        line: &FuncLine,
        factor: Y,
    ) {
        points.into_iter().enumerate().for_each(|(_i, p)| {
            let ly = line.at(p.x);
            p.y = factor.mul_add(p.y - ly, ly);
        });
    }

    fn y_stretch_line(&self) -> Option<FuncLine> {
        let range = self.selection_index_range();
        if self.points.len() < 2 || range.len() < 2 {
            return None;
        }
        let start = &self.points[range.start];
        let end = &self.points[range.end - 1];
        // This should be safe, because if start and end are not valid function points,
        // then somewhere is bug, which needs to be fixed.
        // Otherwise this as whole doesn't matter.
        Some(FuncLine::new(start, end).expect("Function is compromised!\nIt contains invalid points!\nThis means that whole application could be compromised!"))
    }

    /// Returns whether points were modified
    pub fn stretch_y_with_factor(&mut self, stretch: &StretchY) -> bool {
        let is_stretch = match stretch.stretch_type() {
            StretchType::Expand => true,
            StretchType::Shrink => false,
            StretchType::None => return false,
        };
        if let Some(line) = self.y_stretch_line() {
            if self.stretch_y_with_factor_and_line_no_stats_update(stretch, &line) {
                let flags = stretch.flags;
                let is_factor_negative = stretch.factor.is_sign_negative();
                let mut negate_recompute = false;
                if is_factor_negative {
                    if flags.is_all() {
                        self.selection.stats_mut().swap_min_max(&self.points);
                    } else {
                        negate_recompute = true;
                    }
                }

                let mut len = 0;
                let mut recompute = [(RecomputeStats::Min, 0); 2];

                if flags.contains(YStretcherFlags::Top) {
                    recompute[len] = (RecomputeStats::Max, self.selection.stats().max_index());
                    len += 1;
                }
                if flags.contains(YStretcherFlags::Bottom) {
                    recompute[len] = (RecomputeStats::Min, self.selection.stats().min_index());
                    len += 1;
                }
                for (stats_type, bound_index) in recompute[0..len].into_iter() {
                    let stretches_up = is_stretch == (*stats_type == RecomputeStats::Max);
                    let search_range = self.new_bound_search_range(
                        *bound_index,
                        line.k(),
                        stretches_up,
                        BitFlags::all(),
                    );
                    let mut recompute: BitFlags<RecomputeStats> = stats_type.clone().into();
                    if negate_recompute {
                        recompute = !recompute;
                    }

                    self.selection.stats_mut().update_with_range(
                        &self.points,
                        &search_range,
                        recompute,
                    );
                }
                if negate_recompute {
                    let range = self.selection_index_range();
                    let stats = self.selection.stats_mut();
                    if flags.contains(YStretcherFlags::Top) {
                        stats.set_max(if line.k() >= Y::EPSILON {
                            range.end.saturating_sub(1)
                        } else {
                            range.start
                        });
                    } else if flags.contains(YStretcherFlags::Bottom) {
                        stats.set_min(if line.k() >= Y::EPSILON {
                            range.start
                        } else {
                            range.end.saturating_sub(1)
                        });
                    }
                }
                self.update_stats_with_selection();
                return true;
            }
        }
        false
    }

    /// Returns whether points were modified
    fn stretch_y_with_factor_and_line_no_stats_update(
        &mut self,
        stretch: &StretchY,
        line: &FuncLine,
    ) -> bool {
        let StretchY { factor, flags } = stretch;
        if stretch.stretches() {
            let range = self.selection_index_range();
            let iter = self.points[range].iter_mut();
            if flags.is_all() {
                Self::stretch_y_points_with_factor(iter, &line, *factor)
            } else if flags.contains(YStretcherFlags::Top) {
                Self::stretch_y_points_with_factor(
                    iter.filter(|p| p.y > line.at(p.x)),
                    &line,
                    *factor,
                )
            } else {
                Self::stretch_y_points_with_factor(
                    iter.filter(|p| p.y < line.at(p.x)),
                    &line,
                    *factor,
                )
            }
            return true;
        } else {
            return false;
        }
    }

    fn is_selection_stretchable(&self) -> bool {
        let range = self.selection_index_range();
        if self.points.len() <= 2 || range.len() <= 2 {
            return false;
        }
        true
    }

    /// Returns whether points were modified
    fn stretch_y_with_factor_no_stats_update(&mut self, stretch: &StretchY) -> bool {
        if self.is_selection_stretchable() {
            if let Some(line) = self.y_stretch_line() {
                return self.stretch_y_with_factor_and_line_no_stats_update(stretch, &line);
            }
        }
        false
    }
}

impl Func {
    fn x_to_points_index(points: &[Point], x: X) -> usize {
        if let Some(first) = points.first() {
            (x - first.x).trunc().max(0.0).min(points.len() as f64) as usize
        } else {
            0
        }
    }

    fn x_to_index(&self, x: X) -> usize {
        Self::x_to_points_index(&self.points, x)
    }

    fn selection_index_range(&self) -> Range<usize> {
        self.selection.index_range(&self.points)
    }

    fn points_on_same_line(a: &Point, b: &Point, c: &Point) -> bool {
        let u = vector(a, c);
        let v = vector(a, b);
        u.1 * v.0 == v.1 * u.0
    }

    fn insert_points<
        'a,
        Iter: ExactSizeIterator<Item = &'a (Point, usize)>,
        IntoIter: IntoIterator<IntoIter = Iter, Item = &'a (Point, usize)>,
    >(
        &mut self,
        points_and_pos: IntoIter,
    ) {
        // TODO: OPTIMALIZE THIS
        let iter = points_and_pos.into_iter();
        self.points.reserve(iter.len());
        iter.for_each(|(point, index)| self.points.insert(*index, *point));
    }
}
