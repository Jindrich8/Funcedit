use std::iter::Copied;

use crate::types::point::{Point, Y};

use super::{
    func_check_iter::FuncCheckIter, func_range::FuncRange, func_safe_iter::FuncSafeIter,
    func_values_check_iter::FuncValuesCheckIter,
};

pub struct FuncSafeCopyIter<Iter: IntoIterator<Item = Point>> {
    iter: Iter,
}

impl<Iter: IntoIterator<Item = Point>> IntoIterator for FuncSafeCopyIter<Iter> {
    fn into_iter(self) -> <Iter as IntoIterator>::IntoIter {
        self.iter.into_iter()
    }

    type Item = Iter::Item;

    type IntoIter = Iter::IntoIter;
}

impl<'a, Iter: IntoIterator<Item = &'a Point>> From<FuncSafeIter<'a, Iter>>
    for FuncSafeCopyIter<Copied<<Iter as IntoIterator>::IntoIter>>
{
    fn from(value: FuncSafeIter<'a, Iter>) -> Self {
        Self {
            iter: value.into_iter().copied(),
        }
    }
}

impl<'a> From<FuncRange<'a>> for FuncSafeCopyIter<Copied<std::slice::Iter<'a, Point>>> {
    fn from(value: FuncRange<'a>) -> Self {
        Self {
            iter: value.points().into_iter().copied(),
        }
    }
}

impl<Iter: Iterator<Item = Point>> From<FuncCheckIter<Iter>>
    for FuncSafeCopyIter<FuncCheckIter<Iter>>
{
    fn from(value: FuncCheckIter<Iter>) -> Self {
        Self {
            iter: value.into_iter(),
        }
    }
}

impl<Iter: Iterator<Item = Y>> From<FuncValuesCheckIter<Iter>>
    for FuncSafeCopyIter<FuncValuesCheckIter<Iter>>
{
    fn from(value: FuncValuesCheckIter<Iter>) -> Self {
        Self { iter: value }
    }
}
