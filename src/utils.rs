use std::{
    fmt::Debug,
    ops::{Add, RangeBounds},
};

use crate::ui::history::ApplyDataOp;

#[macro_export]
macro_rules! if_debug {
    ($ifex:expr ; else $elseex:expr) => {
        if cfg!(debug_assertions) {
            $ifex
        } else {
            $elseex
        }
    };
    ($ifex:expr) => {
        if cfg!(debug_assertions) {
            $ifex
        }
    };
}

pub fn get_value<'a, T>(result: &'a Result<T, T>) -> &'a T {
    match result {
        Ok(value) => value,
        Err(value) => value,
    }
}

pub trait Changeable<Change> {
    fn get_change(&self, new_value: &Self) -> Change;

    fn change(&mut self, change: &Change);

    fn apply_change(&mut self, change: ApplyDataOp<&Change>)
    where
        Change: Debug + PartialEq + Clone;
}

pub trait Change {
    fn inverse(&mut self);
}
