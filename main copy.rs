// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)]

use core::f64;
use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    fmt::Write,
    ops::{Range, RangeBounds, RangeInclusive},
    str::FromStr,
};

use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
// it's an example
use eframe::{
    egui::{
        self, widgets, Checkbox, Color32, Pos2, Rect, Response, Rgba, Rounding, ScrollArea,
        Separator, Slider, TextEdit, Ui, Vec2, Vec2b,
    },
    epaint::RectShape,
};
use egui_plot::{Line, Plot};

macro_rules! ftcmp {
    () => {
        |a, b| a.total_cmp(b)
    };
}

fn main() -> eframe::Result {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Plots",
        options,
        Box::new(|cc| {
            // Use the dark theme
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Ok(Box::<MyApp>::default())
        }),
    )
}

struct Func<X, Y: Copy> {
    x: Vec<X>,
    y: Vec<Y>,
    y_max_idx: usize,
    y_min_idx: usize,
}

impl<X, Y: Copy> Func<X, Y> {
    fn y_range(&self) -> RangeInclusive<Y> {
        self.min_y()..=self.max_y()
    }

    fn min_y(&self) -> Y {
        self.y[self.y_min_idx].clone()
    }

    fn max_y(&self) -> Y {
        self.y[self.y_max_idx].clone()
    }
}

impl Func<f64, f64> {
    pub fn new(x: Vec<f64>, y: Vec<f64>, selection: &RangeInclusive<f64>) -> Self {
        let mut func = Self {
            x,
            y,
            y_min_idx: 0,
            y_max_idx: 0,
        };
        func.compute_y_bounds_from_selection(selection);
        func
    }

    fn compute_y_bounds_from_selection(&mut self, selection: &RangeInclusive<f64>) {
        let sel_idxs = self.idxs_by_val_range(selection);
        let mut min_i = *sel_idxs.start();
        let mut max_i = min_i;
        let mut i: usize = min_i;
        self.y[sel_idxs].iter().for_each(|y| {
            if *y < self.y[min_i] {
                min_i = i;
            } else if *y > self.y[max_i] {
                max_i = i;
            }
            i += 1;
        });
        self.y_min_idx = min_i;
        self.y_max_idx = max_i;
    }

    fn idx_by_val_in_range(&mut self, val: f64) -> usize {
        match self.x.binary_search_by(|b| b.total_cmp(&val)) {
            Ok(idx) => idx,
            Err(idx) => {
                if idx <= 0 {
                    return 0;
                } else if idx >= self.x.len() {
                    return self.x.len() - 1;
                }
                let y = self.y[idx - 1]
                    + (self.y[idx] - self.y[idx - 1]) / (self.x[idx] - self.x[idx - 1])
                        * (val - self.x[idx - 1]);
                self.y.insert(idx, y);
                self.x.insert(idx, val);
                if self.y_min_idx >= idx {
                    self.y_min_idx += 1;
                }
                if self.y_max_idx >= idx {
                    self.y_max_idx += 1;
                }
                idx
            }
        }
    }

    fn idx_by_val(&mut self, val: f64) -> usize {
        self.idx_by_val_in_range(val)
    }

    fn idxs_by_val_range(&mut self, range: &RangeInclusive<f64>) -> RangeInclusive<usize> {
        let start_idx = self.idx_by_val(*range.start());
        start_idx..=self.idx_by_val_in_range(*range.end())
    }
}

struct Selection {
    values: RangeInclusive<f64>,
    strs: SelectionStrs,
}

struct Stretcher {
    stretch_factor: f64,
    add_factor: f64,
}

impl Stretcher {
    pub fn no_stretch() -> Self {
        Self {
            stretch_factor: 1.0,
            add_factor: 0.0,
        }
    }

    pub fn stretches(&self) -> bool {
        (self.stretch_factor - 1.0).abs() >= f64::EPSILON || self.add_factor.abs() >= f64::EPSILON
    }

    pub fn new_start(old: &RangeInclusive<f64>, new_start: f64) -> Self {
        if new_start > *old.end() {
            panic!("Cannot stretch start beyond end.");
        }
        let stretch_factor = (old.end() - new_start) / (old.end() - old.start());
        Self {
            stretch_factor,
            add_factor: old.end() * (1.0 - stretch_factor),
        }
    }

