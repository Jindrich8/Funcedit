use eframe::egui::{Rangef, Ui};
use egui_plot::{CoordinatesFormatter, Corner, Plot as EPlot, PlotBounds, PlotResponse, PlotUi};

use crate::{date_time_ctx::DateTimeCtx, types::point::X};

pub struct Plotter {
    pub bounds: PlotBounds,
    pub data_aspect: f32,
    x_base_size: f64,
    grid_spacing: Rangef,
    pub time_ctx: Option<DateTimeCtx>,
}

impl Plotter {
    pub fn new(bounds: PlotBounds, data_aspect: f32, time_ctx: Option<DateTimeCtx>) -> Self {
        Self {
            bounds,
            data_aspect,
            x_base_size: 1.0,
            grid_spacing: Rangef::new(8.0, 300.0),
            time_ctx,
        }
    }

    pub fn show<R>(
        &mut self,
        ui: &mut Ui,
        id_source: impl std::hash::Hash,
        width: f32,
        height: f32,
        build_fn: impl FnOnce(&mut PlotUi) -> R,
    ) -> PlotResponse<R> {
        let mut plot = EPlot::new(id_source);
        if let Some(ctx) = &self.time_ctx {
            plot = plot.x_grid_spacer(ctx.grid_spacer());
            plot = plot.custom_x_axes(ctx.x_axes(self.x_base_size));
            plot = plot.coordinates_formatter(
                Corner::LeftTop,
                CoordinatesFormatter::new(|point, bounds| {
                    ctx.info.mapper.f64_to_time(point.x).to_string()
                }),
            );
            plot = plot.label_formatter(|name, value| {
                let info = &ctx.info;
                format!(
                    "{}\n{}\n{}",
                    name,
                    info.mapper
                        .f64_to_time(value.x)
                        .format(&info.format)
                        .to_string(),
                    value.y
                )
            });
        };

        let response = plot
            .width(width)
            .height(height)
            .data_aspect(self.data_aspect)
            .grid_spacing(self.grid_spacing)
            .show(ui, |plot_ui| {
                let r = build_fn(plot_ui);
                if self.bounds.width() < X::EPSILON {
                    self.bounds = plot_ui.plot_bounds();
                }

                if self.bounds.width() >= X::EPSILON {
                    plot_ui.set_plot_bounds(self.bounds);
                }
                r
            });
        self.x_base_size = response.transform.dvalue_dpos()[0].abs() * self.grid_spacing.min as f64;
        let new_bounds = response.transform.bounds();

        if *new_bounds != self.bounds {
            self.bounds = *response.transform.bounds();
        }
        response
    }
}
