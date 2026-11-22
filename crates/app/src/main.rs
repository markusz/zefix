//! Desktop UI for schafsim: build a scenario from card tiles and watch the probabilities converge.

mod app;
mod format;
mod model;
mod paint;
mod runner;
mod theme;
mod view;

use eframe::egui;

const WINDOW_SIZE: [f32; 2] = [1440.0, 960.0];
const MIN_WINDOW_SIZE: [f32; 2] = [900.0, 600.0];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Schafsim")
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        "Schafsim",
        options,
        Box::new(|cc| Ok(Box::new(app::SchafsimApp::new(&cc.egui_ctx)))),
    )
}
