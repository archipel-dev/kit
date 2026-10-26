use super::*;

pub(super) struct State {
    toggles: [bool; 2],
    last_action: String,
    splits: [f32; 3],
    sidebar_collapsed: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            toggles: [true, false],
            last_action: String::new(),
            splits: [0.25, 0.5, 0.5],
            sidebar_collapsed: false,
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(TitledSeparator::new("Menu"));
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_width(260.0);
            ui.add(
                ActionMenuItem::new("Copy")
                    .icon(Glyph::Copy)
                    .shortcut("Ctrl+C"),
            );
            ui.add(
                ActionMenuItem::new("Find in Files…")
                    .icon(Glyph::FindInFiles)
                    .shortcut("Ctrl+Shift+F"),
            );
            ui.add(ListSeparator::new());
            let mut wrap = self.toggles[0];
            if ui
                .add(ActionMenuItem::new("Soft-Wrap").checked(wrap))
                .clicked()
            {
                wrap = !wrap;
                self.toggles[0] = wrap;
            }
            ui.add(ActionMenuItem::new("Run").icon(Glyph::Run).enabled(false));
            ui.add(ActionMenuItem::new("New").submenu(|ui| {
                ui.add(ActionMenuItem::new("File…").icon(Glyph::File));
                ui.add(ActionMenuItem::new("Directory…").icon(Glyph::Folder));
            }));
        });
        ui.add(TitledSeparator::new("Tool window header"));
        SimpleToolWindowPanel::new("Project")
            .header(180.0, |ui| {
                ui.label("Project");
                ui.add(SeparatorComponent::group());
                ui.label("Files");
            })
            .padded(true)
            .header_separator(true)
            .hover_actions(true)
            .actions(|ui| {
                ui.add(ActionButton::new(Glyph::CollapseAll, "Collapse all"));
                ui.add(ActionButton::new(Glyph::Search, "Find"));
            })
            .on_hide(|| self.last_action = "Hide Project".into())
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.label("src/");
            });
        ui.add(TitledSeparator::new("Status bar (Surface::Chrome)"));
        Surface::Chrome.frame(ui.ctx()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    StatusBarWidget::new("main")
                        .icon(Glyph::Branch)
                        .chevron(true)
                        .enabled(enabled),
                );
                ui.add(StatusBarWidget::new("UTF-8").enabled(enabled));
                ui.add(
                    StatusBarWidget::new("2 errors")
                        .tone(Tone::Error)
                        .enabled(enabled),
                );
                ui.add(
                    StatusBarWidget::new("A long item that is cut off")
                        .max_width(100.0)
                        .enabled(enabled),
                );
            });
        });
        ui.add(TitledSeparator::new("Context menu"));
        ui.add(Button::secondary("Right-click me"))
            .context_menu(|ui| {
                ui.add(ActionMenuItem::new("Rename…").shortcut("Shift+F6"));
                ui.add(ActionMenuItem::new("Delete").shortcut("Delete"));
            });
        ui.add(TitledSeparator::new("Split pane"));
        ui.label(
            "Drag the dividers, double-click to reset, or Tab to one and use the arrow keys. \
                     Drag the sidebar divider far to the left to collapse it, drag it back out to \
                     expand, or press Enter on it.",
        );
        let collapsed = self.sidebar_collapsed;
        let toggle = ui.add(
            ActionButton::new(
                Glyph::SidebarLeft,
                if collapsed {
                    "Show sidebar"
                } else {
                    "Hide sidebar"
                },
            )
            .selected(!collapsed)
            .enabled(enabled),
        );
        if toggle.clicked() {
            self.sidebar_collapsed = !collapsed;
        }
        let size = egui::vec2(ui.available_width(), 260.0);
        let [sidebar, request, response] = &mut self.splits;
        ui.allocate_ui(size, |ui| {
            Splitter::horizontal("gallery-sidebar", sidebar)
                .default_ratio(0.25)
                .min_size(100.0)
                .label("Resize sidebar")
                .collapsible(Side::First, &mut self.sidebar_collapsed)
                .enabled(enabled)
                .with(
                    HelpTooltip::new("Resize sidebar")
                        .hint("Double-click to reset the width, or to expand when collapsed"),
                )
                .show(
                    ui,
                    |ui| split_island(ui, "Collections", "src/\ntests/"),
                    |ui| {
                        Splitter::vertical("gallery-stack", request)
                            .min_size(60.0)
                            .label("Resize response")
                            .enabled(enabled)
                            .show(
                                ui,
                                |ui| split_island(ui, "Request", "GET /api/users"),
                                |ui| {
                                    Splitter::horizontal("gallery-response", response)
                                        .min_size(60.0)
                                        .label("Resize headers")
                                        .enabled(enabled)
                                        .show(
                                            ui,
                                            |ui| split_island(ui, "Body", "200 OK"),
                                            |ui| split_island(ui, "Headers", "ETag"),
                                        );
                                },
                            );
                    },
                );
        });
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}

/// A pane of the split demo: an island filling it, with a title and some text.
fn split_island(ui: &mut egui::Ui, title: &str, text: &str) {
    SimpleToolWindowPanel::new(title).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(tokens::size::CONTENT_INSET);
            ui.label(text);
        });
    });
}
