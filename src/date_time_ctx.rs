use std::ops::RangeInclusive;

use chrono::{Datelike, Months, NaiveDate, NaiveDateTime, SubsecRound, TimeDelta, Timelike};
use egui_plot::{AxisHints, GridInput, GridMark};

use crate::{importer::DateTimeInfo, types::datetime_f64_mapper::DateTimeF64Mapper};

pub struct DateTimeCtx {
    pub info: DateTimeInfo,
    base_steps: Box<[f64]>,
    month_step_idx: u8,
    year_step_idx: u8,
}

impl DateTimeCtx {
    fn base_steps(&self) -> &[f64] {
        &self.base_steps
    }

    fn month_step_idx(&self) -> isize {
        self.month_step_idx as isize
    }

    fn year_step_idx(&self) -> isize {
        self.year_step_idx as isize
    }

    pub fn new(info: DateTimeInfo) -> Self {
        let mapper = &info.mapper;
        Self {
            base_steps: vec![
                mapper.time_delta_to_f64(&TimeDelta::seconds(1)),
                mapper.time_delta_to_f64(&TimeDelta::seconds(10)),
                mapper.time_delta_to_f64(&TimeDelta::minutes(1)),
                mapper.time_delta_to_f64(&TimeDelta::minutes(10)),
                mapper.time_delta_to_f64(&TimeDelta::hours(1)),
                mapper.time_delta_to_f64(&TimeDelta::hours(6)),
                mapper.time_delta_to_f64(&TimeDelta::days(1)),
                mapper.time_delta_to_f64(&TimeDelta::days(28)),
                mapper.time_delta_to_f64(&(TimeDelta::days(365) + TimeDelta::hours(6))),
            ]
            .into_boxed_slice(),
            info,
            month_step_idx: 7,
            year_step_idx: 8,
        }
    }

    pub fn x_axes(&self, base_step_size: f64) -> Vec<AxisHints> {
        let mapper = &self.info.mapper;
        let base_steps = self.base_steps();
        let axes = vec![
            AxisHints::new_x().formatter(
                move |mark: GridMark, r: &RangeInclusive<f64>| -> String {
                    let steps = [
                        mapper.time_delta_to_f64(&TimeDelta::seconds(1)),
                        mapper.time_delta_to_f64(&TimeDelta::minutes(1)),
                        mapper.time_delta_to_f64(&TimeDelta::hours(1)),
                        mapper.time_delta_to_f64(&TimeDelta::days(1)),
                        mapper.time_delta_to_f64(&TimeDelta::days(28)),
                        mapper.time_delta_to_f64(&(TimeDelta::days(365) + TimeDelta::hours(6))),
                    ];
                    let i = match steps.binary_search_by(|b| b.total_cmp(&base_step_size)) {
                        Ok(i) => i,
                        Err(i) => i,
                    };
                    let time = mapper.f64_to_time(mark.value);
                    let value_format = match i {
                        0 => "%H:%M:%S.%f",
                        1 => "%d %H:%M",
                        2 => "%d.%m %H",
                        3 => "%d.%m.%y",
                        4 => "%m.%y",
                        _ => "%y",
                    };
                    time.format("%T%n%d.%m.%y").to_string()
                },
            ),
            /* AxisHints::new_x().formatter(move |mark, r| {
                let steps = base_steps;
                let i = match steps.binary_search_by(|b| b.total_cmp(&base_step_size)) {
                    Ok(i) => i,
                    Err(i) => i,
                };
                if (mark.value - *r.start()) >= 2.0 * steps[i + 1]
                    && (*r.end() - mark.value) >= 2.0 * steps[i + 1]
                {
                    "".to_string()
                } else {
                    let new_steps = [
                        mapper.time_delta_to_f64(&TimeDelta::seconds(1)),
                        mapper.time_delta_to_f64(&TimeDelta::minutes(1)),
                        mapper.time_delta_to_f64(&TimeDelta::hours(1)),
                        mapper.time_delta_to_f64(&TimeDelta::days(1)),
                        mapper.time_delta_to_f64(&TimeDelta::days(28)),
                        mapper.time_delta_to_f64(&(TimeDelta::days(365) + TimeDelta::hours(6))),
                    ];
                    let j = match new_steps.binary_search_by(|b| b.total_cmp(&base_step_size)) {
                        Ok(j) => j,
                        Err(j) => j,
                    };
                    let time = mapper.f64_to_time(mark.value);
                    let value_format = match j {
                        0 => "%d.%m.%y %H:%M",
                        1 => "%d.%m.%y %H",
                        2 => "%d.%m.%y",
                        3 => "%m.%y",
                        4 => "%y",
                        _ => "",
                    };
                    time.format(value_format).to_string()
                }
            }) */
        ];
        axes
    }