    pub fn new_end(old: &RangeInclusive<f64>, new_end: f64) -> Self {
        if new_end < *old.start() {
            panic!("Cannot stretch end beyond start.");
        }
        let stretch_factor = (new_end - old.start()) / (old.end() - old.start());
        Self {
            stretch_factor,
            add_factor: old.start() * (1.0 - stretch_factor),
        }
    }

    pub fn stretched(&self, x: f64) -> f64 {
        x * self.stretch_factor + self.add_factor
    }

    pub fn stretch(&self, x: &mut f64) {
        *x = *x * self.stretch_factor + self.add_factor;
    }

    pub fn combine(&self, other: Self) -> Self {
        Self {
            stretch_factor: self.stretch_factor * other.stretch_factor,
            add_factor: self.add_factor * other.stretch_factor + other.add_factor,
        }
    }
}

struct SelectionStrs {
    start: String,
    end: String,
}

struct YRange<T> {
    top: T,
    bottom: T,
}

struct YRangeWStrs {
    idxs: YRange<usize>,
    strs: YRange<String>,
}

struct MyApp {
    functions: HashMap<String, usize>,
    data: Vec<Func<f64, f64>>,
    active_funcs: HashSet<usize>,
    selection: Selection,
    x_range_strs: SelectionStrs,
    y_range: YRangeWStrs,
}

impl MyApp {}

impl Default for MyApp {
    fn default() -> Self {
        let data_x: Vec<f64> = (0..1000).into_iter().map(|x| x as f64 * 0.01).collect();
        let data_ys: Vec<Vec<f64>> = (0..3)
            .map(|fi| {
                data_x
                    .iter()
                    .map(|x| {
                        if fi % 2 == 0 {
                            x.sin() + fi as f64
                        } else {
                            x.cos() + fi as f64
                        }
                    })
                    .collect()
            })
            .collect();
        let selection = *data_x.first().unwrap()..=*data_x.last().unwrap();
        let selection = Selection {
            strs: SelectionStrs {
                start: selection.start().to_string(),
                end: selection.end().to_string(),
            },
            values: selection,
        };
        let active_funcs: HashSet<_> = (0..data_ys.len()).into_iter().collect();
        let mut data: Vec<_> = data_ys
            .into_iter()
            .map(|y| Func::new(data_x.clone(), y, &selection.values))
            .collect();
        let y_range_idxs =
            Self::compute_y_range_from(&mut data, active_funcs.iter(), &selection.values, true);
        Self {
            functions: HashMap::from_iter((0..data.len()).map(|i| (format!("func{}", i + 1), i))),
            y_range: YRangeWStrs {
                strs: YRange {
                    top: data[y_range_idxs.top].max_y().to_string(),
                    bottom: data[y_range_idxs.bottom].min_y().to_string(),
                },
                idxs: y_range_idxs,
            },
            data,
            x_range_strs: SelectionStrs {
                start: selection.strs.start().clone(),
                end: selection.strs.end().clone(),
            },
            selection,
            active_funcs,
        }
    }
}

fn txt_input<'a>(ui: &mut Ui, buffer: &'a mut String, label: &str) -> Option<&'a str> {
    let mut ret = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        let response = ui.text_edit_singleline(buffer);
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            ret = Some(buffer.as_str())
        }
    });
    ret
}

fn tinput<'a, T: FromStr + std::fmt::Display>(
    ui: &mut Ui,
    buffer: &'a mut String,
    label: &str,
    value: &T,
) -> Option<T> {
    if let Some(txt) = txt_input(ui, buffer, label) {
        if let Ok(value) = txt.parse() {
            return Some(value);
        } else {
            owerwrite_str(buffer, value).unwrap();
        }
    }
    None
}

impl MyApp {
    fn recompute_y_range(&mut self, selection_changed: bool) {
        self.y_range.idxs = Self::compute_y_range_from(
            self.data.as_mut_slice(),
            self.active_funcs.iter(),
            &self.selection.values,
            selection_changed,
        );
        self.update_y_range_strs();
    }

    fn update_y_range_strs(&mut self) {
        // y_range_idxs.0, y_range_idxs.1.to_string()
        let y_range = self.y_range();
        let strs = &mut self.y_range.strs;
        strs.bottom.clear();
        write!(&mut strs.bottom, "{}", y_range.start()).unwrap();
        strs.top.clear();
        write!(&mut strs.top, "{}", y_range.end()).unwrap();
    }

