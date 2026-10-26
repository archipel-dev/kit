use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Pretty,
    Raw,
    Preview,
}

pub(super) struct State {
    view: View,
    tab: usize,
    tabs: Vec<TabInfo>,
    /// The overflowing strip: more tabs than fit, a "More actions" menu.
    many_tab: usize,
    many_tabs: Vec<TabInfo>,
}

/// Files for the overflowing strip.
const MANY: [&str; 9] = [
    "main.rs",
    "lib.rs",
    "theme.rs",
    "tokens.rs",
    "segmented.rs",
    "navigation.rs",
    "Cargo.toml",
    "README.md",
    "CHANGELOG.md",
];

fn file_tabs(names: &[&str]) -> Vec<TabInfo> {
    names
        .iter()
        .map(|name| {
            TabInfo::new(*name)
                .icon(Icon::for_file(name))
                .closable(true)
        })
        .collect()
}

impl Default for State {
    fn default() -> Self {
        Self {
            view: View::Pretty,
            tab: 0,
            tabs: file_tabs(&["main.rs", "theme.rs", "Cargo.toml", "README.md"]),
            many_tab: 2,
            many_tabs: file_tabs(&MANY),
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(TitledSeparator::new("Segmented control"));
        ui.add(
            SegmentedButton::new(
                &mut self.view,
                [
                    (View::Pretty, "Pretty"),
                    (View::Raw, "Raw"),
                    (View::Preview, "Preview"),
                ],
            )
            .enabled(enabled),
        );
        ui.add(TitledSeparator::new("Tab bar"));
        ui.add(
            SegmentedButton::new(
                &mut self.view,
                [
                    (View::Pretty, "Pretty"),
                    (View::Raw, "Raw"),
                    (View::Preview, "Preview"),
                ],
            )
            .icon(View::Pretty, Glyph::Code)
            .icon(View::Raw, Glyph::File)
            .icon(View::Preview, Glyph::Eye)
            .icon_only(true)
            .enabled(enabled),
        );
        let out = JBTabs::new(&self.tabs, self.tab).show(ui);
        if let Some(index) = out.selected {
            self.tab = index;
        }
        if let Some(index) = out.closed {
            self.tabs.remove(index);
            self.tab = self.tab.min(self.tabs.len().saturating_sub(1));
        }
        if self.tabs.is_empty() && ui.add(ActionLink::new("Reopen tabs")).clicked() {
            self.tabs = State::default().tabs;
        }
        // More tabs than fit: the strip scrolls (wheel, or the thin bar at its top edge
        // while the pointer is over it), and "Show hidden tabs" lists the ones out of view.
        let mut close_all = false;
        ui.allocate_ui(egui::vec2(360.0, tokens::size::HEADER_HEIGHT), |ui| {
            let out = JBTabs::new(&self.many_tabs, self.many_tab)
                .id_salt("many")
                .more_menu(|ui| {
                    if ui.add(ActionMenuItem::new("Close All Tabs")).clicked() {
                        close_all = true;
                    }
                })
                .show(ui);
            if let Some(index) = out.selected {
                self.many_tab = index;
            }
            if let Some(index) = out.closed {
                self.many_tabs.remove(index);
                self.many_tab = self.many_tab.min(self.many_tabs.len().saturating_sub(1));
            }
        });
        if close_all {
            self.many_tabs.clear();
        }
        if self.many_tabs.is_empty() && ui.add(ActionLink::new("Reopen all tabs")).clicked() {
            self.many_tabs = file_tabs(&MANY);
            self.many_tab = 0;
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
