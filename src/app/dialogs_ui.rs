use egui::{Color32, RichText, Vec2};

use super::{App, InputAction, InputDialog};
use crate::db::{ColumnDef, ForeignKeyDef};

/// Shows a floating popup at `pos`. Returns `true` when a click landed outside
/// it, unless `just_opened` says that click is the one that opened the popup.
fn show_context_area(
    ctx: &egui::Context,
    id: &str,
    pos: egui::Pos2,
    just_opened: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> bool {
    let response = egui::Area::new(egui::Id::unique(id))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, add_contents);
        })
        .response;

    response.clicked_elsewhere() && !just_opened
}

/// Blocking confirmation for an action that cannot be undone. Returns
/// `Some(true)` if confirmed, `Some(false)` if cancelled (button, Esc or a click
/// outside) and `None` while unanswered.
fn show_destructive_confirm(
    ctx: &egui::Context,
    title: &str,
    message: &str,
    warning: &str,
    confirm_label: &str,
    cancel_label: &str,
) -> Option<bool> {
    let modal = egui::Modal::new(egui::Id::unique("destructive_confirm")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(
            RichText::new(format!("⚠ {title}"))
                .size(18.0)
                .strong()
                .color(Color32::from_rgb(230, 160, 40)),
        );
        ui.add_space(8.0);
        ui.label(message);
        ui.add_space(4.0);
        ui.label(RichText::new(warning).strong().color(Color32::RED));
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            let cancel = ui.button(cancel_label);
            let confirm = ui.add(
                egui::Button::new(RichText::new(confirm_label).color(Color32::WHITE))
                    .fill(Color32::from_rgb(180, 40, 40)),
            );
            // Keep focus on the safe choice, so a stray Enter cannot confirm.
            if !cancel.has_focus() && !confirm.has_focus() {
                cancel.request_focus();
            }

            if confirm.clicked() {
                Some(true)
            } else if cancel.clicked() {
                Some(false)
            } else {
                None
            }
        })
        .inner
    });

    if modal.inner.is_none() && modal.should_close() {
        Some(false)
    } else {
        modal.inner
    }
}

