use egui::{Color32, RichText, ScrollArea, Ui};

use crate::db::{ColumnDef, ForeignKey, ForeignKeyDef};
use crate::localization::T;

// ── Create Table Dialog ───────────────────────────────────────────────────────

pub struct ColumnRow {
    pub name: String,
    pub col_type: String,
    pub pk: bool,
    pub ai: bool,
    pub nn: bool,
}

impl Default for ColumnRow {
    fn default() -> Self {
        Self {
            name: String::new(),
            col_type: "TEXT".into(),
            pk: false,
            ai: false,
            nn: false,
        }
    }
}

pub struct FkRow {
    pub from_col: String,
    pub ref_table: String,
    pub ref_col: String,
}

pub struct CreateTableDialog {
    pub table_name: String,
    pub columns: Vec<ColumnRow>,
    pub fks: Vec<FkRow>,
}

impl CreateTableDialog {
    pub fn new() -> Self {
        let mut d = Self {
            table_name: String::new(),
            columns: vec![],
            fks: vec![],
        };
        d.columns.push(ColumnRow {
            name: "id".into(),
            col_type: "INTEGER".into(),
            pk: true,
            ai: true,
            nn: false,
        });
        d
    }

    /// Returns Some((name, cols, fks)) when user clicks Save.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        t: &'static T,
        existing_tables: &[String],
    ) -> Option<(String, Vec<ColumnDef>, Vec<ForeignKeyDef>)> {
        let mut result = None;

        ui.horizontal(|ui| {
            ui.label(t.table_name_title);
            ui.text_edit_singleline(&mut self.table_name);
        });

        ui.separator();
        ui.label(t.columns);

        // Header
        ui.horizontal(|ui| {
            ui.add_sized(
                [150.0, 18.0],
                egui::Label::new(RichText::new(t.col_name).strong()),
            );
            ui.add_sized(
                [90.0, 18.0],
                egui::Label::new(RichText::new(t.col_type).strong()),
            );
            ui.add_sized([30.0, 18.0], egui::Label::new(RichText::new(t.pk).strong()));
            ui.add_sized([30.0, 18.0], egui::Label::new(RichText::new(t.ai).strong()));
            ui.add_sized([30.0, 18.0], egui::Label::new(RichText::new(t.nn).strong()));
        });

        ScrollArea::vertical()
            .id_salt("col_rows")
            .max_height(180.0)
            .show(ui, |ui| {
                let mut to_remove = None;
                for (i, row) in self.columns.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut row.name));
                        egui::ComboBox::from_id_salt(format!("ct_{i}"))
                            .selected_text(&row.col_type)
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for tp in &["TEXT", "INTEGER", "REAL", "BLOB", "NUMERIC"] {
                                    ui.selectable_value(&mut row.col_type, tp.to_string(), *tp);
                                }
                            });
                        ui.checkbox(&mut row.pk, "");
                        ui.checkbox(&mut row.ai, "");
                        ui.checkbox(&mut row.nn, "");
                        if ui
                            .small_button(RichText::new(t.remove).color(Color32::RED))
                            .clicked()
                        {
                            to_remove = Some(i);
                        }
                    });
                }
                if let Some(i) = to_remove {
                    self.columns.remove(i);
                }
            });

        if ui.button(t.add_column).clicked() {
            self.columns.push(ColumnRow::default());
        }

        ui.separator();
        ui.label(t.foreign_keys);

        ScrollArea::vertical()
            .id_salt("fk_rows")
            .max_height(120.0)
            .show(ui, |ui| {
                let mut to_remove = None;
                for (i, fk) in self.fks.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [120.0, 20.0],
                            egui::TextEdit::singleline(&mut fk.from_col).hint_text(t.col_name),
                        );

                        let ref_table_display = if fk.ref_table.is_empty() {
                            "—".to_string()
                        } else {
                            fk.ref_table.clone()
                        };
                        egui::ComboBox::from_id_salt(format!("fk_tbl_{i}"))
                            .selected_text(ref_table_display)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for tbl in existing_tables {
                                    ui.selectable_value(&mut fk.ref_table, tbl.clone(), tbl);
                                }
                            });

                        ui.add_sized(
                            [100.0, 20.0],
                            egui::TextEdit::singleline(&mut fk.ref_col).hint_text("id"),
                        );
                        if ui
                            .small_button(RichText::new(t.remove).color(Color32::RED))
                            .clicked()
                        {
                            to_remove = Some(i);
                        }
                    });
                }
                if let Some(i) = to_remove {
                    self.fks.remove(i);
                }
            });

        if ui.button(t.add_fk).clicked() {
            self.fks.push(FkRow {
                from_col: String::new(),
                ref_table: String::new(),
                ref_col: "id".into(),
            });
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(t.save).clicked() && !self.table_name.trim().is_empty() {
                let cols: Vec<ColumnDef> = self
                    .columns
                    .iter()
                    .map(|r| ColumnDef {
                        name: r.name.clone(),
                        col_type: r.col_type.clone(),
                        pk: r.pk,
                        ai: r.ai,
                        nn: r.nn,
                    })
                    .collect();
                let fks: Vec<ForeignKeyDef> = self
                    .fks
                    .iter()
                    .filter(|f| !f.from_col.is_empty() && !f.ref_table.is_empty())
                    .map(|f| ForeignKeyDef {
                        from_col: f.from_col.clone(),
                        ref_table: f.ref_table.clone(),
                        ref_col: f.ref_col.clone(),
                    })
                    .collect();
                result = Some((self.table_name.trim().to_string(), cols, fks));
            }
        });

        result
    }
}

