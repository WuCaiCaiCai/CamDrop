#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::CamDropApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("assets/icon.png is a valid PNG");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "CamDrop",
        options,
        Box::new(|cc| Ok(Box::new(CamDropApp::new(cc)))),
    )
}
