use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

use egui::{Color32, FontId, RichText, ScrollArea, Ui, Vec2};
use egui_extras;
use serde::{Deserialize, Serialize};

use crate::db::{ColumnDef, ColumnStats, DatabaseManager, FilterConfig, ForeignKeyDef};
use crate::dialogs::{CreateTableDialog, EditRecordDialog, ErdWindow};
use crate::localization::{self, Language};

const SIDEBAR_WIDTH: f32 = 210.0;
const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET",
    "DELETE", "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "VIEW", "TRIGGER",
    "AND", "OR", "NOT", "NULL", "IS", "IN", "BETWEEN", "LIKE", "LIMIT",
    "OFFSET", "ORDER", "BY", "GROUP", "HAVING", "JOIN", "ON", "AS",
    "DISTINCT", "CASE", "WHEN", "THEN", "ELSE", "END", "PRAGMA",
];

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub language: Language,
    pub theme: AppTheme,
    pub rows_per_page: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub enum AppTheme { #[default] Dark, Light }

impl Default for Settings {
    fn default() -> Self {
        Self { language: Language::Pt, theme: AppTheme::Dark, rows_per_page: 1000 }
    }
}

#[derive(Default, Clone, PartialEq)]
pub enum ActiveTab { #[default] Data, Schema, Stats, Sql }

#[derive(Default)]
struct StatsState {
    loading: bool,
    cancelled: bool,
    current_col: String,
    progress: f32,
    result: Option<Vec<(String, ColumnStats)>>,
    total_rows: i64,
    error: Option<String>,
}

type AsyncStatsResult = Arc<Mutex<Option<Result<(Vec<(String, ColumnStats)>, i64), String>>>>;

pub struct App {
    pub settings: Settings,
    settings_path: String,
    favorites_path: String,
    history_path: String,

    db: Option<DatabaseManager>,
    db_path: String,

    tables: Vec<String>,
    table_search: String,
    selected_table: Option<String>,

    // Data tab
    filter: FilterConfig,
    filter_col_options: Vec<String>,
    current_page: usize,
    total_pages: usize,
    total_rows: i64,
    rows_per_page_str: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    sort_col: Option<String>,
    sort_asc: bool,
    loading_data: bool,

    // Schema tab
    schema_text: String,

    // Stats tab
    stats: StatsState,
    stats_result_chan: Option<AsyncStatsResult>,
    stats_export_data: Option<Vec<(String, ColumnStats)>>,
    stats_total_rows: i64,

    // SQL tab
    sql_input: String,
    sql_columns: Vec<String>,
    sql_rows: Vec<Vec<String>>,
    sql_error: Option<String>,
    sql_autocomplete: Vec<String>,
    sql_ac_prefix: String,
    sql_ac_show: bool,
    sql_ac_selected: usize,
    history: VecDeque<String>,
    favorites: Vec<String>,

    active_tab: ActiveTab,

    // Dialogs
    create_table_dialog: Option<CreateTableDialog>,
    edit_dialog: Option<EditRecordDialog>,
    erd_window: Option<ErdWindow>,

    // Inline input dialogs (simple single-field)
    input_dialog: Option<InputDialog>,

    // Context menus
    tree_ctx_row: Option<(i64, Vec<String>, egui::Pos2)>,
    sidebar_ctx_table: Option<(String, egui::Pos2)>,

    // Toast / status messages
    toast: Option<(String, f64)>,
    read_only: bool,

    // Pending async data load
    pending_load: Option<Arc<Mutex<Option<Result<(Vec<String>, Vec<Vec<String>>), String>>>>>,
    pending_page: usize,
    pending_total: Option<Arc<Mutex<Option<Result<i64, String>>>>>,
}

struct InputDialog {
    pub title: String,
    pub label: String,
    pub value: String,
    pub action: InputAction,
}

#[derive(Clone)]
enum InputAction {
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

        // Apply visual style
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

    fn base_keywords() -> Vec<String> {
        SQL_KEYWORDS.iter().map(|s| s.to_string()).collect()
    }

    fn t(&self) -> &'static localization::T {
        localization::get(&self.settings.language)
    }

    fn save_settings(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.settings) {
            let _ = std::fs::write(&self.settings_path, s);
        }
    }

    fn save_favorites(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.favorites) {
            let _ = std::fs::write(&self.favorites_path, s);
        }
    }

    fn save_history(&self) {
        let v: Vec<&String> = self.history.iter().collect();
        if let Ok(s) = serde_json::to_string_pretty(&v) {
            let _ = std::fs::write(&self.history_path, s);
        }
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 3.0));
    }

    fn refresh_tables(&mut self) {
        if let Some(db) = &self.db {
            self.tables = db.get_tables().unwrap_or_default();
            self.update_autocomplete();
        }
    }

    fn update_autocomplete(&mut self) {
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

    fn load_table(&mut self, name: &str) {
        let name = name.to_string();
        self.selected_table = Some(name.clone());
        self.current_page = 1;
        self.sort_col = None;
        self.sort_asc = true;
        self.filter = FilterConfig { column: String::new(), operator: "LIKE".into(), value: String::new() };

        if let Some(db) = &self.db {
            // Schema
            self.schema_text = db.get_table_schema(&name);
            // Column options for filter
            let cols = db.get_columns(&name).unwrap_or_default();
            self.filter_col_options = std::iter::once(self.t().all_columns.to_string())
                .chain(cols)
                .collect();
        }

        self.active_tab = ActiveTab::Data;
        self.start_data_load(1, true);
        self.start_stats_load();
    }

    fn start_data_load(&mut self, page: usize, recalc_total: bool) {
        let Some(db_path) = self.db.as_ref().map(|db| db.path.clone()) else { return };
        let table = match &self.selected_table { Some(t) => t.clone(), None => return };

        self.loading_data = true;
        self.pending_page = page;

        let filter = self.filter.clone();
        let limit = self.settings.rows_per_page as i64;
        let offset = ((page - 1) * self.settings.rows_per_page) as i64;
        let sort_col = self.sort_col.clone();
        let sort_asc = self.sort_asc;

        // Total row count
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

        let arc_data: Arc<Mutex<Option<Result<(Vec<String>, Vec<Vec<String>>), String>>>> = Arc::new(Mutex::new(None));
        let arc_clone = arc_data.clone();
        thread::spawn(move || {
            let result = DatabaseManager::open(&db_path)
                .and_then(|db| db.get_table_data(&table, limit, offset, &filter, sort_col.as_deref(), sort_asc));
            *arc_clone.lock().unwrap() = Some(result);
        });
        self.pending_load = Some(arc_data);
    }

    fn start_stats_load(&mut self) {
        let Some(db_path) = self.db.as_ref().map(|db| db.path.clone()) else { return };
        let table = match &self.selected_table { Some(t) => t.clone(), None => return };

        self.stats = StatsState { loading: true, ..Default::default() };
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

    fn poll_async(&mut self) {
        // Poll data load
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

        // Poll total count
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

        // Poll stats
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

    fn push_history(&mut self, query: &str) {
        let q = query.to_string();
        if self.history.front().map(|h| h != &q).unwrap_or(true) {
            self.history.push_front(q);
            if self.history.len() > 20 { self.history.pop_back(); }
            self.save_history();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint(); // keep polling async tasks
        self.poll_async();

        // Decay toast
        if let Some((_, ref mut t)) = self.toast {
            *t -= ctx.input(|i| i.stable_dt) as f64;
            if *t <= 0.0 { self.toast = None; }
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(self.t().load_db).clicked() {
                        self.open_db_file();
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.db.is_some(), egui::Button::new(self.t().import_csv)).clicked() {
                        self.open_import_csv();
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.db.is_some(), egui::Button::new(self.t().compact_db)).clicked() {
                        self.do_vacuum(ctx);
                        ui.close_menu();
                    }
                });
                ui.menu_button("Table", |ui| {
                    if ui.add_enabled(self.db.is_some(), egui::Button::new(self.t().new_table)).clicked() {
                        self.open_create_table_dialog();
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.db.is_some(), egui::Button::new(self.t().erd_view)).clicked() {
                        self.open_erd();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Settings", |ui| {
                    let t = self.t();
                    ui.label(t.language_label);
                    if ui.selectable_label(self.settings.language == Language::Pt, "Português").clicked() {
                        self.settings.language = Language::Pt;
                        self.save_settings();
                    }
                    if ui.selectable_label(self.settings.language == Language::En, "English").clicked() {
                        self.settings.language = Language::En;
                        self.save_settings();
                    }
                    ui.separator();
                    ui.label(t.theme);
                    if ui.selectable_label(self.settings.theme == AppTheme::Dark, "Dark").clicked() {
                        self.settings.theme = AppTheme::Dark;
                        ctx.set_visuals(egui::Visuals::dark());
                        self.save_settings();
                    }
                    if ui.selectable_label(self.settings.theme == AppTheme::Light, "Light").clicked() {
                        self.settings.theme = AppTheme::Light;
                        ctx.set_visuals(egui::Visuals::light());
                        self.save_settings();
                    }
                    ui.separator();
                    ui.label(t.rows_per_page);
                    for &n in &[100usize, 500, 1000, 5000] {
                        if ui.selectable_label(self.settings.rows_per_page == n, n.to_string()).clicked() {
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

        // Footer
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(self.t().footer_author).color(Color32::GRAY).small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some((ref msg, _)) = self.toast {
                        ui.label(RichText::new(msg).color(Color32::GOLD));
                    }
                });
            });
        });

        // Sidebar
        egui::SidePanel::left("sidebar")
            .exact_width(SIDEBAR_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                self.show_sidebar(ui, ctx);
            });

        // Main panel
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_main(ui, ctx);
        });

        // Dialogs (after panels so they layer on top)
        self.show_dialogs(ctx);

        // Context menus
        self.show_context_menus(ctx);
    }
}

