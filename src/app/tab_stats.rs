use egui::{Color32, RichText, ScrollArea, Ui};

use super::App;

impl App {
    pub(super) fn show_stats_tab(&mut self, ui: &mut Ui) {
        if self.stats.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!(
                    "{} {}",
                    self.t().calculating_stats,
                    self.stats.current_col
                ));
            });
            if ui.button(self.t().cancel).clicked() {
                self.stats.cancelled = true;
            }
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
                ui.label(
                    RichText::new(format!(
                        "{} {table}  —  {total} {}",
                        self.t().stats_table,
                        self.t().lines
                    ))
                    .strong(),
                );
                ui.separator();
                for (col, s) in data {
                    ui.collapsing(format!("[{col}]"), |ui| {
                        ui.label(format!("{} {}", self.t().stats_filled, s.non_null_count));
                        ui.label(format!("{} {}", self.t().stats_empty, s.null_count));
                        ui.label(format!("{} {}", self.t().stats_unique, s.unique_count));
                        if let Some(ref v) = s.min_value {
                            ui.label(format!("{} {v}", self.t().stats_min));
                        }
                        if let Some(ref v) = s.max_value {
                            ui.label(format!("{} {v}", self.t().stats_max));
                        }
                        if let Some(a) = s.avg_value {
                            ui.label(format!("{} {:.4}", self.t().stats_avg, a));
                        }
                        if !s.top_values.is_empty() {
                            ui.label(self.t().stats_top_values);
                            for (val, count) in s.top_values.iter().take(5) {
                                let pct = if total > 0 {
                                    *count as f64 / total as f64 * 100.0
                                } else {
                                    0.0
                                };
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
}
