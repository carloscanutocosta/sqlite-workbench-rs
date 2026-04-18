use egui::{Color32, RichText, ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};

use super::App;

impl App {
    pub(super) fn show_data_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let t = self.t();
            let all_col = t.all_columns.to_string();
            let opts: Vec<String> = self.filter_col_options.clone();

            egui::ComboBox::from_id_salt("filter_col")
                .selected_text(if self.filter.column.is_empty() {
                    &all_col
                } else {
                    &self.filter.column
                })
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
                    .desired_width(200.0),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.start_data_load(1, true);
            }

            if ui.button(t.search_btn).clicked() {
                self.start_data_load(1, true);
            }
            if ui.button(t.clear_btn).clicked() {
                self.filter.value.clear();
                self.filter.column.clear();
                self.filter.operator = "LIKE".into();
                self.start_data_load(1, true);
            }
        });

        if self.loading_data {
            ui.add_space(4.0);
            ui.add(
                egui::ProgressBar::new(f32::NAN)
                    .animate(true)
                    .desired_width(f32::INFINITY),
            );
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
            if ui
                .add_enabled(self.current_page > 1, egui::Button::new(t.prev))
                .clicked()
            {
                let p = self.current_page - 1;
                self.start_data_load(p, false);
            }

            ui.label(format!(
                "{} {} {} {}  ({} {})",
                t.tab_data,
                self.current_page,
                t.page_of,
                self.total_pages,
                self.total_rows,
                t.lines
            ));

            if ui
                .add_enabled(self.current_page < self.total_pages, egui::Button::new(t.next))
                .clicked()
            {
                let p = self.current_page + 1;
                self.start_data_load(p, false);
            }
        });
    }

    pub(super) fn show_data_table(&mut self, ui: &mut Ui) {
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
                header.col(|ui: &mut egui::Ui| {
                    ui.label("");
                });
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
                        row.col(|ui: &mut egui::Ui| {
                            ui.label(val.as_str());
                        });
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
}
