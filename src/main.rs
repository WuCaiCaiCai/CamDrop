#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::CamDropApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1040.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CamDrop",
        options,
        Box::new(|cc| Ok(Box::new(CamDropApp::new(cc)))),
    )
}
