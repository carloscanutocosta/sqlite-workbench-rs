use std::collections::VecDeque;

use crate::db::FilterConfig;
use crate::localization;

use super::{ActiveTab, App, AppTheme, StatsState};

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let settings_path = "settings.json".to_string();
        let settings: super::Settings = std::fs::read_to_string(&settings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let mut visuals = if settings.theme == AppTheme::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.override_text_color = None;
        cc.egui_ctx.set_visuals(visuals);
        cc.egui_ctx.set_fonts(egui::FontDefinitions::default());

        let about_icon = {
            let bytes = include_bytes!("../../Assets/Icons/icon.png");
            image::load_from_memory(bytes).ok().map(|img| {
                let rgba = img.into_rgba8();
                let (w, h) = rgba.dimensions();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &rgba,
                );
                cc.egui_ctx.load_texture("about_icon", color_image, egui::TextureOptions::LINEAR)
            })
        };

        let favorites_path = "favorites.json".to_string();
        let history_path = "history.json".to_string();

        let favorites: Vec<String> = std::fs::read_to_string(&favorites_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let history_vec: Vec<String> = std::fs::read_to_string(&history_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let rows_per_page_str = settings.rows_per_page.to_string();

        Self {
            settings,
            settings_path,
            favorites_path,
            history_path,
            db: None,
            db_path: String::new(),
            tables: vec![],
            table_search: String::new(),
            selected_table: None,
            filter: FilterConfig { column: String::new(), operator: "LIKE".into(), value: String::new() },
            filter_col_options: vec![],
            current_page: 1,
            total_pages: 1,
            total_rows: 0,
            rows_per_page_str,
            columns: vec![],
            rows: vec![],
            sort_col: None,
            sort_asc: true,
            loading_data: false,
            schema_text: String::new(),
            stats: StatsState::default(),
            stats_result_chan: None,
            stats_export_data: None,
            stats_total_rows: 0,
            sql_input: "-- SQL Query\nSELECT * FROM sqlite_master;".into(),
            sql_columns: vec![],
            sql_rows: vec![],
            sql_error: None,
            sql_autocomplete: Self::base_keywords(),
            history: VecDeque::from(history_vec),
            favorites,
            active_tab: ActiveTab::default(),
            create_table_dialog: None,
            edit_dialog: None,
            erd_window: None,
            input_dialog: None,
            tree_ctx_row: None,
            sidebar_ctx_table: None,
            toast: None,
            show_about: false,
            about_icon,
            pending_load: None,
            pending_page: 1,
            pending_total: None,
        }
    }

    pub(super) fn base_keywords() -> Vec<String> {
        super::SQL_KEYWORDS.iter().map(|s| s.to_string()).collect()
    }

    pub(super) fn t(&self) -> &'static localization::T {
        localization::get(&self.settings.language)
    }

    pub(super) fn save_settings(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.settings) {
            let _ = std::fs::write(&self.settings_path, s);
        }
    }

    pub(super) fn save_favorites(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.favorites) {
            let _ = std::fs::write(&self.favorites_path, s);
        }
    }

    pub(super) fn save_history(&self) {
        let v: Vec<&String> = self.history.iter().collect();
        if let Ok(s) = serde_json::to_string_pretty(&v) {
            let _ = std::fs::write(&self.history_path, s);
        }
    }

    pub(super) fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 3.0));
    }

    pub(super) fn refresh_tables(&mut self) {
        if let Some(db) = &self.db {
            self.tables = db.get_tables().unwrap_or_default();
            self.update_autocomplete();
        }
    }

    pub(super) fn update_autocomplete(&mut self) {
        let mut list = Self::base_keywords();
        if let Some(db) = &self.db {
            for t in &self.tables {
                list.push(t.clone());
                if let Ok(cols) = db.get_columns(t) {
                    list.extend(cols);
                }
            }
        }
        list.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        list.dedup();
        self.sql_autocomplete = list;
    }

    pub(super) fn load_table(&mut self, name: &str) {
        let name = name.to_string();
        self.selected_table = Some(name.clone());
        self.current_page = 1;
        self.sort_col = None;
        self.sort_asc = true;
        self.filter = FilterConfig { column: String::new(), operator: "LIKE".into(), value: String::new() };

        if let Some(db) = &self.db {
            self.schema_text = db.get_table_schema(&name);
            let cols = db.get_columns(&name).unwrap_or_default();
            self.filter_col_options = std::iter::once(self.t().all_columns.to_string())
                .chain(cols)
                .collect();
        }

        self.active_tab = ActiveTab::Data;
        self.start_data_load(1, true);
        self.start_stats_load();
    }

    pub(super) fn push_history(&mut self, query: &str) {
        let q = query.to_string();
        if self.history.front().map(|h| h != &q).unwrap_or(true) {
            self.history.push_front(q);
            if self.history.len() > 20 { self.history.pop_back(); }
            self.save_history();
        }
    }
}
