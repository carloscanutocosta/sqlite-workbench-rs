use egui::{Color32, RichText, ScrollArea, Ui};

use super::App;

impl App {
    pub(super) fn show_sidebar(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        ui.add_space(8.0);
        ui.heading(self.t().db_explorer);
        ui.separator();

        if ui.button(self.t().load_db).clicked() {
            self.open_db_file();
        }

        let db_loaded = self.db.is_some();

        ui.add_enabled_ui(db_loaded, |ui| {
            if ui.button(self.t().import_csv).clicked() {
                self.open_import_csv();
            }
            if ui.button(self.t().new_table).clicked() {
                self.open_create_table_dialog();
            }
            if ui.button(self.t().compact_db).clicked() {
                self.do_vacuum(_ctx);
            }
            if ui.button(self.t().erd_view).clicked() {
                self.open_erd();
            }
        });

        ui.separator();

        if !self.db_path.is_empty() {
            let filename = std::path::Path::new(&self.db_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            ui.label(
                RichText::new(format!("{} {filename}", self.t().file_loaded))
                    .small()
                    .color(Color32::GRAY),
            );
        }

        ui.add_space(4.0);
        ui.label(self.t().tables);
        let hint = self.t().search_placeholder;
        ui.add_enabled(
            db_loaded,
            egui::TextEdit::singleline(&mut self.table_search)
                .hint_text(hint)
                .desired_width(f32::INFINITY),
        );

        ui.separator();

        let search_lower = self.table_search.to_lowercase();
        ScrollArea::vertical().id_salt("table_list").show(ui, |ui| {
            let tables: Vec<String> = self
                .tables
                .iter()
                .filter(|t| search_lower.is_empty() || t.to_lowercase().contains(&search_lower))
                .cloned()
                .collect();

            let mut to_load: Option<String> = None;
            let mut ctx_table: Option<(String, egui::Pos2)> = None;

            for table in &tables {
                let selected = self.selected_table.as_deref() == Some(table.as_str());
                let resp = ui.selectable_label(selected, table);
                if resp.clicked() {
                    to_load = Some(table.clone());
                }
                if resp.secondary_clicked() {
                    let pos = resp.interact_pointer_pos().unwrap_or(resp.rect.center());
                    ctx_table = Some((table.clone(), pos));
                }
            }

            if let Some(t) = to_load {
                self.load_table(&t);
            }
            if let Some(t) = ctx_table {
                self.sidebar_ctx_table = Some(t);
            }
        });
    }
}
