use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

use egui::{Color32, RichText};
use serde::{Deserialize, Serialize};

use crate::db::{ColumnStats, DatabaseManager, FilterConfig};
use crate::dialogs::{CreateTableDialog, EditRecordDialog, ErdWindow};
use crate::localization::{self, Language};

mod actions;
mod dialogs_ui;
mod sidebar;
mod tabs;

pub(super) const SIDEBAR_WIDTH: f32 = 210.0;
pub(super) const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE",
    "TABLE", "DROP", "ALTER", "INDEX", "VIEW", "TRIGGER", "AND", "OR", "NOT", "NULL", "IS", "IN",
    "BETWEEN", "LIKE", "LIMIT", "OFFSET", "ORDER", "BY", "GROUP", "HAVING", "JOIN", "ON", "AS",
    "DISTINCT", "CASE", "WHEN", "THEN", "ELSE", "END", "PRAGMA",
];

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub language: Language,
    pub theme: AppTheme,
    pub rows_per_page: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::Pt,
            theme: AppTheme::Dark,
            rows_per_page: 1000,
        }
    }
}

#[derive(Default, Clone, PartialEq)]
pub(super) enum ActiveTab {
    #[default]
    Data,
    Schema,
    Stats,
    Sql,
}

#[derive(Default)]
pub(super) struct StatsState {
    pub loading: bool,
    pub cancelled: bool,
    pub current_col: String,
    pub progress: f32,
    pub result: Option<Vec<(String, ColumnStats)>>,
    pub total_rows: i64,
    pub error: Option<String>,
}

type AsyncStatsResult = Arc<Mutex<Option<Result<(Vec<(String, ColumnStats)>, i64), String>>>>;

pub struct App {
    pub settings: Settings,
    pub(super) settings_path: String,
    pub(super) favorites_path: String,
    pub(super) history_path: String,

    pub(super) db: Option<DatabaseManager>,
    pub(super) db_path: String,

    pub(super) tables: Vec<String>,
    pub(super) table_search: String,
    pub(super) selected_table: Option<String>,

    // Data tab
    pub(super) filter: FilterConfig,
    pub(super) filter_col_options: Vec<String>,
    pub(super) current_page: usize,
    pub(super) total_pages: usize,
    pub(super) total_rows: i64,
    pub(super) rows_per_page_str: String,
    pub(super) columns: Vec<String>,
    pub(super) rows: Vec<Vec<String>>,
    pub(super) sort_col: Option<String>,
    pub(super) sort_asc: bool,
    pub(super) loading_data: bool,

    // Schema tab
    pub(super) schema_text: String,

    // Stats tab
    pub(super) stats: StatsState,
    pub(super) stats_result_chan: Option<AsyncStatsResult>,
    pub(super) stats_export_data: Option<Vec<(String, ColumnStats)>>,
    pub(super) stats_total_rows: i64,

    // SQL tab
    pub(super) sql_input: String,
    pub(super) sql_columns: Vec<String>,
    pub(super) sql_rows: Vec<Vec<String>>,
    pub(super) sql_error: Option<String>,
    pub(super) sql_autocomplete: Vec<String>,
    pub(super) sql_ac_prefix: String,
    pub(super) sql_ac_show: bool,
    pub(super) sql_ac_selected: usize,
    pub(super) history: VecDeque<String>,
    pub(super) favorites: Vec<String>,

    pub(super) active_tab: ActiveTab,

    // Dialogs
    pub(super) create_table_dialog: Option<CreateTableDialog>,
    pub(super) edit_dialog: Option<EditRecordDialog>,
    pub(super) erd_window: Option<ErdWindow>,

    // Inline input dialogs
    pub(super) input_dialog: Option<InputDialog>,

    // Context menus
    pub(super) tree_ctx_row: Option<(i64, Vec<String>, egui::Pos2)>,
    pub(super) sidebar_ctx_table: Option<(String, egui::Pos2)>,

    // Toast / status messages
    pub(super) toast: Option<(String, f64)>,
    pub(super) read_only: bool,

    // Pending async data load
    pub(super) pending_load:
        Option<Arc<Mutex<Option<Result<(Vec<String>, Vec<Vec<String>>), String>>>>>,
    pub(super) pending_page: usize,
    pub(super) pending_total: Option<Arc<Mutex<Option<Result<i64, String>>>>>,
}

