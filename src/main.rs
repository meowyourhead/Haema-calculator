#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod chrome;
mod fonts;
mod logic;
mod panels;
mod reference;
mod theme;
mod ui;

use eframe::egui;

fn fox_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!("../assets/icon/fox-128.rgba").to_vec(),
        width: 128,
        height: 128,
    }
}

fn main() -> eframe::Result<()> {
    let mut size = [1180.0, 820.0];
    #[cfg(feature = "screenshot")]
    if let Ok(s) = std::env::var("HAEM_SIZE") {
        let v: Vec<f32> = s.split('x').filter_map(|p| p.parse().ok()).collect();
        if v.len() == 2 {
            size = [v[0], v[1]];
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Haematology Calculator")
            .with_decorations(false)
            .with_icon(fox_icon())
            .with_inner_size(size)
            .with_min_inner_size([720.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Haematology Calculator",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
