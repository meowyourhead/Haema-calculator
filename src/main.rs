#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod chrome;
mod cursor;
mod fonts;
mod hbel_custom;
mod hbel_data;
mod hbel_match;
mod hbel_panel;
mod logic;
mod paimon;
mod panels;
mod pikachu;
mod reference;
mod theme;
mod ui;
mod update;

use eframe::egui;

fn fox_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!("../assets/icon/fox-128.rgba").to_vec(),
        width: 128,
        height: 128,
    }
}

fn main() -> eframe::Result<()> {
    // Only the first frame's size: after that the window fits the page.
    #[cfg_attr(not(feature = "screenshot"), allow(unused_mut))]
    let mut size = [1014.0, 832.0];
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
            .with_min_inner_size([400.0, 240.0])
            .with_maximize_button(true),
        ..Default::default()
    };
    eframe::run_native(
        "Haematology Calculator",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
