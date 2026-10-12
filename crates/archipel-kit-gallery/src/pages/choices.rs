use super::*;

pub(super) struct State {
    checks: [bool; 3],
    indent: usize,
    toggles: [bool; 2],
}

impl Default for State {
    fn default() -> Self {
        Self {
            checks: [true, false, false],
            indent: 1,
            toggles: [true, false],
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        // A settings page: the form owns the spacing, groups and indents.
        Form::new().show(ui, |form| {
            form.group("Checkboxes", |form| {
                let [a, b, c] = &mut self.checks;
                let all = *a && *b && *c;
                let some = *a || *b || *c;
                let mut parent = all;
                if form
                    .add(
                        Checkbox::new(&mut parent, "All inspections")
                            .mixed(some && !all)
                            .enabled(enabled),
                    )
                    .changed()
                {
                    self.checks = [parent; 3];
                }
                let [a, b, c] = &mut self.checks;
                form.indent(|form| {
                    form.add(Checkbox::new(a, "Unused imports").enabled(enabled));
                    form.add(
                        Checkbox::new(b, "Spelling")
                            .inline_comment("Requires restart")
                            .enabled(enabled),
                    );
                    form.add(
                        Checkbox::new(c, "Deprecated API")
                            .comment("Highlights calls to APIs marked deprecated by their library")
                            .enabled(enabled),
                    );
                });
            });
            form.group("Radio buttons", |form| {
                form.radios("Indent with", |ui| {
                    ui.add(Radio::new(&mut self.indent, 0, "Tabs").enabled(enabled));
                    ui.add(Radio::new(&mut self.indent, 1, "Spaces").enabled(enabled));
                });
            });
            form.group("Toggles", |form| {
                form.add(
                    Toggle::new(&mut self.toggles[0], "Save files automatically").enabled(enabled),
                );
            });
        });
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
