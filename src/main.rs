#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod expiration;
use app::CoordinatesSuite;
use egui::IconData;
use egui::ViewportBuilder;

fn load_icon() -> IconData {
    let icon_bytes = include_bytes!("../assets/icon-256.png");
    let img = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon from embedded bytes")
        .into_rgba8();

    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    IconData {
        rgba,
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    expiration::panic_if_expired();

    let icon_data = load_icon();
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            title: Some("Coordinates Suite".to_string()),
            inner_size: Some([1150.0, 720.0].into()),
            min_inner_size: Some([1150.0, 720.0].into()),
            icon: Some(std::sync::Arc::new(icon_data)),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "Coordinates Suite",
        native_options,
        Box::new(|cc| Ok(Box::new(CoordinatesSuite::new(cc)))),
    )
}
