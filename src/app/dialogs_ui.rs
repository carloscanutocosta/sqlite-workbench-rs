use egui::{Color32, RichText, Vec2};

use super::{App, InputAction, InputDialog};
use crate::db::{ColumnDef, ForeignKeyDef};

impl App {
    pub(super) fn show_dialogs(&mut self, ctx: &egui::Context) {
        // ── Single-field input dialog ─────────────────────────────────────────
        if let Some(ref mut dialog) = self.input_dialog {
            let mut open = true;
            let action = dialog.action.clone();
            let title = dialog.title.clone();
            let label = dialog.label.clone();

            egui::Window::new(&title)
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(&label);
                    let resp =
                        ui.text_edit_singleline(&mut self.input_dialog.as_mut().unwrap().value);
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let value = self.input_dialog.take().unwrap().value;
                        self.execute_input_action(action, value);
                        return;
                    }
                    ui.horizontal(|ui| {
                        if ui.button(self.t().cancel).clicked() {
                            self.input_dialog = None;
                        }
                        if ui.button(self.t().save).clicked() {
                            let value = self.input_dialog.take().unwrap().value;
                            self.execute_input_action(action, value);
                        }
                    });
                });
            if !open {
                self.input_dialog = None;
            }
        }

        // ── Create table dialog ───────────────────────────────────────────────
        {
            let tables = self.tables.clone();
            let t = self.t();
            let title = t.create_table_title;
            let mut open = true;
            let mut result: Option<(String, Vec<ColumnDef>, Vec<ForeignKeyDef>)> = None;

            if self.create_table_dialog.is_some() {
                egui::Window::new(title)
                    .collapsible(false)
                    .min_width(600.0)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        if let Some(dlg) = self.create_table_dialog.as_mut() {
                            result = dlg.show(ui, t, &tables);
                        }
                    });

                if let Some((name, cols, fks)) = result {
                    if let Some(db) = &self.db {
                        match db.create_table(&name, &cols, &fks) {
                            Ok(()) => {
                                self.refresh_tables();
                                self.load_table(&name);
                                self.toast(self.t().success.to_string());
                                self.create_table_dialog = None;
                            }
                            Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                        }
                    }
                }
                if !open {
                    self.create_table_dialog = None;
                }
            }
        }

        // ── Edit / Insert record dialog ───────────────────────────────────────
        {
            let t = self.t();
            let is_insert = self
                .edit_dialog
                .as_ref()
                .map(|d| d.rowid.is_none())
                .unwrap_or(false);
            let title = if is_insert {
                t.new_record
            } else {
                t.edit_values
            };
            let mut open = true;
            let mut action: Option<Result<Vec<(String, String)>, ()>> = None;

            if self.edit_dialog.is_some() {
                egui::Window::new(title)
                    .collapsible(false)
                    .min_width(400.0)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        if let Some(dlg) = self.edit_dialog.as_mut() {
                            action = dlg.show(ui, t);
                        }
                    });

                match action {
                    Some(Ok(pairs)) => {
                        let table = self.selected_table.clone().unwrap_or_default();
                        let rowid = self.edit_dialog.as_ref().and_then(|d| d.rowid);
                        let result = if let Some(rid) = rowid {
                            self.db.as_ref().unwrap().update_record(&table, rid, &pairs)
                        } else {
                            self.db.as_ref().unwrap().insert_record(&table, &pairs)
                        };
                        match result {
                            Ok(()) => {
                                self.toast(if rowid.is_some() {
                                    self.t().record_updated
                                } else {
                                    self.t().record_inserted
                                });
                                self.edit_dialog = None;
                                self.start_data_load(self.current_page, rowid.is_none());
                            }
                            Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                        }
                    }
                    Some(Err(())) => self.edit_dialog = None,
                    None => {}
                }
                if !open {
                    self.edit_dialog = None;
                }
            }
        }

        // ── ERD window ────────────────────────────────────────────────────────
        {
            let t = self.t();
            let mut open = true;
            if self.erd_window.is_some() {
                egui::Window::new(t.erd_view)
                    .min_size(Vec2::new(800.0, 600.0))
                    .open(&mut open)
                    .show(ctx, |ui| {
                        if let Some(erd) = self.erd_window.as_mut() {
                            erd.show(ui);
                        }
                    });
                if !open {
                    self.erd_window = None;
                }
            }
        }
    }

    pub(super) fn execute_input_action(&mut self, action: InputAction, value: String) {
        match action {
            InputAction::RenameTable(old) => {
                if !value.trim().is_empty() {
                    if let Some(db) = &self.db {
                        match db.rename_table(&old, value.trim()) {
                            Ok(()) => {
                                if self.selected_table.as_deref() == Some(old.as_str()) {
                                    self.selected_table = Some(value.trim().to_string());
                                }
                                self.refresh_tables();
                                self.toast(self.t().success.to_string());
                            }
                            Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                        }
                    }
                }
            }
            InputAction::ImportCsvName(csv_path) => {
                let name = value.trim().to_string();
                if !name.is_empty() {
                    if let Some(db) = &self.db {
                        match db.import_csv(&csv_path, &name) {
                            Ok(n) => {
                                self.refresh_tables();
                                self.load_table(&name);
                                self.toast(format!("{} ({n} rows)", self.t().success));
                            }
                            Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                        }
                    }
                }
            }
            InputAction::NewTableName => {}
        }
    }

    pub(super) fn show_context_menus(&mut self, ctx: &egui::Context) {
        // ── Sidebar context menu (right-click on table) ───────────────────────
        if let Some((ref table, pos)) = self.sidebar_ctx_table.clone() {
            egui::Area::new(egui::Id::new("sidebar_ctx"))
                .order(egui::Order::Tooltip)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        if ui.button(self.t().rename).clicked() {
                            self.input_dialog = Some(InputDialog {
                                title: self.t().rename.to_string(),
                                label: format!("{} {table}:", self.t().rename_table_prompt),
                                value: table.clone(),
                                action: InputAction::RenameTable(table.clone()),
                            });
                            self.sidebar_ctx_table = None;
                        }
                        if ui
                            .button(RichText::new(self.t().delete).color(Color32::RED))
                            .clicked()
                        {
                            if let Some(db) = &self.db {
                                match db.drop_table(table) {
                                    Ok(()) => {
                                        if self.selected_table.as_deref() == Some(table.as_str()) {
                                            self.selected_table = None;
                                            self.columns.clear();
                                            self.rows.clear();
                                        }
                                        self.refresh_tables();
                                        self.toast(self.t().success.to_string());
                                    }
                                    Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                                }
                            }
                            self.sidebar_ctx_table = None;
                        }
                    });
                });

            if ctx.input(|i| i.pointer.any_click()) {
                let released_outside = !ctx.is_pointer_over_area();
                if released_outside {
                    self.sidebar_ctx_table = None;
                }
            }
        }

        // ── Data row context menu (edit / delete row) ─────────────────────────
        if let Some((rowid, ref vals, pos)) = self.tree_ctx_row.clone() {
            egui::Area::new(egui::Id::new("tree_ctx"))
                .order(egui::Order::Tooltip)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        if ui.button(self.t().edit).clicked() {
                            self.open_edit_dialog(rowid, vals.clone());
                            self.tree_ctx_row = None;
                        }
                        if ui
                            .button(RichText::new(self.t().delete).color(Color32::RED))
                            .clicked()
                        {
                            if let Some(db) = &self.db {
                                let table = self.selected_table.clone().unwrap_or_default();
                                match db.delete_record(&table, rowid) {
                                    Ok(()) => {
                                        self.toast(self.t().success.to_string());
                                        self.start_data_load(self.current_page, true);
                                    }
                                    Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                                }
                            }
                            self.tree_ctx_row = None;
                        }
                    });
                });

            if ctx.input(|i| i.pointer.any_click()) && !ctx.is_pointer_over_area() {
                self.tree_ctx_row = None;
            }
        }
    }

    pub(super) fn show_about_window(&mut self, ctx: &egui::Context) {
        if !self.show_about { return; }

        let t = self.t();
        let mut open = self.show_about;

        egui::Window::new(t.about_title)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(360.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("SQLite Workbench")
                            .size(22.0)
                            .strong()
                            .color(Color32::from_rgb(100, 160, 230)),
                    );
                    ui.label(
                        RichText::new(t.about_description)
                            .size(12.0)
                            .color(Color32::GRAY),
                    );
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                egui::Grid::new("about_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new(t.about_org).strong());
                        ui.label("NORMAXIS");
                        ui.end_row();

                        ui.label(RichText::new(t.about_author).strong());
                        ui.label("Carlos Canuto Costa");
                        ui.end_row();

                        ui.label(RichText::new(t.about_version).strong());
                        ui.label(env!("CARGO_PKG_VERSION"));
                        ui.end_row();

                        ui.label(RichText::new(t.about_license).strong());
                        ui.label("EUPL v1.2");
                        ui.end_row();

                        ui.label(RichText::new(t.about_repository).strong());
                        ui.hyperlink_to(
                            "github.com/carloscanutocosta/sqlite-workbench-rs",
                            "https://github.com/carloscanutocosta/sqlite-workbench-rs",
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(t.about_copyright)
                            .size(11.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(8.0);
                    if ui.button(t.about_close).clicked() {
                        self.show_about = false;
                    }
                });

                ui.add_space(4.0);
            });

        self.show_about = open;
    }
}
