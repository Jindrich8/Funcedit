pub mod history;
pub mod plotter;
pub mod utils;

use core::f64;
use std::{
    fmt::Write,
    mem::swap,
    ops::{Add, RangeInclusive},
    str::FromStr,
};

use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use history::{
    plot_bounds_change::change::PlotBoundsChange, ApplyDataOp, OwnedHistoryOp, SharedDataOp,
};
use plotter::Plotter;
// it's an example
use crate::{
    date_time_ctx::DateTimeCtx,
    functions::function::{StretchY, StretchYBounds},
    graph::{Graph, GraphFuncState},
    history::{
        history_stack::{
            shared_entry::{ApplyOp, ApplyOtherOp, OtherOp, OwnedOp, SharedOutOp},
            HistoryOption, IsGraphOpNonAltering,
        },
        History,
    },
    importer::{DateTimeInfo, Importer},
    layout::calculator::{self},
    reversible_graph::basic_reversible_graph::BasicReversibleGraph,
    shared_op::SharedOp,
    stretchers::y_stretcher::YStretcherFlags,
    types::{
        self,
        datetime_f64_mapper::{DateTimeF64Mapper, DateTimePrecision},
        point::{X, Y},
    },
    ui,
    utils::Changeable,
    widgets::{
        legend_widget::{
            simple_legend_entries::{LegendActionId, SimpleLegendEntries, SimpleLegendEntry},
            LegendEntries, LegendEntry, LegendWidget,
        },
        slider_widget::{SliderOptionalData, SliderWidget},
    },
};
use eframe::egui::{self, Color32, Id, Key, Rangef, Response, ScrollArea, Slider, Ui, Widget};
use egui_plot::{CoordinatesFormatter, Corner, HLine, Legend, Plot, PlotBounds, PlotUi, VLine};
use enumflags2::BitFlags;

#[derive(Debug, Default, Clone, PartialEq)]
enum ActionId {
    #[default]
    Conditions,
    InsertValues,
    InsertPattern,
    DeleteSelection,
    StretchY,
}

impl LegendActionId for ActionId {
    fn change_active_funcs() -> Self {
        ActionId::Conditions
    }
}

impl Into<usize> for ActionId {
    fn into(self) -> usize {
        self as usize
    }
}

struct SelectionVLines {
    start: Id,
    end: Id,
}

struct NonAlteringGraphOpHelper;

impl IsGraphOpNonAltering<ActionId> for NonAlteringGraphOpHelper {
    fn graph_op_alters_history<IterChangeActiveFuncs, FuncIter, YExactIter>(
        g_op: &crate::shared_op::SharedOp<IterChangeActiveFuncs, FuncIter, YExactIter>,
        group_id: &ActionId,
    ) -> bool
    where
        IterChangeActiveFuncs: Iterator<Item = usize> + Clone,
        FuncIter: Iterator<Item = YExactIter>,
        YExactIter: ExactSizeIterator<Item = Y> + Clone,
    {
        *group_id != ActionId::Conditions
            || !matches!(
                g_op,
                SharedOp::ChangeActiveFuncs(_) | SharedOp::MoveSelectBy(_)
            )
    }
}

pub struct MyApp {
    graph: BasicReversibleGraph<ActionId, OwnedHistoryOp, NonAlteringGraphOpHelper>,
    history: ui::history::History,
    selection_range: RangeInclusive<X>,
    selection: SelectionVLines,
    legend_entries: Vec<SimpleLegendEntry>,
    plot: Plotter,
}

