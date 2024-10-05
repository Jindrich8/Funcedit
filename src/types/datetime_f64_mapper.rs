use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike, Utc};

pub enum DateTimePrecision {
    Seconds(u8),
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone)]
pub struct DateTimeF64Mapper {
    zero: NaiveDateTime,
    precision: f64,
}

impl DateTimeF64Mapper {
    const NANOS_IN_SECOND_U128: i128 = 1000_000_000;
    const NANOS_IN_SECOND_F64: f64 = 1000_000_000.0;
    const SECS_IN_DAY_U128: i128 = 60 * 60 * 24;
    pub fn new(zero: NaiveDateTime, precision: DateTimePrecision) -> Self {
        Self {
            zero,
            precision: match precision {
                DateTimePrecision::Seconds(places) => 10_usize.pow(places.into()) as f64,
                DateTimePrecision::Minutes => 1.0 / 60.0,
                DateTimePrecision::Hours => 1.0 / (60.0 * 60.0),
                DateTimePrecision::Days => 1.0 / (60.0 * 60.0 * 24.0),
            },
        }
    }

    pub fn f64_to_time_delta(&self, mut f: f64) -> TimeDelta {
        f /= self.precision;
        let seconds = f.trunc();
        let nanos = (f.fract() * Self::NANOS_IN_SECOND_F64 as f64).trunc();
        TimeDelta::seconds(seconds as i64) + TimeDelta::nanoseconds(nanos as i64)
    }

    pub fn time_delta_to_f64(&self, delta: &TimeDelta) -> f64 {
        let mut value = (delta.num_seconds() as f64)
            + (delta.subsec_nanos() as f64 / Self::NANOS_IN_SECOND_F64);
        value *= self.precision;
        value
    }

    pub fn f64_to_time(&self, f: f64) -> NaiveDateTime {
        self.zero + self.f64_to_time_delta(f)
    }

    pub fn time_to_f64(&self, t: &NaiveDateTime) -> f64 {
        let delta = *t - self.zero;
        self.time_delta_to_f64(&delta)
    }

    pub fn time_to_abs(t: &NaiveDateTime) -> i128 {
        let t = t.and_utc();
        let res =
            t.timestamp() as i128 * Self::NANOS_IN_SECOND_U128 + t.timestamp_subsec_nanos() as i128;
        res
    }

    pub fn delta_to_abs(delta: &TimeDelta) -> i128 {
        delta.num_seconds() as i128 * Self::NANOS_IN_SECOND_U128 + delta.subsec_nanos() as i128
    }

    pub fn abs_to_time(abs: i128) -> NaiveDateTime {
        let nanos = abs % Self::NANOS_IN_SECOND_U128;
        DateTime::<Utc>::from_timestamp(
            (abs / Self::NANOS_IN_SECOND_U128 - if nanos < 0 { 1 } else { 0 }) as i64,
            nanos.abs() as u32,
        )
        .unwrap()
        .naive_utc()
    }

    pub fn abs_to_delta(abs: i128) -> TimeDelta {
        TimeDelta::seconds((abs / Self::NANOS_IN_SECOND_U128) as i64)
            + TimeDelta::nanoseconds((abs % Self::NANOS_IN_SECOND_U128) as i64)
    }

    pub fn f64_delta_to_abs(&self, delta: f64) -> i128 {
        Self::delta_to_abs(&self.f64_to_time_delta(delta))
    }

    pub fn f64_to_abs(&self, f: f64) -> i128 {
        Self::time_to_abs(&self.f64_to_time(f))
    }
    pub fn abs_delta_to_f64(&self, delta: i128) -> f64 {
        self.time_delta_to_f64(&Self::abs_to_delta(delta))
    }

    pub fn abs_to_f64(&self, abs: i128) -> f64 {
        self.time_to_f64(&Self::abs_to_time(abs))
    }
}
