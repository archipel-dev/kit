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
    tabs: Vec<Tab>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            view: View::Pretty,
            tab: 0,
            tabs: ["main.rs", "theme.rs", "Cargo.toml", "README.md"]
                .into_iter()
                .map(|name| Tab::new(name).icon(Icon::for_file(name)).closable(true))
                .collect(),
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(SectionHeader::new("Segmented control"));
        ui.add(
            SegmentedControl::new(
                &mut self.view,
                [
                    (View::Pretty, "Pretty"),
                    (View::Raw, "Raw"),
                    (View::Preview, "Preview"),
                ],
            )
            .enabled(enabled),
        );
        ui.add(SectionHeader::new("Tab bar"));
        ui.add(
            SegmentedControl::new(
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
        let out = TabBar::new(&self.tabs, self.tab).show(ui);
        if let Some(index) = out.selected {
            self.tab = index;
        }
        if let Some(index) = out.closed {
            self.tabs.remove(index);
            self.tab = self.tab.min(self.tabs.len().saturating_sub(1));
        }
        if self.tabs.is_empty() && ui.add(Link::new("Reopen tabs")).clicked() {
            self.tabs = State::default().tabs;
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
