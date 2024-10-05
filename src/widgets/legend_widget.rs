pub mod simple_legend_entries;

use eframe::{
    egui::{
        pos2, vec2, Align, Color32, Direction, Frame, Layout, PointerButton, Rect, Response, Sense,
        TextStyle, Ui, Widget, WidgetInfo, WidgetType,
    },
    epaint,
};
use egui_plot::{Corner, Legend};

pub trait LegendEntries {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() < 1
    }

    fn get_name(&self, index: usize) -> Option<String>;
    fn get_color(&self, index: usize) -> Option<Color32>;
    fn get_hovered(&self, index: usize) -> Option<bool>;
    fn get_checked(&self, index: usize) -> Option<bool>;
    fn get_entry(&self, index: usize) -> Option<LegendEntry>;

    fn iter_checked(&self) -> impl Iterator<Item = usize>;
    fn iter_unchecked(&self) -> impl Iterator<Item = usize>;

    fn check_all(&mut self);

    fn uncheck_all(&mut self);

    fn set_hovered(&mut self, index: usize, hovered: bool);
    fn set_checked(&mut self, index: usize, checked: bool);
    fn toggle_checked(&mut self, index: usize) {
        if let Some(checked) = self.get_checked(index) {
            self.set_checked(index, !checked);
        }
    }
    fn toggle_hovered(&mut self, index: usize) {
        if let Some(hovered) = self.get_hovered(index) {
            self.set_hovered(index, !hovered);
        }
    }
}

#[derive(Clone)]
pub struct LegendEntry {
    pub name: String,
    pub color: Color32,
    pub checked: bool,
    pub hovered: bool,
}

impl LegendEntry {
    pub fn new(name: String, color: Color32, checked: bool) -> Self {
        Self {
            name,
            color,
            checked,
            hovered: false,
        }
    }

    fn ui(&self, ui: &mut Ui, text_style: &TextStyle) -> Response {
        let Self {
            name,
            color,
            checked,
            hovered: _,
        } = self;
        let text = name;

        let font_id = text_style.resolve(ui.style());

        let galley = ui.fonts(|f| f.layout_delayed_color(text.clone(), font_id, f32::INFINITY));

        let icon_size = galley.size().y;
        let icon_spacing = icon_size / 5.0;
        let total_extra = vec2(icon_size + icon_spacing, 0.0);

        let desired_size = total_extra + galley.size();
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                ui.is_enabled(),
                *checked,
                galley.text(),
            )
        });

        let visuals = ui.style().interact(&response);
        let label_on_the_left = ui.layout().horizontal_placement() == Align::RIGHT;

        let icon_position_x = if label_on_the_left {
            rect.right() - icon_size / 2.0
        } else {
            rect.left() + icon_size / 2.0
        };
        let icon_position = pos2(icon_position_x, rect.center().y);
        let icon_rect = Rect::from_center_size(icon_position, vec2(icon_size, icon_size));

        let painter = ui.painter();

        painter.add(epaint::CircleShape {
            center: icon_rect.center(),
            radius: icon_size * 0.5,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if *checked {
            let fill = if *color == Color32::TRANSPARENT {
                ui.visuals().noninteractive().fg_stroke.color
            } else {
                *color
            };
            painter.add(epaint::Shape::circle_filled(
                icon_rect.center(),
                icon_size * 0.4,
                fill,
            ));
        }

        let text_position_x = if label_on_the_left {
            rect.right() - icon_size - icon_spacing - galley.size().x
        } else {
            rect.left() + icon_size + icon_spacing
        };

        let text_position = pos2(text_position_x, rect.center().y - 0.5 * galley.size().y);
        painter.galley(text_position, galley, visuals.text_color());

        response
    }
}

pub struct LegendWidget<'a, Entries: LegendEntries> {
    rect: Rect,
    entries: &'a mut Entries,
    config: Legend,
}

impl<'a, Entries: LegendEntries> LegendWidget<'a, Entries> {
    /// Create a new legend from items, the names of items that are hidden and the style of the
    /// text. Returns `None` if the legend has no entries.
    pub fn try_new(rect: Rect, config: Legend, entries: &'a mut Entries) -> Option<Self> {
        (!entries.is_empty()).then_some(Self {
            rect,
            entries,
            config,
        })
    }
}

impl<'a, Entries: LegendEntries> Widget for &mut LegendWidget<'a, Entries> {
    fn ui(self, ui: &mut Ui) -> Response {
        let LegendWidget {
            rect,
            entries,
            config,
        } = self;

        let main_dir = match config.position {
            Corner::LeftTop | Corner::RightTop => Direction::TopDown,
            Corner::LeftBottom | Corner::RightBottom => Direction::BottomUp,
        };
        let cross_align = match config.position {
            Corner::LeftTop | Corner::LeftBottom => Align::LEFT,
            Corner::RightTop | Corner::RightBottom => Align::RIGHT,
        };
        let layout = Layout::from_main_dir_and_cross_align(main_dir, cross_align);
        let legend_pad = 4.0;
        let legend_rect = rect.shrink(legend_pad);
        let mut legend_ui = ui.child_ui(legend_rect, layout, None);
        legend_ui
            .scope(|ui| {
                let background_frame = Frame {
                    inner_margin: vec2(8.0, 4.0).into(),
                    rounding: ui.style().visuals.window_rounding,
                    shadow: epaint::Shadow::NONE,
                    fill: ui.style().visuals.extreme_bg_color,
                    stroke: ui.style().visuals.window_stroke(),
                    ..Default::default()
                }
                .multiply_with_opacity(config.background_alpha);
                background_frame
                    .show(ui, |ui| {
                        let mut focus_on_item = None;

                        let response_union = (0..entries.len())
                            .into_iter()
                            .map(|(entry_index)| {
                                let entry = entries.get_entry(entry_index).unwrap();
                                let response = entry.ui(ui, &config.text_style);

                                // Handle interactions. Alt-clicking must be deferred to end of loop
                                // since it may affect all entries.
                                handle_interaction_on_legend_item(&response, entry_index, *entries);
                                if response.clicked() && ui.input(|r| r.modifiers.alt) {
                                    focus_on_item = Some(entry_index);
                                }

                                response
                            })
                            .reduce(|r1, r2| r1.union(r2))
                            .unwrap();

                        if let Some(focus_on_item) = focus_on_item {
                            handle_focus_on_legend_item(focus_on_item, *entries);
                        }

                        response_union
                    })
                    .inner
            })
            .inner
    }
}

/// Handle per-entry interactions.
fn handle_interaction_on_legend_item(
    response: &Response,
    index: usize,
    entries: &mut impl LegendEntries,
) {
    if response.clicked_by(PointerButton::Primary) {
        entries.toggle_checked(index);
    }
    entries.set_hovered(index, response.hovered());
}

/// Handle alt-click interaction (which may affect all entries).
fn handle_focus_on_legend_item(clicked_entry_index: usize, entries: &mut impl LegendEntries) {
    // if all other items are already hidden, we show everything
    if clicked_entry_index < entries.len() {
        let is_focus_item_only_visible = {
            let mut checked_entries = entries.iter_checked();
            if let Some(entry_index) = checked_entries.next() {
                entry_index == clicked_entry_index && checked_entries.next().is_none()
            } else {
                true
            }
        };

        // either show everything or show only the focus item
        if is_focus_item_only_visible {
            entries.check_all();
        } else {
            entries.uncheck_all();
        }
        entries.set_checked(clicked_entry_index, true);
    }
}
