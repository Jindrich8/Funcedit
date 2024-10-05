use std::ops::RangeInclusive;

use eframe::egui::{style::HandleShape, Slider, SliderOrientation, Widget, WidgetText};

pub type NumFormatter<'a> = Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>;
pub type NumParser<'a> = Box<dyn 'a + Fn(&str) -> Option<f64>>;

pub struct SliderOptionalData<'a> {
    pub logarithmic: bool,
    pub clamp_to_range: bool,
    pub smart_aim: bool,
    pub orientation: SliderOrientation,
    pub show_value: bool,
    pub smallest_positive: f64,
    pub largest_finite: f64,
    pub prefix: String,
    pub suffix: String,
    pub text: WidgetText,

    /// Sets the minimal step of the widget value
    pub step: f64,

    pub drag_value_speed: Option<f64>,
    pub min_decimals: usize,
    pub max_decimals: Option<usize>,
    pub custom_formatter: Option<NumFormatter<'a>>,
    pub custom_parser: Option<NumParser<'a>>,
    pub trailing_fill: Option<bool>,
    pub handle_shape: Option<HandleShape>,
}

impl<'a> Default for SliderOptionalData<'a> {
    fn default() -> Self {
        Self {
            logarithmic: false,
            clamp_to_range: true,
            smart_aim: true,
            orientation: SliderOrientation::Horizontal,
            show_value: true,
            smallest_positive: 1e-6,
            largest_finite: f64::INFINITY,
            prefix: Default::default(),
            suffix: Default::default(),
            text: Default::default(),
            step: 0.0,
            drag_value_speed: None,
            min_decimals: 0,
            max_decimals: None,
            custom_formatter: None,
            custom_parser: None,
            trailing_fill: None,
            handle_shape: None,
        }
    }
}

pub struct SliderWidget<'a, Num: eframe::emath::Numeric + Send + Sync> {
    pub num: &'a mut Num,
    pub range: RangeInclusive<Num>,
    pub data: SliderOptionalData<'a>,
}
impl<'a, Num: eframe::emath::Numeric + Send + Sync> Widget for SliderWidget<'a, Num> {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let id = ui.next_auto_id();
        let mut curr = ui.data(|r| r.get_temp(id).unwrap_or(*self.num));

        let mut slider = Slider::new(&mut curr, self.range)
            .clamp_to_range(self.data.clamp_to_range)
            .smart_aim(self.data.smart_aim)
            .show_value(self.data.show_value)
            .logarithmic(self.data.logarithmic)
            .orientation(self.data.orientation)
            .smallest_positive(self.data.smallest_positive)
            .largest_finite(self.data.largest_finite)
            .prefix(self.data.prefix)
            .suffix(self.data.suffix)
            .text(self.data.text)
            .step_by(self.data.step)
            .min_decimals(self.data.min_decimals);
        if let Some(v) = self.data.drag_value_speed {
            slider = slider.drag_value_speed(v);
        }
        if let Some(v) = self.data.max_decimals {
            slider = slider.max_decimals(v);
        }
        if let Some(v) = self.data.custom_formatter {
            slider = slider.custom_formatter(v);
        }
        if let Some(v) = self.data.custom_parser {
            slider = slider.custom_parser(v);
        }
        if let Some(v) = self.data.trailing_fill {
            slider = slider.trailing_fill(v);
        }
        if let Some(v) = self.data.handle_shape {
            slider = slider.handle_shape(v);
        }
        let resp = slider.ui(ui);
        if resp.lost_focus() || resp.dragged() || resp.drag_started() || resp.drag_stopped() {
            *self.num = curr;
            ui.data_mut(|w| w.remove::<Num>(id));
        } else {
            ui.data_mut(|w| w.insert_temp(id, curr));
        }
        resp
    }
}