impl Default for MyApp {
    fn default() -> Self {
        let path = r#"C:\Users\Jindra\Downloads\UVN_TO_obj23_zap0_reg3.txt"#;
        let mut funcs = Vec::new();
        let res = Importer::import(&path, &mut funcs).unwrap();
        let graph = Graph::new(funcs.into_iter().map(|b| b.into()).collect());
        let mut history = History::new();
        history.with_options(HistoryOption::TreatNonAlteringEntriesAsRegular);
        Self {
            selection_range: graph.selection().clone(),
            graph: BasicReversibleGraph::new(graph, history),
            selection: SelectionVLines {
                start: Id::new(0),
                end: Id::new(1),
            },
            plot: Plotter::new(
                PlotBounds::from_min_max([0.0, 0.0], [0.0, 0.0]),
                12.0,
                match res.mapper {
                    Some(mapper) => Some(DateTimeCtx::new(mapper)),
                    None => None,
                },
            ),
            legend_entries: res
                .names
                .into_iter()
                .enumerate()
                .map(|(i, name)| SimpleLegendEntry::new(name, utils::auto_color(i), false))
                .collect(),
            history: ui::history::History::new(),
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

fn owerwrite_str<T: std::fmt::Display>(str: &mut String, value: T) -> Result<(), std::fmt::Error> {
    str.clear();
    write!(str, "{}", value)
}

impl MyApp {
    fn top(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("my_top_panel").show(ctx, |ui| {
            ui.label(format!("Plot bounds: {:#?}", self.plot.bounds));
            if ui
                .button(format!("Undo {}", self.graph.history().undo_len()))
                .clicked()
            {
                self.graph.undo(|op| {
                    if let Some(op) = self.history.undo(op) {
                        match op {
                            history::SharedDataOp::ChangePlotBounds(change) => {
                                self.plot.bounds.apply_change(change);
                            }
                        }
                    }
                });
            }
            if ui
                .button(format!("Redo {}", self.graph.history().redo_len()))
                .clicked()
            {
                self.graph.redo(|op| {
                    if let Some(op) = self.history.redo(op) {
                        match op {
                            SharedDataOp::ChangePlotBounds(change) => {
                                self.plot.bounds.apply_change(change);
                            }
                        }
                    }
                });
            }
            ui.label(format!("{:#?}", self.graph.graph().selection()));
            ui.horizontal(|ui| {
                ui.label(format!(
                    "min: {}",
                    self.graph.graph().min().unwrap_or(f64::NAN)
                ));
                ui.label(format!(
                    "max: {}",
                    self.graph.graph().max().unwrap_or(f64::NAN)
                ));
            });
        });
    }

    fn selection(plot_ui: &mut PlotUi, selection: &RangeInclusive<f64>) {
        plot_ui.vline(
            VLine::new(*selection.start())
                .name("Start")
                .color(Color32::LIGHT_RED)
                .id(Id::new("Selection_start_vline_marker")),
        );
        plot_ui.vline(
            VLine::new(*selection.end())
                .name("End")
                .color(Color32::LIGHT_RED)
                .id(Id::new("Selection_end_vline_marker")),
        );
    }

    fn selection_control(selection: &mut RangeInclusive<f64>, bounds: &RangeInclusive<f64>) {
        if !bounds.contains(&selection.start()) && !bounds.contains(&selection.end()) {
            *selection = bounds.clone();
        } else {
            if selection.start() > selection.end() {
                swap(&mut selection.start(), &mut selection.end());
            }
            *selection =
                selection.start().max(*bounds.start())..=selection.end().min(*bounds.end());
            if selection.start() > selection.end() {
                swap(&mut selection.start(), &mut selection.end());
            }
        }
        /*  let diff = selection.start() - bounds.start() + bounds.end() - selection.end();
         *selection = bounds.clone() + (diff / 2.0); */
    }

    fn selection_slider(
        ui: &mut Ui,
        selection: &mut RangeInclusive<f64>,
        bounds: &RangeInclusive<f64>,
    ) {
        let mut start = *selection.start();
        let mut end = *selection.end();
        ui.vertical_centered_justified(|ui| {
            let label_width1_id = Id::new("slider_label_width1");
            let label_width2_id = Id::new("slider_label_width2");
            let aw = ui.available_width();
            let mut lwidth1 = ui.data(|r| r.get_temp(label_width1_id).unwrap_or(0.0));
            let mut lwidth2 = ui.data(|r| r.get_temp(label_width2_id).unwrap_or(0.0));
            let slider_width_before = ui.spacing_mut().slider_width;
            ui.spacing_mut().slider_width = (aw - lwidth1).max(0.0);
            let w1 = ui
                .add(
                    Slider::new(&mut start, bounds.clone().into())
                        .clamp_to_range(false)
                        .max_decimals(0),
                )
                .rect
                .width();
            ui.spacing_mut().slider_width = (aw - lwidth2).max(0.0);
            let w2 = ui
                .add(
                    Slider::new(&mut end, bounds.clone().into())
                        .clamp_to_range(false)
                        .max_decimals(0),
                )
                .rect
                .width();
            if w1 > aw {
                lwidth1 = (w1 - aw) + 10.0;
            }
            if w2 > aw {
                lwidth2 = (w2 - aw) + 10.0;
            }
            ui.data_mut(|w| {
                w.insert_temp(label_width1_id, lwidth1);
                w.insert_temp(label_width2_id, lwidth2);
            });
            ui.spacing_mut().slider_width = slider_width_before;
        });
        *selection = if start <= end {
            start..=end
        } else {
            end..=start
        };
    }

    fn aspect_ratio_slider(ui: &mut Ui, ratio: &mut f32) -> Response {
        ui.add(Slider::new(ratio, 1.0..=100.0).max_decimals(0))
    }

    fn stretch_y_controls(
        ui: &mut Ui,
        bounds: &RangeInclusive<f64>,
        graph: &mut BasicReversibleGraph<ActionId, OwnedHistoryOp, NonAlteringGraphOpHelper>,
    ) {
        ui.vertical(|ui| {
            let mut stretch_flags = BitFlags::empty();
            ui.horizontal(|ui| {
                let stretch_y_mode_id = Id::new("stretch_y_mode");
                let old_both = ui.data(|r| r.get_temp(stretch_y_mode_id).unwrap_or(false));
                let mut both = old_both;

                ui.checkbox(&mut both, "Stretch both").rect.height();
                if old_both != both {
                    ui.data_mut(|w| w.insert_temp(stretch_y_mode_id, both));
                }
                if both {
                    stretch_flags = BitFlags::all();
                }
                ui.group(|ui| {
                    if stretch_flags.is_all() {
                        ui.disable();
                    }
                    let stretch_factor_mode_id = Id::new("stretch_factor_mode");
                    let mut is_top =
                        ui.data(|r| r.get_temp(stretch_factor_mode_id).unwrap_or(true));
                    let mut new_is_top = is_top;
                    ui.toggle_value(&mut new_is_top, if is_top { "Top" } else { "Bottom" });

                    stretch_flags.insert(if new_is_top {
                        YStretcherFlags::Top
                    } else {
                        YStretcherFlags::Bottom
                    });
                    if new_is_top != is_top {
                        ui.data_mut(|w| w.insert_temp(stretch_factor_mode_id, new_is_top));
                    }
                    is_top = new_is_top;
                });
            });
            let mut stretch_bounds = if stretch_flags.is_all() {
                StretchYBounds::new(Y::NEG_INFINITY, Y::INFINITY)
            } else {
                StretchYBounds::new(Y::NAN, Y::NAN)
            };

            let value_range = match graph.graph().value_range() {
                Some(r) => r,
                None => return,
            };
            let overflow_height_id = Id::new("overwlow/underflow_of_height_of_stretch_y_controls");
            let height =
                ui.available_height() - ui.data(|r| r.get_temp(overflow_height_id).unwrap_or(0.0));
            ui.spacing_mut().slider_width = height;
            let mut min_height = 0.0_f32;
            let range =
                value_range.start().min(*bounds.start())..=value_range.end().max(*bounds.end());

            ui.horizontal_top(|ui| {
                let max = *value_range.end();
                let mut new_max = max;

                let resp = ui.add(SliderWidget {
                    num: &mut new_max,
                    range: range.clone(),
                    data: SliderOptionalData {
                        orientation: egui::SliderOrientation::Vertical,
                        ..Default::default()
                    },
                });
                min_height = min_height.max(resp.rect.height());

                fn is_action_end(response: &Response) -> bool {
                    response.drag_stopped() || response.lost_focus()
                }

                let mut action_end = false;
                if (new_max - max).abs() >= Y::EPSILON {
                    action_end = is_action_end(&resp);
                    stretch_bounds.set_max(new_max);
                }

                let min = *value_range.start();
                let mut new_min = min;
                let resp = ui.add(SliderWidget {
                    num: &mut new_min,
                    range: range.clone(),
                    data: SliderOptionalData {
                        orientation: egui::SliderOrientation::Vertical,
                        ..Default::default()
                    },
                });
                min_height = min_height.max(resp.rect.height());
                if (new_min - min).abs() >= Y::EPSILON {
                    if action_end {
                        action_end = is_action_end(&resp);
                    }
                    stretch_bounds.set_min(new_min);
                }
                let stretch_factor_zp_id = Id::new("stretch_factor_zp");

                let mut factor: f64 = ui.data(|r| r.get_temp(stretch_factor_zp_id).unwrap_or(1.0));
                let mut new_factor = factor;
                let resp = ui.add(SliderWidget {
                    num: &mut new_factor,
                    range: -1.5..=1.5,
                    data: SliderOptionalData {
                        step: 0.01,
                        clamp_to_range: false,
                        orientation: egui::SliderOrientation::Vertical,
                        ..Default::default()
                    },
                });
                min_height = min_height.max(resp.rect.height());

                let mut is_factor_action_end = false;
                if (new_factor - factor).abs() >= Y::EPSILON {
                    let factor_action_end = is_action_end(&resp);
                    if action_end {
                        action_end = factor_action_end;
                    }
                    is_factor_action_end = factor_action_end;
                }
                if !is_factor_action_end && (resp.dragged() || resp.drag_started()) {
                    if new_factor.abs() < f64::EPSILON {
                        new_factor = new_factor.signum() * 0.01;
                    }
                    if (new_factor - factor).abs() >= f64::EPSILON {
                        ui.data_mut(|w| w.insert_temp(stretch_factor_zp_id, new_factor));
                    }
                } else {
                    ui.data_mut(|w| w.remove::<f64>(stretch_factor_zp_id));
                }
                factor = new_factor / factor;
                if factor.abs() > 10.0 {
                    println!("factor {} is too large!", factor);
                }
                if (factor - 1.0).abs() < 0.0001 {
                    factor = 1.0;
                }
                {
                    let mut b = graph.open_action(ActionId::StretchY);
                    let _ = b.stretch_y_bounds(&stretch_bounds);
                    if let Some(stretch) = StretchY::new(factor, stretch_flags) {
                        let _ = b.stretch_y_with_factor(&stretch);
                    }
                }

                if action_end {
                    graph.close_action(ActionId::StretchY);
                }
            });
            ui.data_mut(|w| {
                w.insert_temp(
                    overflow_height_id,
                    min_height.max(ui.min_rect().height()) - height,
                )
            });
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.top(ctx, _frame);
        let y_bounds = self.plot.bounds.range_y();
        egui::SidePanel::right("history_side_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label("REDO");
                ui.separator();

                ScrollArea::vertical()
                    .id_source("redo_scroll_area")
                    .show(ui, |ui| {
                        let mut history = self.history.iter();
                        self.graph
                            .history()
                            .redo_iter()
                            .enumerate()
                            .for_each(|(i, e)| {
                                let text = format!("{:?} ({})", e.id(), e.len());
                                ui.vertical(|ui| {
                                    ui.label(text);
                                    ui.separator();
                                    for op in e.iter() {
                                        let op_str = match op {
                                            ApplyOp::Graph(op) => {
                                                format!("{:#?}", op)
                                            }
                                            ApplyOp::Other(op) => {
                                                let v = history.next_redo(op).unwrap();
                                                format!("{:#?}", v)
                                            }
                                        };
                                        ui.label(op_str);
                                    }
                                    ui.separator();
                                });
                            });
                    });
                ui.separator();
                ui.label("UNDO");
                ui.separator();
                ScrollArea::vertical()
                    .id_source("undo_scroll_area")
                    .show(ui, |ui| {
                        let mut history = self.history.iter();
                        self.graph
                            .history()
                            .undo_iter()
                            .enumerate()
                            .for_each(|(i, e)| {
                                let text = format!("{:?} ({})", e.id(), e.len());
                                ui.vertical(|ui| {
                                    ui.label(text);
                                    ui.separator();
                                    for op in e.iter() {
                                        let op_str = match op {
                                            ApplyOp::Graph(op) => {
                                                format!("{:#?}", op)
                                            }
                                            ApplyOp::Other(op) => {
                                                let v = history.next_undo(op).unwrap();
                                                format!("{:#?}", v)
                                            }
                                        };
                                        ui.label(op_str);
                                    }
                                    ui.separator();
                                });
                            });
                    });
            });
        });
        egui::SidePanel::right("y_stretch_controls_side_panel").show(ctx, |ui| {
            Self::stretch_y_controls(ui, &y_bounds, &mut self.graph)
        });

        let selection = self.graph.graph().selection();
        if *selection != self.selection_range {
            let bounds_width = self.plot.bounds.width();
            let center = (selection.start() + selection.end()) / 2.0;
            self.plot.bounds.set_x_center_width(center, bounds_width);
            self.plot.bounds.extend_with_x(*selection.start());
            self.plot.bounds.extend_with_x(*selection.end());
            self.selection_range = selection.clone();
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                let x_bounds = self.plot.bounds.range_x();

                let mut selection = self.graph.graph().selection().clone();
                Self::selection_slider(ui, &mut selection, &x_bounds);

                let hoverflow_id = Id::new("height_oweflow_plot");
                let available_height = ui.available_height();
                let howeflow = ui.data(|r| r.get_temp(hoverflow_id).unwrap_or(0.0));
                let height = available_height - howeflow;
                ui.horizontal(|ui| {
                    calculator::width(ui, |ui, width| {
                        let lines = self.graph.graph().active_functions_index().map(|(f, fi)| {
                            f.line()
                                .name(&self.legend_entries[fi].name)
                                .color(self.legend_entries[fi].color)
                                .highlight(self.legend_entries[fi].hovered)
                        });
                        let old_bounds = self.plot.bounds;
                        let response = self.plot.show(ui, "my_plot", width, height, |plot_ui| {
                            let mut i = 0;
                            Self::selection(plot_ui, &selection);
                            if let Some(range) = self.graph.graph().value_range() {
                                plot_ui.hline(
                                    HLine::new(*range.start()).name("Min").allow_hover(true),
                                );
                                plot_ui
                                    .hline(HLine::new(*range.end()).name("Max").allow_hover(true));
                            }
                            lines.into_iter().for_each(|line| {
                                plot_ui.line(line);
                                i += 1;
                            });
                        });

                        let new_bounds = response.transform.bounds();

                        if *new_bounds != old_bounds {
                            let x_bounds = new_bounds.range_x();
                            Self::selection_control(&mut selection, &x_bounds);
                            self.graph.open_action(ActionId::Conditions).other(
                                self.history.bounds_change(&PlotBoundsChange::from_old_new(
                                    &old_bounds,
                                    new_bounds,
                                )),
                            );
                        }

                        ui.input(|r| {
                            if r.key_pressed(Key::Delete) {
                                dbg!("plot has focus and delete");
                                let mut builder = self.graph.action(ActionId::DeleteSelection);
                                builder.delete();
                                builder.change_selection(x_bounds.clone());
                            }
                        });
                        if let Some(mut legend) = LegendWidget::try_new(
                            response.response.rect,
                            Legend::default(),
                            &mut SimpleLegendEntries::new(
                                &mut self.graph,
                                &mut self.legend_entries,
                            ),
                        ) {
                            ui.put(response.response.rect, &mut legend);
                        }
                    });
                });
                Self::selection_control(&mut selection, &x_bounds);

                self.graph
                    .open_action(ActionId::Conditions)
                    .change_selection(selection.clone());
                self.selection_range = selection;
                Self::aspect_ratio_slider(ui, &mut self.plot.data_aspect);
                let ah = ui.min_rect().height();
                ui.data_mut(|w| w.insert_temp(hoverflow_id, ah - height));
            });
        });
    }
}
