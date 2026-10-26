use super::*;
use archipel_kit::ide::{GotItPosition, GotItTooltip};
use archipel_kit::openapi::ui::{DialogValidation, Messages, ValidationInfo};

enum GalleryDialog {
    Delete,
    NewBranch,
    YesNo,
}

pub(super) struct State {
    query: String,
    dialog: Option<GalleryDialog>,
    branch: String,
    validation: DialogValidation,
    busy: bool,
    last_action: String,
    got_it: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            query: String::new(),
            dialog: None,
            branch: "feature/islands".into(),
            validation: DialogValidation::new(),
            busy: false,
            last_action: String::new(),
            got_it: false,
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, _enabled: bool) {
        ui.add(TitledSeparator::new("Tooltips"));
        ui.horizontal(|ui| {
            ui.add(Button::secondary("Hover me"))
                .tooltip_with(HelpTooltip::new("Run 'gallery'").shortcut("Shift+F10"));
            ui.add(Button::secondary("With hint")).tooltip_with(
                HelpTooltip::new("Search Everywhere")
                    .shortcut("Double Shift")
                    .hint("Find files, actions, classes and symbols in one place."),
            );
            ui.add(Button::primary("Deploy").enabled(false))
                .tooltip("Connect a cluster first");
            ui.add(Button::secondary("With link")).tooltip_with(
                HelpTooltip::new("Hot Reload")
                    .shortcut("Ctrl+F10")
                    .hint("Applies code changes to the running application without a restart.")
                    .link("Learn more", true),
            );
        });
        ui.add(TitledSeparator::new("Got it tooltip"));
        ui.horizontal(|ui| {
            let target = ui.add(Button::secondary("Run configuration")).rect;
            if ui.add(Button::secondary("Show the tip")).clicked() {
                self.got_it = true;
            }
            if self.got_it {
                let tip =
                    GotItTooltip::new("gallery.run.config", "Pick the configuration to run here.")
                        .header("Run configurations")
                        .step(1)
                        .position(GotItPosition::TopMiddle);
                if tip.show(ui.ctx(), target, true).dismissed.is_some() {
                    self.got_it = false;
                }
            }
        });
        ui.add(TitledSeparator::new("Dialogs"));
        ui.horizontal(|ui| {
            if ui.add(Button::danger("Delete file…")).clicked() {
                self.dialog = Some(GalleryDialog::Delete);
            }
            if ui.add(Button::secondary("New branch…")).clicked() {
                self.dialog = Some(GalleryDialog::NewBranch);
            }
            if ui.add(Button::secondary("Yes / No…")).clicked() {
                self.dialog = Some(GalleryDialog::YesNo);
            }
        });
        if !self.last_action.is_empty() {
            ui.label(format!("Last dialog: {}", self.last_action));
        }
        ui.add(TitledSeparator::new("Context menu"));
        ui.add(Button::secondary("Right-click me"))
            .context_menu(|ui| {
                ui.add(ActionMenuItem::new("Rename…").shortcut("Shift+F6"));
                ui.add(ActionMenuItem::new("Copy Path").icon(Glyph::Copy));
                ui.add(ListSeparator::new());
                ui.add(
                    ActionMenuItem::new("Delete")
                        .icon(Glyph::Delete)
                        .shortcut("Delete"),
                );
            });
        ui.add(TitledSeparator::new("Cursors"));
        ui.add(JBCheckBox::new(
            &mut self.busy,
            "Busy: override every cursor below",
        ));
        cursor::override_in(ui, self.busy.then_some(egui::CursorIcon::Progress), |ui| {
            ui.horizontal(|ui| {
                ui.add(Button::secondary("Arrow"));
                ui.add(ActionLink::new("Hand"));
                ui.add(JBTextField::new(&mut self.query).placeholder("Text beam"));
            });
        });
        self.show_dialog(ui.ctx());
    }
    fn show_dialog(&mut self, ctx: &egui::Context) {
        let action = match self.dialog {
            None => return,
            Some(GalleryDialog::Delete) => DialogWrapper::new("Delete “main.rs”?")
                .icon(Glyph::Warning)
                .message("The file will be moved to the trash. You can restore it from there.")
                .primary("Delete")
                .danger(true)
                .secondary("Cancel")
                .show(ctx),
            Some(GalleryDialog::YesNo) => {
                Messages::yes_no_dialog("Delete branch?", "The branch has unmerged commits.")
                    .show(ctx)
            }
            Some(GalleryDialog::NewBranch) => {
                // Validated 300 ms after typing stops; OK is off while an error shows.
                let branch = &mut self.branch;
                self.validation.update(ctx, &*branch, || {
                    if branch.trim().is_empty() {
                        vec![ValidationInfo::error("Enter a branch name")]
                    } else if branch.contains(' ') {
                        vec![ValidationInfo::warning(
                            "Spaces in a branch name are unusual",
                        )]
                    } else {
                        vec![]
                    }
                });
                DialogWrapper::new("New Branch")
                    .content(|ui| {
                        ui.add(JBTextField::new(branch).label("Branch name"));
                    })
                    .primary("Create")
                    .secondary("Cancel")
                    .validation(&self.validation)
                    .width(360.0)
                    .show(ctx)
            }
        };
        if let Some(action) = action {
            self.last_action = format!("{action:?} (exit code {})", action.code());
            self.dialog = None;
            // The next dialog validates from scratch.
            self.validation = DialogValidation::new();
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
