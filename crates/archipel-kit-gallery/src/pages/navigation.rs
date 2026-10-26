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

/// Moves the tab at `from` to `to` (what a drag reports) and keeps `selected` on the same tab.
fn move_tab(tabs: &mut Vec<TabInfo>, selected: &mut usize, from: usize, to: usize) {
    if from >= tabs.len() || to >= tabs.len() {
        return;
    }
    let tab = tabs.remove(from);
    tabs.insert(to, tab);
    *selected = if *selected == from {
        to
    } else if from < *selected && to >= *selected {
        *selected - 1
    } else if from > *selected && to <= *selected {
        *selected + 1
    } else {
        *selected
    };
}

/// Pins or unpins the tab at `index`.
fn toggle_pin(tabs: &mut [TabInfo], index: usize) {
    if let Some(tab) = tabs.get_mut(index) {
        *tab = tab.clone().pinned(!tab.is_pinned());
    }
}

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
            tabs: {
                // Drag tabs to reorder them; the pin unpins.
                let mut tabs = file_tabs(&["main.rs", "theme.rs", "Cargo.toml", "README.md"]);
                tabs[0] = tabs[0].clone().pinned(true);
                tabs
            },
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
        if let Some((from, to)) = out.moved {
            move_tab(&mut self.tabs, &mut self.tab, from, to);
        }
        if let Some(index) = out.pin_toggled {
            toggle_pin(&mut self.tabs, index);
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
            if let Some((from, to)) = out.moved {
                move_tab(&mut self.many_tabs, &mut self.many_tab, from, to);
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
