mod app;
mod db;
mod dialogs;
mod localization;

use app::App;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SQLite Workbench v1.0.0")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SQLite Workbench",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
