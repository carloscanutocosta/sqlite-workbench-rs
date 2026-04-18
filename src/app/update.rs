use egui::{Color32, RichText};

use crate::localization::Language;

use super::{App, AppTheme, SIDEBAR_WIDTH};

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
                    if ui.button(self.t().new_db).clicked() {
                        self.create_new_db();
                        ui.close_menu();
                    }
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
                if ui.button(self.t().menu_about).clicked() {
                    self.show_about = true;
                }
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
        self.show_about_window(ctx);
    }
}
