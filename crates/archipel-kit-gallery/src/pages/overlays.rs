use super::*;

enum GalleryDialog {
    Delete,
    NewBranch,
}

pub(super) struct State {
    query: String,
    dialog: Option<GalleryDialog>,
    branch: String,
    busy: bool,
    last_action: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            query: String::new(),
            dialog: None,
            branch: "feature/islands".into(),
            busy: false,
            last_action: String::new(),
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, _enabled: bool) {
        ui.add(SectionHeader::new("Tooltips"));
        ui.horizontal(|ui| {
            ui.add(Button::secondary("Hover me"))
                .tooltip_with(Tooltip::new("Run 'gallery'").shortcut("Shift+F10"));
            ui.add(Button::secondary("With hint")).tooltip_with(
                Tooltip::new("Search Everywhere")
                    .shortcut("Double Shift")
                    .hint("Find files, actions, classes and symbols in one place."),
            );
            ui.add(Button::primary("Deploy").enabled(false))
                .tooltip("Connect a cluster first");
        });
        ui.add(SectionHeader::new("Dialogs"));
        ui.horizontal(|ui| {
            if ui.add(Button::danger("Delete file…")).clicked() {
                self.dialog = Some(GalleryDialog::Delete);
            }
            if ui.add(Button::secondary("New branch…")).clicked() {
                self.dialog = Some(GalleryDialog::NewBranch);
            }
        });
        if !self.last_action.is_empty() {
            ui.label(format!("Last dialog: {}", self.last_action));
        }
        ui.add(SectionHeader::new("Context menu"));
        ui.add(Button::secondary("Right-click me"))
            .context_menu(|ui| {
                ui.add(MenuItem::new("Rename…").shortcut("Shift+F6"));
                ui.add(MenuItem::new("Copy Path").icon(Glyph::Copy));
                ui.add(MenuSeparator);
                ui.add(
                    MenuItem::new("Delete")
                        .icon(Glyph::Delete)
                        .shortcut("Delete"),
                );
            });
        ui.add(SectionHeader::new("Cursors"));
        ui.add(Checkbox::new(
            &mut self.busy,
            "Busy: override every cursor below",
        ));
        cursor::override_in(ui, self.busy.then_some(egui::CursorIcon::Progress), |ui| {
            ui.horizontal(|ui| {
                ui.add(Button::secondary("Arrow"));
                ui.add(Link::new("Hand"));
                ui.add(TextField::new(&mut self.query).placeholder("Text beam"));
            });
        });
        self.show_dialog(ui.ctx());
    }
    fn show_dialog(&mut self, ctx: &egui::Context) {
        let action = match self.dialog {
            None => return,
            Some(GalleryDialog::Delete) => Dialog::new("Delete “main.rs”?")
                .icon(Glyph::Warning)
                .message("The file will be moved to the trash. You can restore it from there.")
                .primary("Delete")
                .danger(true)
                .secondary("Cancel")
                .show(ctx),
            Some(GalleryDialog::NewBranch) => {
                let valid = !self.branch.trim().is_empty();
                let branch = &mut self.branch;
                Dialog::new("New Branch")
                    .content(|ui| {
                        ui.add(TextField::new(branch).label("Branch name"));
                    })
                    .primary("Create")
                    .primary_enabled(valid)
                    .secondary("Cancel")
                    .width(360.0)
                    .show(ctx)
            }
        };
        if let Some(action) = action {
            self.last_action = format!("{action:?}");
            self.dialog = None;
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
