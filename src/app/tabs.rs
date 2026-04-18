use egui::{Color32, FontId, RichText, ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};

use super::{ActiveTab, App};

impl App {
    pub(super) fn show_main(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if self.db.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(self.t().no_file).size(18.0).color(Color32::GRAY));
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
                if ui.add_enabled(self.selected_table.is_some(), egui::Button::new(self.t().new_record)).clicked() {
                    self.open_insert_dialog();
                }
                if ui.add_enabled(self.selected_table.is_some(), egui::Button::new(self.t().export)).clicked() {
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

    // ── Data Tab ─────────────────────────────────────────────────────────────

    pub(super) fn show_data_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let t = self.t();
            let all_col = t.all_columns.to_string();
            let opts: Vec<String> = self.filter_col_options.clone();

            egui::ComboBox::from_id_salt("filter_col")
                .selected_text(if self.filter.column.is_empty() { &all_col } else { &self.filter.column })
                .width(120.0)
                .show_ui(ui, |ui| {
                    for opt in &opts {
                        ui.selectable_value(&mut self.filter.column, opt.clone(), opt);
                    }
                });

            let ops = ["LIKE", "=", "!=", ">", "<", ">=", "<="];
            egui::ComboBox::from_id_salt("filter_op")
                .selected_text(&self.filter.operator)
                .width(70.0)
                .show_ui(ui, |ui| {
                    for op in ops {
                        ui.selectable_value(&mut self.filter.operator, op.to_string(), op);
                    }
                });

            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.filter.value)
                    .hint_text(t.search_data_placeholder)
                    .desired_width(200.0)
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.start_data_load(1, true);
            }

            if ui.button(t.search_btn).clicked() { self.start_data_load(1, true); }
            if ui.button(t.clear_btn).clicked() {
                self.filter.value.clear();
                self.filter.column.clear();
                self.filter.operator = "LIKE".into();
                self.start_data_load(1, true);
            }
        });

        if self.loading_data {
            ui.add_space(4.0);
            ui.add(egui::ProgressBar::new(f32::NAN).animate(true).desired_width(f32::INFINITY));
        }

        ui.add_space(4.0);

        let available = ui.available_height() - 30.0;
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ScrollArea::both().max_height(available).show(ui, |ui| {
                self.show_data_table(ui);
            });
        });

        ui.separator();
        ui.horizontal(|ui| {
            let t = self.t();
            if ui.add_enabled(self.current_page > 1, egui::Button::new(t.prev)).clicked() {
                let p = self.current_page - 1;
                self.start_data_load(p, false);
            }

            ui.label(format!("{} {} {} {}  ({} {})",
                t.tab_data, self.current_page, t.page_of, self.total_pages,
                self.total_rows, t.lines));

            if ui.add_enabled(self.current_page < self.total_pages, egui::Button::new(t.next)).clicked() {
                let p = self.current_page + 1;
                self.start_data_load(p, false);
            }
        });
    }

    fn show_data_table(&mut self, ui: &mut Ui) {
        let col_count = self.columns.len();
        if col_count == 0 {
            ui.label(RichText::new(self.t().no_file).color(Color32::GRAY));
            return;
        }

        let display_cols: Vec<&String> = self.columns.iter().skip(1).collect();
        let display_count = display_cols.len();

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(60.0));

        for _ in 0..display_count {
            table = table.column(Column::auto().at_least(80.0).clip(true));
        }

        let sort_col_clone = self.sort_col.clone();
        let sort_asc = self.sort_asc;
        let mut new_sort: Option<(String, bool)> = None;
        let mut ctx_row: Option<(i64, Vec<String>, egui::Pos2)> = None;

        table
            .header(24.0, |mut header: egui_extras::TableRow| {
                header.col(|ui: &mut egui::Ui| { ui.label(""); });
                for col in &display_cols {
                    let col_str = col.to_string();
                    let sorted = sort_col_clone.as_deref() == Some(col.as_str());
                    let label = if sorted {
                        if sort_asc { format!("{col_str} ▲") } else { format!("{col_str} ▼") }
                    } else {
                        col_str.clone()
                    };
                    header.col(|ui: &mut egui::Ui| {
                        if ui.button(RichText::new(label.as_str()).strong()).clicked() {
                            let asc = if sorted { !sort_asc } else { true };
                            new_sort = Some((col_str.clone(), asc));
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(22.0, self.rows.len(), |mut row: egui_extras::TableRow| {
                    let row_idx = row.index();
                    let data = &self.rows[row_idx];
                    let rowid: i64 = data.first().and_then(|v| v.parse().ok()).unwrap_or(-1);

                    row.col(|ui: &mut egui::Ui| {
                        let resp = ui.small_button("✏");
                        if resp.clicked() {
                            let pos = resp.interact_pointer_pos().unwrap_or(resp.rect.center());
                            ctx_row = Some((rowid, data.clone(), pos));
                        }
                    });

                    for i in 1..col_count {
                        let val = data.get(i).map(String::as_str).unwrap_or("").to_string();
                        row.col(|ui: &mut egui::Ui| { ui.label(val.as_str()); });
                    }
                });
            });

        if let Some((col, asc)) = new_sort {
            self.sort_col = Some(col);
            self.sort_asc = asc;
            self.start_data_load(1, true);
        }

        if let Some((rowid, vals, pos)) = ctx_row {
            self.tree_ctx_row = Some((rowid, vals, pos));
        }
    }

    // ── Schema Tab ───────────────────────────────────────────────────────────

    pub(super) fn show_schema_tab(&mut self, ui: &mut Ui) {
        ScrollArea::both().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.schema_text.clone())
                    .font(FontId::monospace(13.0))
                    .desired_width(f32::INFINITY)
                    .interactive(false)
            );
        });
    }

    // ── Stats Tab ────────────────────────────────────────────────────────────

    pub(super) fn show_stats_tab(&mut self, ui: &mut Ui) {
        if self.stats.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("{} {}", self.t().calculating_stats, self.stats.current_col));
            });
            if ui.button(self.t().cancel).clicked() { self.stats.cancelled = true; }
            return;
        }

        if let Some(ref err) = self.stats.error.clone() {
            ui.colored_label(Color32::RED, format!("{} {err}", self.t().stats_error));
            return;
        }

        let table = self.selected_table.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            if ui.button(self.t().export_stats).clicked() {
                self.export_stats_json(&table);
            }
        });
        ui.separator();

        if let Some(ref data) = self.stats.result.clone() {
            let total = self.stats.total_rows;
            ScrollArea::vertical().show(ui, |ui| {
                ui.label(RichText::new(format!("{} {table}  —  {total} {}", self.t().stats_table, self.t().lines)).strong());
                ui.separator();
                for (col, s) in data {
                    ui.collapsing(format!("[{col}]"), |ui| {
                        ui.label(format!("{} {}", self.t().stats_filled, s.non_null_count));
                        ui.label(format!("{} {}", self.t().stats_empty, s.null_count));
                        ui.label(format!("{} {}", self.t().stats_unique, s.unique_count));
                        if let Some(ref v) = s.min_value { ui.label(format!("{} {v}", self.t().stats_min)); }
                        if let Some(ref v) = s.max_value { ui.label(format!("{} {v}", self.t().stats_max)); }
                        if let Some(a) = s.avg_value { ui.label(format!("{} {:.4}", self.t().stats_avg, a)); }
                        if !s.top_values.is_empty() {
                            ui.label(self.t().stats_top_values);
                            for (val, count) in s.top_values.iter().take(5) {
                                let pct = if total > 0 { *count as f64 / total as f64 * 100.0 } else { 0.0 };
                                ui.label(format!("  {val:<15}  {count}  ({pct:.1}%)"));
                            }
                        }
                    });
                }
            });
        } else {
            ui.label(self.t().stats_no_data);
        }
    }

    // ── SQL Tab ──────────────────────────────────────────────────────────────

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
                                .hint_text("-- SQL Query")
                        );

                        if resp.has_focus() && ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter)) {
                            self.run_query();
                        }
                    });

                ui.horizontal(|ui| {
                    if ui.add_enabled(self.db.is_some(), egui::Button::new(self.t().exec_sql)).clicked() {
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
                            let label = if label.len() > 40 { format!("{}…", &label[..40]) } else { label };
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
                            let label = if label.len() > 35 { format!("{}…", &label[..35]) } else { label };
                            ui.horizontal(|ui| {
                                if ui.small_button("🗑").clicked() { to_remove = Some(i); }
                                if ui.button(label).clicked() { self.sql_input = q.clone(); }
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
        if col_count == 0 { return; }

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
                    header.col(|ui: &mut egui::Ui| { ui.label(RichText::new(col.as_str()).strong()); });
                }
            })
            .body(|body| {
                body.rows(22.0, self.sql_rows.len(), |mut row: egui_extras::TableRow| {
                    let row_data = &self.sql_rows[row.index()];
                    for val in row_data {
                        let v = val.clone();
                        row.col(|ui: &mut egui::Ui| { ui.label(v.as_str()); });
                    }
                });
            });
    }
}