impl App {
    fn show_sidebar(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.add_space(8.0);
        ui.heading(self.t().db_explorer);
        ui.separator();

        if ui.button(self.t().load_db).clicked() {
            self.open_db_file();
        }

        let db_loaded = self.db.is_some();

        ui.add_enabled_ui(db_loaded, |ui| {
            if ui.button(self.t().import_csv).clicked() { self.open_import_csv(); }
            if ui.button(self.t().new_table).clicked() { self.open_create_table_dialog(); }
            if ui.button(self.t().compact_db).clicked() { self.do_vacuum(ctx); }
            if ui.button(self.t().erd_view).clicked() { self.open_erd(); }
        });

        ui.separator();

        // DB path info
        if !self.db_path.is_empty() {
            let filename = std::path::Path::new(&self.db_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            ui.label(RichText::new(format!("{} {filename}", self.t().file_loaded)).small().color(Color32::GRAY));
        }

        ui.add_space(4.0);
        ui.label(self.t().tables);
        let hint = self.t().search_placeholder;
        ui.add_enabled(db_loaded, egui::TextEdit::singleline(&mut self.table_search)
            .hint_text(hint)
            .desired_width(f32::INFINITY));

        ui.separator();

        let search_lower = self.table_search.to_lowercase();
        ScrollArea::vertical().id_salt("table_list").show(ui, |ui| {
            let tables: Vec<String> = self.tables.iter()
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

            if let Some(t) = to_load { self.load_table(&t); }
            if let Some(t) = ctx_table { self.sidebar_ctx_table = Some(t); }
        });
    }

    fn show_main(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if self.db.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(self.t().no_file).size(18.0).color(Color32::GRAY));
            });
            return;
        }

        // Header bar
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

        // Tabs
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
    fn show_data_tab(&mut self, ui: &mut Ui) {
        // Filter bar
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

        // Table
        let available = ui.available_height() - 30.0;
        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ScrollArea::both().max_height(available).show(ui, |ui| {
                self.show_data_table(ui);
            });
        });

        // Pagination
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
        use egui_extras::{Column, TableBuilder};

        let col_count = self.columns.len();
        if col_count == 0 {
            ui.label(RichText::new(self.t().no_file).color(Color32::GRAY));
            return;
        }

        // Skip rowid (index 0) in display
        let display_cols: Vec<&String> = self.columns.iter().skip(1).collect();
        let display_count = display_cols.len();

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(60.0)); // actions column

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
    fn show_schema_tab(&mut self, ui: &mut Ui) {
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
    fn show_stats_tab(&mut self, ui: &mut Ui) {
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
    fn show_sql_tab(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        let available_height = ui.available_height();
        let editor_height = available_height * 0.35;
        let result_height = available_height * 0.45;

        ui.columns(2, |cols| {
            // Left: editor + results
            cols[0].vertical(|ui| {
                // SQL Editor
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

                        // Ctrl+Enter to run
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

                // SQL results
                ScrollArea::both()
                    .id_salt("sql_results_scroll")
                    .max_height(result_height)
                    .show(ui, |ui| {
                        self.show_sql_results(ui);
                    });
            });

            // Right: history + favorites
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
        use egui_extras::{Column, TableBuilder};

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

    // ── Dialogs ──────────────────────────────────────────────────────────────
    fn show_dialogs(&mut self, ctx: &egui::Context) {
        // Input dialog (single text field)
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
                    let resp = ui.text_edit_singleline(&mut self.input_dialog.as_mut().unwrap().value);
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let value = self.input_dialog.take().unwrap().value;
                        self.execute_input_action(action, value);
                        return;
                    }
                    ui.horizontal(|ui| {
                        if ui.button(self.t().cancel).clicked() { self.input_dialog = None; }
                        if ui.button(self.t().save).clicked() {
                            let value = self.input_dialog.take().unwrap().value;
                            self.execute_input_action(action, value);
                        }
                    });
                });
            if !open { self.input_dialog = None; }
        }

        // Create table dialog
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
                if !open { self.create_table_dialog = None; }
            }
        }

        // Edit / Insert record dialog
        {
            let t = self.t();
            let is_insert = self.edit_dialog.as_ref().map(|d| d.rowid.is_none()).unwrap_or(false);
            let title = if is_insert { t.new_record } else { t.edit_values };
            let mut open = true;
            let mut do_save: Option<Vec<(String, String)>> = None;

            if self.edit_dialog.is_some() {
                egui::Window::new(title)
                    .collapsible(false)
                    .min_width(400.0)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        if let Some(dlg) = self.edit_dialog.as_mut() {
                            do_save = dlg.show(ui, t);
                        }
                    });

                if let Some(pairs) = do_save {
                    let table = self.selected_table.clone().unwrap_or_default();
                    let rowid = self.edit_dialog.as_ref().and_then(|d| d.rowid);
                    let result = if let Some(rid) = rowid {
                        self.db.as_ref().unwrap().update_record(&table, rid, &pairs)
                    } else {
                        self.db.as_ref().unwrap().insert_record(&table, &pairs)
                    };
                    match result {
                        Ok(()) => {
                            self.toast(if rowid.is_some() { self.t().record_updated } else { self.t().record_inserted });
                            self.edit_dialog = None;
                            self.start_data_load(self.current_page, rowid.is_none());
                        }
                        Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                    }
                }
                if !open { self.edit_dialog = None; }
            }
        }

        // ERD window
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
                if !open { self.erd_window = None; }
            }
        }
    }

    fn execute_input_action(&mut self, action: InputAction, value: String) {
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

    fn show_context_menus(&mut self, ctx: &egui::Context) {
        // Sidebar context menu (right-click on table in list)
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
                        if ui.button(RichText::new(self.t().delete).color(Color32::RED)).clicked() {
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
                if released_outside { self.sidebar_ctx_table = None; }
            }
        }

        // Data table context menu (edit / delete row)
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
                        if ui.button(RichText::new(self.t().delete).color(Color32::RED)).clicked() {
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

    // ── Actions ──────────────────────────────────────────────────────────────
    fn open_db_file(&mut self) {
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

    fn open_import_csv(&mut self) {
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

    fn do_vacuum(&mut self, _ctx: &egui::Context) {
        if let Some(db) = &self.db {
            match db.vacuum() {
                Ok(()) => self.toast(self.t().vacuum_success.to_string()),
                Err(e) => self.toast(format!("{}: {e}", self.t().error)),
            }
        }
    }

    fn open_create_table_dialog(&mut self) {
        self.create_table_dialog = Some(CreateTableDialog::new());
    }

    fn open_erd(&mut self) {
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

    fn open_insert_dialog(&mut self) {
        let cols: Vec<String> = self.columns.iter().skip(1).cloned().collect(); // skip rowid
        self.edit_dialog = Some(EditRecordDialog::for_insert(cols));
    }

    fn open_edit_dialog(&mut self, rowid: i64, vals: Vec<String>) {
        let cols: Vec<String> = self.columns.iter().skip(1).cloned().collect();
        let values: Vec<String> = vals.into_iter().skip(1).collect();
        self.edit_dialog = Some(EditRecordDialog::for_edit(rowid, cols, values));
    }

    fn run_query(&mut self) {
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
                // Refresh table list if DDL
                let up = query.to_uppercase();
                if up.starts_with("CREATE") || up.starts_with("DROP") || up.starts_with("ALTER") {
                    self.refresh_tables();
                }
            }
            Err(e) => self.sql_error = Some(e),
        }
    }

    fn export_data(&mut self) {
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
            // Skip rowid column (index 0) for data tab
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

    fn export_stats_json(&mut self, table: &str) {
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