    fn change_selection(&mut self, new_selection: RangeInclusive<f64>) {
        self.selection.values = new_selection;
        let strs = &mut self.selection.strs;
        strs.start().clear();
        write!(&mut strs.start(), "{}", self.selection.values.start()).unwrap();
        strs.end().clear();
        write!(&mut strs.end(), "{}", self.selection.values.end()).unwrap();
        let xstrs = &mut self.x_range_strs;
        xstrs.start().clear();
        xstrs.start().push_str(&strs.start());
        xstrs.end().clear();
        xstrs.end().push_str(&strs.end());
    }

    fn compute_y_range_from<'a, TActiveFuncIter: Iterator<Item = &'a usize>>(
        data: &mut [Func<f64, f64>],
        active_funcs: TActiveFuncIter,
        selection: &RangeInclusive<f64>,
        recompute_each_func_bounds: bool,
    ) -> YRange<usize> {
        let mut min_y_idx = usize::MAX;
        let mut max_y_idx = min_y_idx;
        active_funcs.for_each(|fi| {
            let func = &mut data[*fi];
            if recompute_each_func_bounds {
                func.compute_y_bounds_from_selection(selection);
            }
            let range = func.y_range();
            if min_y_idx == usize::MAX {
                min_y_idx = *fi;
                max_y_idx = *fi;
            } else {
                if data[min_y_idx].min_y() > *range.start() {
                    min_y_idx = *fi;
                }
                if data[max_y_idx].max_y() < *range.end() {
                    max_y_idx = *fi;
                }
            }
        });
        YRange {
            bottom: min_y_idx,
            top: max_y_idx,
        }
    }

    fn y_range(&self) -> RangeInclusive<f64> {
        self.data[self.y_range.idxs.bottom].min_y()..=self.data[self.y_range.idxs.top].max_y()
    }

    fn stretch_selection(&mut self, new_x: RangeInclusive<f64>, new_y: RangeInclusive<f64>) {
        let selection = &self.selection.values;
        let y_range = self.y_range();
        let bottom_stretcher = Stretcher::new_start(&y_range, *new_y.start());
        let top_stretcher = Stretcher::new_end(&y_range, *new_y.end());
        let y_stretcher = bottom_stretcher.combine(top_stretcher);

        self.active_funcs.iter().for_each(|fi| {
            let func = &mut self.data[*fi];
            let selection_idxs = func.idxs_by_val_range(selection);

            let stretch_x_idxs = func.idxs_by_val_range(&new_x);

            let first = func.x[*selection_idxs.start()];
            let left = func.x[*stretch_x_idxs.start()];
            let left_stretch = left - first;
            let left_stretch_is_zero = left_stretch.abs() < f64::EPSILON;

            let last = func.x[*selection_idxs.end()];
            let right = func.x[*stretch_x_idxs.end()];
            let right_stretch = right - last;
            let right_stretch_is_zero = right_stretch.abs() < f64::EPSILON;

            if !left_stretch_is_zero || !right_stretch_is_zero {
                let left_stretcher = {
                    if left_stretch_is_zero {
                        Stretcher::no_stretch()
                    } else {
                        func.x[..*selection_idxs.start()].iter_mut().for_each(|x| {
                            *x += left_stretch;
                        });
                        Stretcher::new_start(&(first..=last), left)
                    }
                };
                let right_stretcher = {
                    if right_stretch_is_zero {
                        Stretcher::no_stretch()
                    } else {
                        func.x[*selection_idxs.end() + 1..]
                            .iter_mut()
                            .for_each(|x| {
                                *x += right_stretch;
                            });
                        Stretcher::new_end(&(first..=last), right)
                    }
                };
                let stretcher = left_stretcher.combine(right_stretcher);
                func.x[selection_idxs.clone()]
                    .iter_mut()
                    .for_each(|x| stretcher.stretch(x));
            }
            if y_stretcher.stretches() {
                func.y[selection_idxs]
                    .iter_mut()
                    .for_each(|y| y_stretcher.stretch(y));
            }
        });
    }
}

