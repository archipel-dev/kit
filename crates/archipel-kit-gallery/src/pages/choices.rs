use super::*;
use archipel_kit::ui::dsl::{RadioGroup, panel};

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
        // A settings page: the DSL owns the spacing, groups and indents.
        let mut parent = {
            let [a, b, c] = self.checks;
            a && b && c
        };
        let some = self.checks.iter().any(|&on| on);
        let all = self.checks.iter().all(|&on| on);
        let mut changed_parent = false;
        let [a, b, c] = &mut self.checks;
        let (indent, toggle) = (RadioGroup::new(&mut self.indent), &mut self.toggles[0]);
        panel(ui, |p| {
            p.group("Checkboxes", |p| {
                p.unlabeled_row(|r| {
                    r.cell(
                        JBCheckBox::new(&mut parent, "All inspections")
                            .mixed(some && !all)
                            .enabled(enabled),
                    )
                    .on_response(|response| changed_parent = response.changed());
                });
                p.indent(|p| {
                    p.unlabeled_row(|r| {
                        r.cell(JBCheckBox::new(a, "Unused imports").enabled(enabled));
                    });
                    p.unlabeled_row(|r| {
                        r.cell(
                            JBCheckBox::new(b, "Spelling")
                                .inline_comment("Requires restart")
                                .enabled(enabled),
                        );
                    });
                    p.unlabeled_row(|r| {
                        r.cell(JBCheckBox::new(c, "Deprecated API").enabled(enabled))
                            .comment("Highlights calls to APIs marked deprecated by their library");
                    });
                });
            });
            p.group("Radio buttons", |p| {
                p.row("Indent with", |r| {
                    r.radio_button(&indent, 0, "Tabs").enabled_if(enabled);
                    r.radio_button(&indent, 1, "Spaces").enabled_if(enabled);
                });
            });
            p.group("Toggles", |p| {
                p.unlabeled_row(|r| {
                    r.cell(OnOffButton::new(toggle, "Save files automatically").enabled(enabled));
                });
            });
        });
        if changed_parent {
            self.checks = [parent; 3];
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
