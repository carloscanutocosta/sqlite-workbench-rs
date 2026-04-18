use crate::db::DatabaseManager;
use crate::dialogs::{EditRecordDialog, ErdWindow};
use super::{ActiveTab, App, InputAction, InputDialog};

impl App {
    pub(super) fn open_db_file(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
            .add_filter("All Files", &["*"])
            .pick_file();

        if let Some(p) = path {
            let path_str = p.to_string_lossy().to_string();
            match DatabaseManager::open(&path_str) {
                Ok(db) => {
                    self.db = Some(db);
                    self.db_path = path_str;
                    self.selected_table = None;
                    self.columns.clear();
                    self.rows.clear();
                    self.schema_text.clear();
                    self.refresh_tables();
                    if self.tables.is_empty() {
                        self.toast(self.t().db_empty_warning.to_string());
                    }
                }
                Err(e) => self.toast(format!("{}: {e}", self.t().error)),
            }
        }
    }

    pub(super) fn open_import_csv(&mut self) {
        if self.db.is_none() { self.toast(self.t().load_db_first.to_string()); return; }
        let path = rfd::FileDialog::new()
            .add_filter("CSV Files", &["csv"])
            .add_filter("All Files", &["*"])
            .set_title(self.t().select_csv_title)
            .pick_file();

        if let Some(p) = path {
            let csv_path = p.to_string_lossy().to_string();
            self.input_dialog = Some(InputDialog {
                title: self.t().table_name_title.to_string(),
                label: self.t().enter_table_name.to_string(),
                value: String::new(),
                action: InputAction::ImportCsvName(csv_path),
            });
        }
    }

    pub(super) fn do_vacuum(&mut self, _ctx: &egui::Context) {
        if let Some(db) = &self.db {
            match db.vacuum() {
                Ok(()) => self.toast(self.t().vacuum_success.to_string()),
                Err(e) => self.toast(format!("{}: {e}", self.t().error)),
            }
        }
    }

    pub(super) fn open_create_table_dialog(&mut self) {
        use crate::dialogs::CreateTableDialog;
        self.create_table_dialog = Some(CreateTableDialog::new());
    }

    pub(super) fn open_erd(&mut self) {
        if let Some(db) = &self.db {
            let tables: Vec<(String, Vec<String>, Vec<crate::db::ForeignKey>)> = self.tables.iter()
                .map(|t| {
                    let cols = db.get_columns(t).unwrap_or_default();
                    let fks = db.get_foreign_keys(t).unwrap_or_default();
                    (t.clone(), cols, fks)
                })
                .collect();
            self.erd_window = Some(ErdWindow::new(tables));
        }
    }

    pub(super) fn open_insert_dialog(&mut self) {
        let cols: Vec<String> = self.columns.iter().skip(1).cloned().collect();
        self.edit_dialog = Some(EditRecordDialog::for_insert(cols));
    }

    pub(super) fn open_edit_dialog(&mut self, rowid: i64, vals: Vec<String>) {
        let cols: Vec<String> = self.columns.iter().skip(1).cloned().collect();
        let values: Vec<String> = vals.into_iter().skip(1).collect();
        self.edit_dialog = Some(EditRecordDialog::for_edit(rowid, cols, values));
    }

    pub(super) fn run_query(&mut self) {
        if self.db.is_none() { self.toast(self.t().load_db_first.to_string()); return; }
        let query = self.sql_input.trim().to_string();
        if query.is_empty() { return; }

        self.sql_error = None;
        self.sql_columns.clear();
        self.sql_rows.clear();

        match self.db.as_ref().unwrap().execute_query(&query) {
            Ok((cols, rows)) => {
                self.sql_columns = cols;
                self.sql_rows = rows;
                self.push_history(&query);
                let up = query.to_uppercase();
                if up.starts_with("CREATE") || up.starts_with("DROP") || up.starts_with("ALTER") {
                    self.refresh_tables();
                }
            }
            Err(e) => self.sql_error = Some(e),
        }
    }

    pub(super) fn export_data(&mut self) {
        let (cols, rows) = match self.active_tab {
            ActiveTab::Sql => (&self.sql_columns, &self.sql_rows),
            _ => (&self.columns, &self.rows),
        };
        if rows.is_empty() { self.toast(self.t().no_data_export.to_string()); return; }

        let base = self.selected_table.clone().unwrap_or_else(|| "export".to_string());
        let path = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .add_filter("JSON", &["json"])
            .set_file_name(format!("{base}_export.csv"))
            .save_file();

        if let Some(p) = path {
            let p_str = p.to_string_lossy().to_string();
            let skip = if matches!(self.active_tab, ActiveTab::Data) { 1 } else { 0 };
            let display_cols: Vec<&String> = cols.iter().skip(skip).collect();
            let result = if p_str.to_lowercase().ends_with(".json") {
                self.export_json(&p_str, &display_cols, rows, skip)
            } else {
                self.export_csv(&p_str, &display_cols, rows, skip)
            };
            match result {
                Ok(()) => self.toast(self.t().export_success.to_string()),
                Err(e) => self.toast(format!("{}: {e}", self.t().export_error)),
            }
        }
    }

    fn export_csv(&self, path: &str, cols: &[&String], rows: &[Vec<String>], skip: usize) -> Result<(), String> {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b';')
            .from_path(path)
            .map_err(|e| e.to_string())?;
        wtr.write_record(cols.iter().map(|c| c.as_str())).map_err(|e| e.to_string())?;
        for row in rows {
            let vals: Vec<&str> = row.iter().skip(skip).map(String::as_str).collect();
            wtr.write_record(&vals).map_err(|e| e.to_string())?;
        }
        wtr.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn export_json(&self, path: &str, cols: &[&String], rows: &[Vec<String>], skip: usize) -> Result<(), String> {
        let data: Vec<serde_json::Value> = rows.iter().map(|row| {
            let obj: serde_json::Map<String, serde_json::Value> = cols.iter().zip(row.iter().skip(skip))
                .map(|(c, v)| (c.to_string(), serde_json::Value::String(v.clone())))
                .collect();
            serde_json::Value::Object(obj)
        }).collect();
        let s = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(super) fn export_stats_json(&mut self, table: &str) {
        if let Some(ref data) = self.stats_export_data.clone() {
            let path = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name(format!("stats_{table}.json"))
                .save_file();
            if let Some(p) = path {
                let obj: serde_json::Map<String, serde_json::Value> = data.iter()
                    .map(|(col, s)| {
                        let v = serde_json::json!({
                            "non_null_count": s.non_null_count,
                            "null_count": s.null_count,
                            "unique_count": s.unique_count,
                            "min_value": s.min_value,
                            "max_value": s.max_value,
                            "avg_value": s.avg_value,
                            "top_values": s.top_values.iter().map(|(v,c)| serde_json::json!({"value": v, "count": c})).collect::<Vec<_>>(),
                        });
                        (col.clone(), v)
                    })
                    .collect();
                match serde_json::to_string_pretty(&obj) {
                    Ok(s) => match std::fs::write(&p, s) {
                        Ok(()) => self.toast(self.t().stats_saved.to_string()),
                        Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                    },
                    Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                }
            }
        }
    }
}
