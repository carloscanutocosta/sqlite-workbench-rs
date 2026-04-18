use std::sync::{Arc, Mutex};
use std::thread;

use crate::db::{DatabaseManager, FilterConfig};

use super::{App, AsyncStatsResult, StatsState};

impl App {
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
                db.get_table_data(&table, limit, offset, &filter, sort_col.as_deref(), sort_asc)
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

        self.stats = StatsState { loading: true, ..Default::default() };
        let arc: AsyncStatsResult = Arc::new(Mutex::new(None));
        let arc_clone = arc.clone();

        thread::spawn(move || {
            let result = (|| {
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
            if arc.lock().unwrap().is_some() {
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
            if arc.lock().unwrap().is_some() {
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
            if arc.lock().unwrap().is_some() {
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
}