    pub fn grid_spacer<'a>(&'a self) -> Box<dyn Fn(GridInput) -> Vec<GridMark> + 'a> {
        /// Fill in all values between [min, max] which are a multiple of `step_size`
        fn next_10power(value: f64) -> i32 {
            debug_assert_ne!(value, 0.0); // can be negative (typical for Y axis)
            10_i32.pow(value.abs().log10().ceil() as u32)
        }

        let mapper = &self.info.mapper;
        let base_steps = self.base_steps();
        let month_step_idx = self.month_step_idx();
        let year_step_idx = self.year_step_idx();
        let spacer = move |input: GridInput| {
            let mut marks: Vec<GridMark> = Vec::new();
            let bounds = input.bounds;
            let base_size = input.base_step_size;

            let mut curr_is = [0.0; 3];
            let mut steps: [f64; 3] = [0.0; 3];
            let mut year_steps: [i32; 3] = [0; 3];
            let min = bounds.0;
            let abs_eps = mapper.f64_delta_to_abs(1.0);
            let lower_date = mapper.f64_to_time(bounds.0);
            let mut curr_month: NaiveDateTime =
                NaiveDate::from_ymd_opt(lower_date.year(), lower_date.month(), 1)
                    .unwrap()
                    .into();
            if curr_month < lower_date {
                curr_month = curr_month.checked_add_months(Months::new(1)).unwrap();
            }

            let mut curr_years = [0; 3];

            let mut i: isize = match base_steps.binary_search_by(|b| b.total_cmp(&base_size)) {
                Ok(i) => i as isize,
                Err(i) => i as isize,
            };

            {
                let mut init_years = |j: usize, mut year_step: i32, curr_is: &mut [f64; 3]| {
                    for j in j..3 {
                        let mut curr_year = (lower_date.year() / year_step) * year_step;
                        let mut date: NaiveDateTime =
                            NaiveDate::from_yo_opt(curr_year, 1).unwrap().into();
                        if date < lower_date {
                            curr_year += year_step;
                            date = date.with_year(curr_year).unwrap();
                        }
                        year_steps[j] = year_step;
                        year_step *= 10;
                        curr_years[j] = curr_year;
                        curr_is[j] = mapper.time_to_f64(&date);
                    }
                };
                let curr = |step: f64| {
                    let l_abs = dbg!(DateTimeF64Mapper::time_to_abs(&lower_date));
                    let step_abs = dbg!(mapper.f64_delta_to_abs(step));
                    let div = dbg!(l_abs / step_abs);
                    let rem = dbg!(l_abs % step_abs);
                    let whole = dbg!(div + if rem > 0 { 1 } else { 0 });
                    mapper.abs_to_f64(dbg!(whole * step_abs))
                };
                if i == 0 || i == base_steps.len() as isize {
                    let base_step = base_steps[i as usize];
                    let base = base_size / base_step;
                    let next_power = next_10power(base);
                    let next = base_step * next_power as f64;

                    steps = [next, next * 10.0, next * 100.0];
                    let base_steps_overflow = next_power.checked_ilog10().unwrap() as isize;
                    if i == 0 {
                        curr_is = [curr(steps[0]), curr(steps[1]), curr(steps[2])];
                        i -= base_steps_overflow;
                    } else {
                        init_years(0_usize, next_power, &mut curr_is);
                        i += base_steps_overflow;
                    }
                } else {
                    let mut idx = 0;
                    let end_i = i as usize + 3;
                    let overflow = end_i.saturating_sub(base_steps.len());
                    for j in i as usize..(end_i - overflow) {
                        let step = base_steps[j];
                        steps[idx] = step;
                        curr_is[idx] = curr(step);
                        idx += 1;
                    }
                    if overflow > 0 {
                        init_years(2 - overflow, 1, &mut curr_is);
                    }
                }
            }

            let mut next = |j: isize, curr: &mut f64| {
                if j == month_step_idx {
                    *curr = mapper.time_to_f64(&curr_month);
                    curr_month = curr_month.checked_add_months(Months::new(1)).unwrap();
                } else if j == year_step_idx {
                    let idx = (j - i) as usize;
                    let curr_year = &mut curr_years[idx];
                    *curr =
                        mapper.time_to_f64(&NaiveDate::from_yo_opt(*curr_year, 1).unwrap().into());
                    *curr_year += year_steps[idx];
                } else {
                    let idx = (j - i) as usize;
                    *curr += steps[idx];
                }
            };

            while (bounds.1 - curr_is[0]) >= steps[0] {
                let step_idx: usize = if (curr_is[1] - curr_is[0]).abs() >= steps[0] {
                    0
                } else if (curr_is[2] - curr_is[1]).abs() >= steps[1] {
                    curr_is[0] = curr_is[1];
                    1
                } else {
                    curr_is[1] = curr_is[2];
                    curr_is[0] = curr_is[2];
                    2
                };
                marks.push(GridMark {
                    value: curr_is[0],
                    step_size: steps[step_idx],
                });
                for j in 0..(step_idx + 1) {
                    next(i + j as isize, &mut curr_is[j]);
                }
            }
            marks
        };
        Box::new(spacer)
    }
}
