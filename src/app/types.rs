use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::db::{ColumnStats, DatabaseManager, FilterConfig};
use crate::dialogs::{CreateTableDialog, EditRecordDialog, ErdWindow};
use crate::localization::Language;

pub const SIDEBAR_WIDTH: f32 = 210.0;
pub const SQL_KEYWORDS: &[&str] = &[
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
        Self { language: Language::Pt, theme: AppTheme::Dark, rows_per_page: 1000 }
    }
}

#[derive(Default, Clone, PartialEq)]
pub enum ActiveTab {
    #[default]
    Data,
    Schema,
    Stats,
    Sql,
}

#[derive(Default)]
pub struct StatsState {
    pub loading: bool,
    pub cancelled: bool,
    pub current_col: String,
    pub progress: f32,
    pub result: Option<Vec<(String, ColumnStats)>>,
    pub total_rows: i64,
    pub error: Option<String>,
}

pub type AsyncStatsResult = Arc<Mutex<Option<Result<(Vec<(String, ColumnStats)>, i64), String>>>>;

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
    pub(super) input_dialog: Option<InputDialog>,

    // Context menus
    pub(super) tree_ctx_row: Option<(i64, Vec<String>, egui::Pos2)>,
    pub(super) sidebar_ctx_table: Option<(String, egui::Pos2)>,

    // Status
    pub(super) toast: Option<(String, f64)>,
    pub(super) read_only: bool,
    pub(super) show_about: bool,

    // Pending async data load
    pub(super) pending_load:
        Option<Arc<Mutex<Option<Result<(Vec<String>, Vec<Vec<String>>), String>>>>>,
    pub(super) pending_page: usize,
    pub(super) pending_total: Option<Arc<Mutex<Option<Result<i64, String>>>>>,
}

pub struct InputDialog {
    pub title: String,
    pub label: String,
    pub value: String,
    pub action: InputAction,
}

#[derive(Clone)]
pub enum InputAction {
    RenameTable(String),
    ImportCsvName(String),
    NewTableName,
}