impl App {
    pub(super) fn show_dialogs(&mut self, ctx: &egui::Context) {
        // ── Single-field input dialog ─────────────────────────────────────────
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
                    let resp =
                        ui.text_edit_singleline(&mut self.input_dialog.as_mut().unwrap().value);
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let value = self.input_dialog.take().unwrap().value;
                        self.execute_input_action(action, value);
                        return;
                    }
                    ui.horizontal(|ui| {
                        if ui.button(self.t().cancel).clicked() {
                            self.input_dialog = None;
                        }
                        if ui.button(self.t().save).clicked() {
                            let value = self.input_dialog.take().unwrap().value;
                            self.execute_input_action(action, value);
                        }
                    });
                });
            if !open {
                self.input_dialog = None;
            }
        }

        // ── Create table dialog ───────────────────────────────────────────────
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
                if !open {
                    self.create_table_dialog = None;
                }
            }
        }

        // ── Edit / Insert record dialog ───────────────────────────────────────
        {
            let t = self.t();
            let is_insert = self
                .edit_dialog
                .as_ref()
                .map(|d| d.rowid.is_none())
                .unwrap_or(false);
            let title = if is_insert {
                t.new_record
            } else {
                t.edit_values
            };
            let mut open = true;
            let mut action: Option<Result<Vec<(String, String)>, ()>> = None;

            if self.edit_dialog.is_some() {
                egui::Window::new(title)
                    .collapsible(false)
                    .min_width(400.0)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        if let Some(dlg) = self.edit_dialog.as_mut() {
                            action = dlg.show(ui, t);
                        }
                    });

                match action {
                    Some(Ok(pairs)) => {
                        let table = self.selected_table.clone().unwrap_or_default();
                        let rowid = self.edit_dialog.as_ref().and_then(|d| d.rowid);
                        let result = if let Some(rid) = rowid {
                            self.db.as_ref().unwrap().update_record(&table, rid, &pairs)
                        } else {
                            self.db.as_ref().unwrap().insert_record(&table, &pairs)
                        };
                        match result {
                            Ok(()) => {
                                self.toast(if rowid.is_some() {
                                    self.t().record_updated
                                } else {
                                    self.t().record_inserted
                                });
                                self.edit_dialog = None;
                                self.start_data_load(self.current_page, rowid.is_none());
                            }
                            Err(e) => self.toast(format!("{}: {e}", self.t().error)),
                        }
                    }
                    Some(Err(())) => self.edit_dialog = None,
                    None => {}
                }
                if !open {
                    self.edit_dialog = None;
                }
            }
        }

        // ── ERD window ────────────────────────────────────────────────────────
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
                if !open {
                    self.erd_window = None;
                }
            }
        }

        // ── Drop table confirmation ───────────────────────────────────────────
        if let Some(table) = self.confirm_drop_table.clone() {
            let t = self.t();
            let message = t.confirm_delete_table.replace("{}", &table);
            let answer = show_destructive_confirm(
                ctx,
                t.warning,
                &message,
                t.drop_table_warning,
                t.delete,
                t.cancel,
            );
            if answer.is_some() {
                self.confirm_drop_table = None;
            }
            if answer == Some(true) {
                self.execute_drop_table(&table);
            }
        }

        // ── Delete record confirmation ────────────────────────────────────────
        if let Some((table, rowid)) = self.confirm_delete_record.clone() {
            let t = self.t();
            let message = t.confirm_delete_record.replace("{}", &rowid.to_string());
            let answer = show_destructive_confirm(
                ctx,
                t.warning,
                &message,
                t.delete_record_warning,
                t.delete,
                t.cancel,
            );
            if answer.is_some() {
                self.confirm_delete_record = None;
            }
            if answer == Some(true) {
                self.execute_delete_record(&table, rowid);
            }
        }
    }

    fn execute_delete_record(&mut self, table: &str, rowid: i64) {
        if let Some(db) = &self.db {
            match db.delete_record(table, rowid) {
                Ok(()) => {
                    self.toast(self.t().success.to_string());
                    self.start_data_load(self.current_page, true);
                }
                Err(e) => self.toast(format!("{}: {e}", self.t().error)),
            }
        }
    }

    fn execute_drop_table(&mut self, table: &str) {
        if let Some(db) = &self.db {
            match db.drop_table(table) {
                Ok(()) => {
                    if self.selected_table.as_deref() == Some(table) {
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
    }

    pub(super) fn execute_input_action(&mut self, action: InputAction, value: String) {
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
        }
    }

    pub(super) fn show_context_menus(&mut self, ctx: &egui::Context) {
        // The click that opens a menu must not also dismiss it.
        let just_opened = std::mem::take(&mut self.ctx_menu_just_opened);

        // ── Sidebar context menu (right-click on table) ───────────────────────
        if let Some((ref table, pos)) = self.sidebar_ctx_table.clone() {
            let dismissed = show_context_area(ctx, "sidebar_ctx", pos, just_opened, |ui| {
                if ui.button(self.t().rename).clicked() {
                    self.input_dialog = Some(InputDialog {
                        title: self.t().rename.to_string(),
                        label: format!("{} {table}:", self.t().rename_table_prompt),
                        value: table.clone(),
                        action: InputAction::RenameTable(table.clone()),
                    });
                    self.sidebar_ctx_table = None;
                }
                if ui
                    .button(RichText::new(self.t().delete).color(Color32::RED))
                    .clicked()
                {
                    self.confirm_drop_table = Some(table.clone());
                    self.sidebar_ctx_table = None;
                }
            });

            if dismissed {
                self.sidebar_ctx_table = None;
            }
        }

        // ── Data row context menu (edit / delete row) ─────────────────────────
        if let Some((rowid, ref vals, pos)) = self.tree_ctx_row.clone() {
            let dismissed = show_context_area(ctx, "tree_ctx", pos, just_opened, |ui| {
                if ui.button(self.t().edit).clicked() {
                    self.open_edit_dialog(rowid, vals.clone());
                    self.tree_ctx_row = None;
                }
                if ui
                    .button(RichText::new(self.t().delete).color(Color32::RED))
                    .clicked()
                {
                    self.confirm_delete_record =
                        self.selected_table.clone().map(|table| (table, rowid));
                    self.tree_ctx_row = None;
                }
            });

            if dismissed {
                self.tree_ctx_row = None;
            }
        }
    }

    pub(super) fn show_about_window(&mut self, ctx: &egui::Context) {
        if !self.show_about { return; }

        let t = self.t();
        let mut open = self.show_about;
        let mut close_clicked = false;

        egui::Window::new(t.about_title)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(360.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.vertical_centered(|ui| {
                    if let Some(ref texture) = self.about_icon {
                        ui.add(egui::Image::new((texture.id(), egui::Vec2::new(96.0, 96.0))));
                        ui.add_space(6.0);
                    }
                    ui.label(
                        RichText::new("SQLite Workbench")
                            .size(22.0)
                            .strong()
                            .color(Color32::from_rgb(100, 160, 230)),
                    );
                    ui.label(
                        RichText::new(t.about_description)
                            .size(12.0)
                            .color(Color32::GRAY),
                    );
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                egui::Grid::new("about_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new(t.about_org).strong());
                        ui.label("NORMAXIS");
                        ui.end_row();

                        ui.label(RichText::new(t.about_author).strong());
                        ui.label("Carlos Canuto Costa");
                        ui.end_row();

                        ui.label(RichText::new(t.about_version).strong());
                        ui.label(env!("CARGO_PKG_VERSION"));
                        ui.end_row();

                        ui.label(RichText::new(t.about_license).strong());
                        ui.label("EUPL v1.2");
                        ui.end_row();

                        ui.label(RichText::new(t.about_repository).strong());
                        ui.hyperlink_to(
                            "github.com/carloscanutocosta/sqlite-workbench-rs",
                            "https://github.com/carloscanutocosta/sqlite-workbench-rs",
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(t.about_copyright)
                            .size(11.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(8.0);
                    if ui.button(t.about_close).clicked() {
                        close_clicked = true;
                    }
                });

                ui.add_space(4.0);
            });

        self.show_about = open && !close_clicked;
    }
}

#[cfg(test)]
mod tests {
    use super::{show_context_area, show_destructive_confirm};
    use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

    const MENU_POS: Pos2 = Pos2::new(200.0, 200.0);
    const INSIDE: Pos2 = Pos2::new(215.0, 215.0);
    const OUTSIDE: Pos2 = Pos2::new(600.0, 500.0);

    struct FrameResult {
        dismissed: bool,
        button_clicked: bool,
    }

    /// One frame of an app whose central panel covers the window, as in `App::ui`.
    fn run_frame(ctx: &egui::Context, events: Vec<Event>, mut content: impl FnMut(&egui::Context)) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |_| {});
            let ctx = ui.ctx().clone();
            content(&ctx);
        });
        output.textures_delta.clear(); // Nothing renders them in a test.
    }

    fn frame(ctx: &egui::Context, events: Vec<Event>, just_opened: bool) -> FrameResult {
        let mut result = FrameResult {
            dismissed: false,
            button_clicked: false,
        };
        run_frame(ctx, events, |ctx| {
            let mut clicked = false;
            let dismissed = show_context_area(ctx, "test_menu", MENU_POS, just_opened, |ui| {
                clicked = ui.button("item").clicked();
            });
            result.dismissed |= dismissed;
            result.button_clicked |= clicked;
        });
        result
    }

    fn button_event(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    /// Moves to `pos`, then presses and releases `button`; returns the release frame.
    fn click(
        ctx: &egui::Context,
        pos: Pos2,
        button: PointerButton,
        just_opened: bool,
    ) -> FrameResult {
        frame(ctx, vec![Event::PointerMoved(pos)], false);
        frame(ctx, vec![button_event(pos, button, true)], false);
        frame(ctx, vec![button_event(pos, button, false)], just_opened)
    }

    fn open_menu() -> egui::Context {
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&ctx, vec![], false);
        }
        ctx
    }

    #[test]
    fn click_outside_over_central_panel_dismisses() {
        let ctx = open_menu();
        assert!(click(&ctx, OUTSIDE, PointerButton::Primary, false).dismissed);
    }

    #[test]
    fn right_click_outside_dismisses() {
        let ctx = open_menu();
        assert!(click(&ctx, OUTSIDE, PointerButton::Secondary, false).dismissed);
    }

    #[test]
    fn opening_click_does_not_dismiss() {
        let ctx = open_menu();
        assert!(!click(&ctx, OUTSIDE, PointerButton::Secondary, true).dismissed);
    }

    #[test]
    fn click_on_menu_item_is_delivered_and_does_not_dismiss() {
        let ctx = open_menu();
        let result = click(&ctx, INSIDE, PointerButton::Primary, false);
        assert!(result.button_clicked);
        assert!(!result.dismissed);
    }

    #[test]
    fn no_click_does_not_dismiss() {
        let ctx = open_menu();
        assert!(!frame(&ctx, vec![Event::PointerMoved(OUTSIDE)], false).dismissed);
    }

    fn confirm_frame(ctx: &egui::Context, events: Vec<Event>) -> Option<bool> {
        let mut answer = None;
        run_frame(ctx, events, |ctx| {
            let shown = show_destructive_confirm(ctx, "Warning", "Drop?", "Final.", "Yes", "No");
            answer = answer.or(shown);
        });
        answer
    }

    fn open_confirm() -> egui::Context {
        let ctx = egui::Context::default();
        for _ in 0..3 {
            assert_eq!(confirm_frame(&ctx, vec![]), None);
        }
        ctx
    }

    fn press(ctx: &egui::Context, key: Key) -> Option<bool> {
        let event = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        let down = confirm_frame(ctx, vec![event(true)]);
        down.or(confirm_frame(ctx, vec![event(false)]))
    }

    #[test]
    fn confirm_stays_open_without_input() {
        let ctx = open_confirm();
        assert_eq!(confirm_frame(&ctx, vec![Event::PointerMoved(OUTSIDE)]), None);
    }

    #[test]
    fn enter_without_moving_focus_cancels() {
        let ctx = open_confirm();
        assert_eq!(press(&ctx, Key::Enter), Some(false));
    }

    #[test]
    fn escape_cancels() {
        let ctx = open_confirm();
        assert_eq!(press(&ctx, Key::Escape), Some(false));
    }

    #[test]
    fn click_outside_cancels() {
        let ctx = open_confirm();
        let pos = Pos2::new(10.0, 10.0);
        confirm_frame(&ctx, vec![Event::PointerMoved(pos)]);
        let press_event = button_event(pos, PointerButton::Primary, true);
        let release_event = button_event(pos, PointerButton::Primary, false);
        let down = confirm_frame(&ctx, vec![press_event]);
        assert_eq!(down.or(confirm_frame(&ctx, vec![release_event])), Some(false));
    }

    #[test]
    fn only_explicitly_reaching_the_confirm_button_confirms() {
        let ctx = open_confirm();
        assert_eq!(press(&ctx, Key::Tab), None);
        assert_eq!(press(&ctx, Key::Enter), Some(true));
    }
}
