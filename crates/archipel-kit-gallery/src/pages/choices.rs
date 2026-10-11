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
        ui.add(SectionHeader::new("Checkboxes"));
        let [a, b, c] = &mut self.checks;
        let all = *a && *b && *c;
        let some = *a || *b || *c;
        let mut parent = all;
        if ui
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
        ui.indent("children", |ui| {
            ui.add(Checkbox::new(a, "Unused imports").enabled(enabled));
            ui.add(Checkbox::new(b, "Spelling").enabled(enabled));
            ui.add(Checkbox::new(c, "Deprecated API").enabled(enabled));
        });
        ui.add(SectionHeader::new("Radio buttons"));
        ui.add(Radio::new(&mut self.indent, 0, "Tabs").enabled(enabled));
        ui.add(Radio::new(&mut self.indent, 1, "Spaces").enabled(enabled));
        ui.add(SectionHeader::new("Toggles"));
        ui.add(Toggle::new(&mut self.toggles[0], "Save files automatically").enabled(enabled));
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
