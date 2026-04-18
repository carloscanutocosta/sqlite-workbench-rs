use egui::{Color32, FontId, RichText, ScrollArea, Ui};

use super::{ActiveTab, App};

impl App {
    pub(super) fn show_main(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if self.db.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new(self.t().no_file)
                        .size(18.0)
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        ui.horizontal(|ui| {
            let table_name = self.selected_table.clone().unwrap_or_default();
            if table_name.is_empty() {
                ui.label(RichText::new(self.t().no_file).color(Color32::GRAY));
            } else {
                ui.label(RichText::new(format!("{} {table_name}", self.t().table_label)).strong());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.selected_table.is_some(),
                        egui::Button::new(self.t().new_record),
                    )
                    .clicked()
                {
                    self.open_insert_dialog();
                }
                if ui
                    .add_enabled(
                        self.selected_table.is_some(),
                        egui::Button::new(self.t().export),
                    )
                    .clicked()
                {
                    self.export_data();
                }
            });
        });

        ui.separator();

        ui.horizontal(|ui| {
            let t = self.t();
            if ui.selectable_label(self.active_tab == ActiveTab::Data, t.tab_data).clicked() {
                self.active_tab = ActiveTab::Data;
            }
            if ui.selectable_label(self.active_tab == ActiveTab::Schema, t.tab_schema).clicked() {
                self.active_tab = ActiveTab::Schema;
            }
            if ui.selectable_label(self.active_tab == ActiveTab::Stats, t.tab_stats).clicked() {
                self.active_tab = ActiveTab::Stats;
            }
            if ui.selectable_label(self.active_tab == ActiveTab::Sql, t.tab_sql).clicked() {
                self.active_tab = ActiveTab::Sql;
            }
        });
        ui.separator();

        match self.active_tab.clone() {
            ActiveTab::Data => self.show_data_tab(ui),
            ActiveTab::Schema => self.show_schema_tab(ui),
            ActiveTab::Stats => self.show_stats_tab(ui),
            ActiveTab::Sql => self.show_sql_tab(ui, ctx),
        }
    }

    pub(super) fn show_schema_tab(&mut self, ui: &mut Ui) {
        ScrollArea::both().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.schema_text.clone())
                    .font(FontId::monospace(13.0))
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        });
    }
}
