use egui::{Color32, FontId, RichText, ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};

use super::App;

impl App {
    pub(super) fn show_sql_tab(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        let available_height = ui.available_height();
        let editor_height = available_height * 0.35;
        let result_height = available_height * 0.45;

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                ScrollArea::vertical()
                    .id_salt("sql_editor_scroll")
                    .max_height(editor_height)
                    .show(ui, |ui| {
                        let resp = ui.add(
                            egui::TextEdit::multiline(&mut self.sql_input)
                                .font(FontId::monospace(13.0))
                                .desired_width(f32::INFINITY)
                                .desired_rows(10)
                                .hint_text("-- SQL Query"),
                        );

                        if resp.has_focus()
                            && ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter))
                        {
                            self.run_query();
                        }
                    });

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.db.is_some(), egui::Button::new(self.t().exec_sql))
                        .clicked()
                    {
                        self.run_query();
                    }
                    if ui.button("★").on_hover_text("Add to favorites").clicked() {
                        let q = self.sql_input.trim().to_string();
                        if !q.is_empty() && !self.favorites.contains(&q) {
                            self.favorites.push(q);
                            self.save_favorites();
                        }
                    }
                });

                if let Some(ref err) = self.sql_error.clone() {
                    ui.colored_label(Color32::RED, err);
                }

                ScrollArea::both()
                    .id_salt("sql_results_scroll")
                    .max_height(result_height)
                    .show(ui, |ui| {
                        self.show_sql_results(ui);
                    });
            });

            cols[1].vertical(|ui| {
                ui.heading(self.t().tab_history);
                ScrollArea::vertical()
                    .id_salt("history_scroll")
                    .max_height(available_height * 0.45)
                    .show(ui, |ui| {
                        let history: Vec<String> = self.history.iter().cloned().collect();
                        for q in &history {
                            let label = q.replace('\n', " ");
                            let label = if label.len() > 40 {
                                format!("{}…", &label[..40])
                            } else {
                                label
                            };
                            if ui.button(label).clicked() {
                                self.sql_input = q.clone();
                            }
                        }
                    });

                ui.separator();
                ui.heading(self.t().tab_favorites);
                ScrollArea::vertical()
                    .id_salt("favorites_scroll")
                    .show(ui, |ui| {
                        let favorites: Vec<String> = self.favorites.clone();
                        let mut to_remove: Option<usize> = None;
                        for (i, q) in favorites.iter().enumerate() {
                            let label = q.replace('\n', " ");
                            let label = if label.len() > 35 {
                                format!("{}…", &label[..35])
                            } else {
                                label
                            };
                            ui.horizontal(|ui| {
                                if ui.small_button("🗑").clicked() {
                                    to_remove = Some(i);
                                }
                                if ui.button(label).clicked() {
                                    self.sql_input = q.clone();
                                }
                            });
                        }
                        if let Some(i) = to_remove {
                            self.favorites.remove(i);
                            self.save_favorites();
                        }
                    });
            });
        });
    }

    fn show_sql_results(&mut self, ui: &mut Ui) {
        let col_count = self.sql_columns.len();
        if col_count == 0 {
            return;
        }

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

        for _ in 0..col_count {
            table = table.column(Column::auto().at_least(80.0).clip(true));
        }

        table
            .header(24.0, |mut header: egui_extras::TableRow| {
                for col in &self.sql_columns {
                    header.col(|ui: &mut egui::Ui| {
                        ui.label(RichText::new(col.as_str()).strong());
                    });
                }
            })
            .body(|body| {
                body.rows(22.0, self.sql_rows.len(), |mut row: egui_extras::TableRow| {
                    let row_data = &self.sql_rows[row.index()];
                    for val in row_data {
                        let v = val.clone();
                        row.col(|ui: &mut egui::Ui| {
                            ui.label(v.as_str());
                        });
                    }
                });
            });
    }
}