// ── Edit / Insert Record Dialog ───────────────────────────────────────────────

pub struct EditRecordDialog {
    pub rowid: Option<i64>,
    pub columns: Vec<String>,
    pub values: Vec<String>,
}

impl EditRecordDialog {
    pub fn for_insert(columns: Vec<String>) -> Self {
        let values = vec![String::new(); columns.len()];
        Self {
            rowid: None,
            columns,
            values,
        }
    }

    pub fn for_edit(rowid: i64, columns: Vec<String>, values: Vec<String>) -> Self {
        Self {
            rowid: Some(rowid),
            columns,
            values,
        }
    }

    /// Returns Some(pairs) when user clicks Save/Insert.
    pub fn show(&mut self, ui: &mut Ui, t: &'static T) -> Option<Result<Vec<(String, String)>, ()>> {
        let mut result = None;

        ScrollArea::vertical()
            .id_salt("edit_scroll")
            .max_height(400.0)
            .show(ui, |ui| {
                for (col, val) in self.columns.iter().zip(self.values.iter_mut()) {
                    ui.label(RichText::new(col.as_str()).strong());
                    ui.text_edit_singleline(val);
                    ui.add_space(2.0);
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(t.cancel).clicked() {
                result = Some(Err(()));
            }
            let btn_label = if self.rowid.is_some() { t.save } else { t.insert };
            if ui.button(btn_label).clicked() {
                let pairs: Vec<(String, String)> = self
                    .columns
                    .iter()
                    .cloned()
                    .zip(self.values.iter().cloned())
                    .collect();
                result = Some(Ok(pairs));
            }
        });

        result
    }
}

// ── ERD Window ────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ErdTable {
    name: String,
    cols: Vec<String>,
    fks: Vec<ForeignKey>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

pub struct ErdWindow {
    tables: Vec<ErdTable>,
    drag_idx: Option<usize>,
    drag_offset: egui::Vec2,
}

impl ErdWindow {
    pub fn new(tables_data: Vec<(String, Vec<String>, Vec<ForeignKey>)>) -> Self {
        let n = tables_data.len();
        let center_x = 400.0f32;
        let center_y = 300.0f32;
        let radius = (n as f32 * 30.0).min(280.0).max(100.0);

        let tables: Vec<ErdTable> = tables_data
            .into_iter()
            .enumerate()
            .map(|(i, (name, cols, fks))| {
                let h = 30.0 + cols.len() as f32 * 20.0;
                let w = 160.0f32;
                let angle = std::f32::consts::TAU * i as f32 / n as f32;
                let x = center_x + radius * angle.cos() - w / 2.0;
                let y = center_y + radius * angle.sin() - h / 2.0;
                ErdTable {
                    name,
                    cols,
                    fks,
                    x,
                    y,
                    w,
                    h,
                }
            })
            .collect();

        Self {
            tables,
            drag_idx: None,
            drag_offset: egui::Vec2::ZERO,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
        let rect = response.rect;

        // Draw FK lines
        let positions: Vec<(f32, f32)> = self
            .tables
            .iter()
            .map(|t| (t.x + t.w / 2.0, t.y + t.h / 2.0))
            .collect();

        for (i, t) in self.tables.iter().enumerate() {
            for fk in &t.fks {
                if let Some(j) = self.tables.iter().position(|t2| t2.name == fk.ref_table) {
                    let p1 =
                        egui::Pos2::new(rect.left() + positions[i].0, rect.top() + positions[i].1);
                    let p2 =
                        egui::Pos2::new(rect.left() + positions[j].0, rect.top() + positions[j].1);
                    painter.line_segment(
                        [p1, p2],
                        egui::Stroke::new(1.5, Color32::from_rgb(80, 80, 120)),
                    );
                    // Arrow head
                    let dir = (p2 - p1).normalized();
                    let perp = egui::Vec2::new(-dir.y, dir.x);
                    let tip = p2 - dir * 10.0;
                    painter.add(egui::Shape::convex_polygon(
                        vec![p2, tip + perp * 5.0, tip - perp * 5.0],
                        Color32::from_rgb(80, 80, 180),
                        egui::Stroke::NONE,
                    ));
                }
            }
        }

        // Draw table boxes
        for (i, t) in self.tables.iter().enumerate() {
            let top_left = rect.left_top() + egui::Vec2::new(t.x, t.y);
            let box_rect = egui::Rect::from_min_size(top_left, egui::Vec2::new(t.w, t.h));
            let header_rect = egui::Rect::from_min_size(top_left, egui::Vec2::new(t.w, 25.0));

            painter.rect_filled(box_rect, 4.0, Color32::from_rgb(50, 50, 55));
            painter.rect_stroke(
                box_rect,
                4.0,
                egui::Stroke::new(1.0, Color32::from_rgb(80, 80, 90)),
            );
            painter.rect_filled(header_rect, 4.0, Color32::from_rgb(31, 83, 141));

            painter.text(
                header_rect.center(),
                egui::Align2::CENTER_CENTER,
                &t.name,
                egui::FontId::proportional(12.0),
                Color32::WHITE,
            );

            for (ci, col) in t.cols.iter().enumerate() {
                painter.text(
                    top_left + egui::Vec2::new(8.0, 38.0 + ci as f32 * 20.0),
                    egui::Align2::LEFT_CENTER,
                    col,
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(200, 200, 200),
                );
            }

            // Hit-test for drag
            let interact_rect = egui::Rect::from_min_size(top_left, egui::Vec2::new(t.w, 25.0));
            let pointer = ui.input(|i| i.pointer.hover_pos());
            if let Some(pos) = pointer {
                if interact_rect.contains(pos) {
                    if ui.input(|i| i.pointer.primary_pressed()) {
                        self.drag_idx = Some(i);
                        self.drag_offset = pos - top_left;
                    }
                }
            }
        }

        // Handle drag movement
        if let Some(drag_idx) = self.drag_idx {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                let new_tl = pos - self.drag_offset;
                self.tables[drag_idx].x = new_tl.x - rect.left();
                self.tables[drag_idx].y = new_tl.y - rect.top();
            }
            if ui.input(|i| i.pointer.primary_released()) {
                self.drag_idx = None;
            }
        }
    }
}
