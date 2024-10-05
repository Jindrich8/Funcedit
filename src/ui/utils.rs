use eframe::{egui::Color32, epaint::Hsva};

pub fn auto_color(index: usize) -> Color32 {
    let i = index;
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    let h = i as f32 * golden_ratio;
    Hsva::new(h, 0.85, 0.5, 1.0).into() // TODO(emilk): OkLab or some other perspective color space
}
