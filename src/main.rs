mod app;
mod db;
mod dialogs;
mod localization;

use app::App;

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../Assets/Icons/icon.png");
    let image = image::load_from_memory(icon_bytes).ok()?;
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("SQLite Workbench v1.0.0")
        .with_inner_size([1100.0, 700.0])
        .with_min_inner_size([800.0, 500.0]);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "SQLite Workbench",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
