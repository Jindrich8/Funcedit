// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)]

// it's an example
use eframe::egui::{self};
use plots::ui::MyApp;

fn main() -> eframe::Result {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    std::env::set_var("RUST_BACKTRACE", "full");
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