fn owerwrite_str<T: std::fmt::Display>(str: &mut String, value: T) -> Result<(), std::fmt::Error> {
    str.clear();
    write!(str, "{}", value)
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let y_range = self.y_range();
            let mut new_y_range = y_range.clone();
            let mut new_x_range = self.selection.values.clone();
            let mut new_selection = self.selection.values.clone();
            let mut activate_funcs = false;

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(format!(
                        "NaiveDateTime::from_str: {:?}",
                        NaiveDateTime::parse_from_str("08.07.2024  13:42:00", "%d.%m.%Y %H:%M:%S")
                    ));
                    ui.label(format!(
                        "DateTime::<Local>::from_str: {:?}",
                        DateTime::<Local>::from_str("08.07.2024  13:42:00")
                    ));
                    ui.label(format!(
                        "DateTime::<FixedOffset>::parse_from_rfc3339 {:?}",
                        DateTime::<FixedOffset>::parse_from_rfc3339("08.07.2024  13:42:00")
                    ));
                    ui.label(format!(
                        "DateTime::<FixedOffset>::parse_from_rfc2822 {:?}",
                        DateTime::<FixedOffset>::parse_from_rfc2822("08.07.2024  13:42:00")
                    ));
                    ui.label("CONTAINER");
                    ui.vertical(|ui| {
                        if let Some(top) =
                            tinput(ui, &mut self.y_range.strs.top, "top", new_y_range.end())
                        {
                            new_y_range = *new_y_range.start()..=top;
                        }
                        ui.horizontal(|ui| {
                            if let Some(left) = tinput(
                                ui,
                                &mut self.x_range_strs.start(),
                                "left",
                                new_x_range.start(),
                            ) {
                                new_x_range = left..=*new_x_range.end();
                            }
                            if let Some(right) =
                                tinput(ui, &mut self.x_range_strs.end(), "right", new_x_range.end())
                            {
                                new_x_range = *new_x_range.start()..=right
                            }
                        });
                        if let Some(start) = tinput(
                            ui,
                            &mut self.y_range.strs.bottom,
                            "bottom",
                            new_y_range.start(),
                        ) {
                            new_y_range = start..=*new_y_range.end();
                        }
                    });
                    ui.horizontal(|ui| {
                        if let Some(start) = tinput(
                            ui,
                            &mut self.selection.strs.start(),
                            "start",
                            new_selection.start(),
                        ) {
                            new_selection = start..=*new_selection.end();
                        }
                        if let Some(last) = tinput(
                            ui,
                            &mut self.selection.strs.end(),
                            "end",
                            new_selection.end(),
                        ) {
                            new_selection = *new_selection.start()..=last;
                        }
                    });
                    let mut rect = Rect::EVERYTHING;
                    ui.horizontal(|ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.functions.iter().for_each(|f| {
                                let mut checked = self.active_funcs.contains(f.1);
                                let prev_checked = checked;
                                if ui.add(Checkbox::new(&mut checked, f.0)).changed()
                                    && checked != prev_checked
                                {
                                    activate_funcs = true;
                                    if !self.active_funcs.insert(*f.1) {
                                        self.active_funcs.remove(f.1);
                                    }
                                }
                            });
                        });
                        let lines = self.active_funcs.iter().map(|fi| {
                            Line::new(
                                self.data[*fi]
                                    .x
                                    .iter()
                                    .zip(self.data[*fi].y.iter())
                                    .map(|p| [*p.0, *p.1])
                                    .collect::<Vec<_>>(),
                            )
                        });
                        rect = Plot::new("my_plot")
                            .view_aspect(2.0)
                            .show(ui, |plot_ui| {
                                let mut i = 0;
                                lines.for_each(|line| {
                                    plot_ui.line(line);
                                    i += 1;
                                });
                            })
                            .response
                            .rect;
                    });

                    ui.add(Separator::default().spacing(50.0)).highlight();
                    let shadow_color = Color32::from_rgba_premultiplied(100, 10, 5, 150);
                    let painter = ui.painter().with_clip_rect(rect);
                    painter.add(RectShape::filled(
                        rect.with_max_x(50.0),
                        Rounding::ZERO,
                        shadow_color,
                    ));
                    painter.add(RectShape::filled(
                        rect.with_min_x(rect.max.x - 50.0),
                        Rounding::ZERO,
                        shadow_color,
                    ));
                    let selection_changed = new_selection != self.selection.values;
                    let xrange_changed = new_x_range != self.selection.values;
                    if selection_changed {
                        self.change_selection(new_selection);
                    }
                    if activate_funcs || selection_changed {
                        self.recompute_y_range(selection_changed);
                    }

                    if xrange_changed || new_y_range != y_range {
                        self.stretch_selection(new_x_range.clone(), new_y_range);
                        self.update_y_range_strs();
                    }
                    if xrange_changed {
                        self.change_selection(new_x_range);
                    }
                });
            });
        });
    }
}