pub(super) struct InputDialog {
    pub title: String,
    pub label: String,
    pub value: String,
    pub action: InputAction,
}

#[derive(Clone)]
pub(super) enum InputAction {
    RenameTable(String),
    ImportCsvName(String),
    NewTableName,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let settings_path = "settings.json".to_string();
        let settings: Settings = std::fs::read_to_string(&settings_path)
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

        let fonts = egui::FontDefinitions::default();
        cc.egui_ctx.set_fonts(fonts);

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
            filter: FilterConfig {
                column: String::new(),
                operator: "LIKE".into(),
                value: String::new(),
            },
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
            sql_ac_prefix: String::new(),
            sql_ac_show: false,
            sql_ac_selected: 0,
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
            read_only: false,
            pending_load: None,
            pending_page: 1,
            pending_total: None,
        }
    }

    pub(super) fn base_keywords() -> Vec<String> {
        SQL_KEYWORDS.iter().map(|s| s.to_string()).collect()
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
        self.filter = FilterConfig {
            column: String::new(),
            operator: "LIKE".into(),
            value: String::new(),
        };

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

    pub(super) fn start_data_load(&mut self, page: usize, recalc_total: bool) {
        let Some(db_path) = self.db.as_ref().map(|db| db.path.clone()) else {
            return;
        };
        let table = match &self.selected_table {
            Some(t) => t.clone(),
            None => return,
        };

        self.loading_data = true;
        self.pending_page = page;

        let filter = self.filter.clone();
        let limit = self.settings.rows_per_page as i64;
        let offset = ((page - 1) * self.settings.rows_per_page) as i64;
        let sort_col = self.sort_col.clone();
        let sort_asc = self.sort_asc;

        if recalc_total {
            let arc_total: Arc<Mutex<Option<Result<i64, String>>>> = Arc::new(Mutex::new(None));
            let arc_clone = arc_total.clone();
            let db_path2 = db_path.clone();
            let table2 = table.clone();
            let filter2 = filter.clone();
            thread::spawn(move || {
                let result = DatabaseManager::open(&db_path2)
                    .and_then(|db| db.get_row_count(&table2, &filter2));
                *arc_clone.lock().unwrap() = Some(result);
            });
            self.pending_total = Some(arc_total);
        }

        let arc_data: Arc<Mutex<Option<Result<(Vec<String>, Vec<Vec<String>>), String>>>> =
            Arc::new(Mutex::new(None));
        let arc_clone = arc_data.clone();
        thread::spawn(move || {
            let result = DatabaseManager::open(&db_path).and_then(|db| {
                db.get_table_data(
                    &table,
                    limit,
                    offset,
                    &filter,
                    sort_col.as_deref(),
                    sort_asc,
                )
            });
            *arc_clone.lock().unwrap() = Some(result);
        });
        self.pending_load = Some(arc_data);
    }

    pub(super) fn start_stats_load(&mut self) {
        let Some(db_path) = self.db.as_ref().map(|db| db.path.clone()) else {
            return;
        };
        let table = match &self.selected_table {
            Some(t) => t.clone(),
            None => return,
        };

        self.stats = StatsState {
            loading: true,
            ..Default::default()
        };
        let arc: AsyncStatsResult = Arc::new(Mutex::new(None));
        let arc_clone = arc.clone();

        thread::spawn(move || {
            let result: Result<(Vec<(String, ColumnStats)>, i64), String> = (|| {
                let db = DatabaseManager::open(&db_path)?;
                let cols = db.get_columns(&table)?;
                let total = db.get_row_count(&table, &FilterConfig::default())?;
                let mut all = Vec::new();
                for col in &cols {
                    let stats = db.get_column_stats(&table, col)?;
                    all.push((col.clone(), stats));
                }
                Ok((all, total))
            })();
            *arc_clone.lock().unwrap() = Some(result);
        });
        self.stats_result_chan = Some(arc);
    }

    pub(super) fn poll_async(&mut self) {
        if let Some(arc) = self.pending_load.take() {
            let ready = arc.lock().unwrap().is_some();
            if ready {
                let result = arc.lock().unwrap().take().unwrap();
                self.loading_data = false;
                match result {
                    Ok((cols, rows)) => {
                        self.columns = cols;
                        self.rows = rows;
                        self.current_page = self.pending_page;
                    }
                    Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                }
            } else {
                self.pending_load = Some(arc);
            }
        }

        if let Some(arc) = self.pending_total.take() {
            let ready = arc.lock().unwrap().is_some();
            if ready {
                let result = arc.lock().unwrap().take().unwrap();
                match result {
                    Ok(n) => {
                        self.total_rows = n;
                        let rpp = self.settings.rows_per_page;
                        self.total_pages = ((n as usize + rpp - 1) / rpp).max(1);
                    }
                    Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                }
            } else {
                self.pending_total = Some(arc);
            }
        }

        if let Some(arc) = self.stats_result_chan.take() {
            let ready = arc.lock().unwrap().is_some();
            if ready {
                let result = arc.lock().unwrap().take().unwrap();
                self.stats.loading = false;
                match result {
                    Ok((data, total)) => {
                        self.stats.total_rows = total;
                        self.stats.result = Some(data.clone());
                        self.stats_export_data = Some(data);
                        self.stats_total_rows = total;
                    }
                    Err(e) => self.stats.error = Some(e),
                }
            } else {
                self.stats_result_chan = Some(arc);
            }
        }
    }

    pub(super) fn push_history(&mut self, query: &str) {
        let q = query.to_string();
        if self.history.front().map(|h| h != &q).unwrap_or(true) {
            self.history.push_front(q);
            if self.history.len() > 20 {
                self.history.pop_back();
            }
            self.save_history();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        self.poll_async();

        if let Some((_, ref mut t)) = self.toast {
            *t -= ctx.input(|i| i.stable_dt) as f64;
            if *t <= 0.0 {
                self.toast = None;
            }
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(self.t().menu_file, |ui| {
                    if ui.button(self.t().load_db).clicked() {
                        self.open_db_file();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.db.is_some(), egui::Button::new(self.t().import_csv))
                        .clicked()
                    {
                        self.open_import_csv();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.db.is_some(), egui::Button::new(self.t().compact_db))
                        .clicked()
                    {
                        self.do_vacuum(ctx);
                        ui.close_menu();
                    }
                });
                ui.menu_button(self.t().menu_table, |ui| {
                    if ui
                        .add_enabled(self.db.is_some(), egui::Button::new(self.t().new_table))
                        .clicked()
                    {
                        self.open_create_table_dialog();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.db.is_some(), egui::Button::new(self.t().erd_view))
                        .clicked()
                    {
                        self.open_erd();
                        ui.close_menu();
                    }
                });
                ui.menu_button(self.t().menu_settings, |ui| {
                    let t = self.t();
                    ui.label(t.language_label);
                    if ui
                        .selectable_label(self.settings.language == Language::Pt, "Português")
                        .clicked()
                    {
                        self.settings.language = Language::Pt;
                        self.save_settings();
                    }
                    if ui
                        .selectable_label(self.settings.language == Language::En, "English")
                        .clicked()
                    {
                        self.settings.language = Language::En;
                        self.save_settings();
                    }
                    ui.separator();
                    ui.label(t.theme);
                    if ui
                        .selectable_label(self.settings.theme == AppTheme::Dark, "Dark")
                        .clicked()
                    {
                        self.settings.theme = AppTheme::Dark;
                        ctx.set_visuals(egui::Visuals::dark());
                        self.save_settings();
                    }
                    if ui
                        .selectable_label(self.settings.theme == AppTheme::Light, "Light")
                        .clicked()
                    {
                        self.settings.theme = AppTheme::Light;
                        ctx.set_visuals(egui::Visuals::light());
                        self.save_settings();
                    }
                    ui.separator();
                    ui.label(t.rows_per_page);
                    for &n in &[100usize, 500, 1000, 5000] {
                        if ui
                            .selectable_label(self.settings.rows_per_page == n, n.to_string())
                            .clicked()
                        {
                            self.settings.rows_per_page = n;
                            self.rows_per_page_str = n.to_string();
                            self.save_settings();
                            if self.selected_table.is_some() {
                                let table = self.selected_table.clone().unwrap();
                                self.load_table(&table.clone());
                            }
                        }
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(self.t().footer_author)
                        .color(Color32::GRAY)
                        .small(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some((ref msg, _)) = self.toast {
                        ui.label(RichText::new(msg).color(Color32::GOLD));
                    }
                });
            });
        });

        egui::SidePanel::left("sidebar")
            .exact_width(SIDEBAR_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                self.show_sidebar(ui, ctx);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_main(ui, ctx);
        });

        self.show_dialogs(ctx);
        self.show_context_menus(ctx);
    }
}
