use super::*;

#[derive(Default)]
pub(super) struct State {
    icon_filter: String,
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, _enabled: bool) {
        ui.add(TextField::search(&mut self.icon_filter).placeholder("Search icons"));
        ui.add(SectionHeader::new("Language icon style"));
        let mut style = Theme::current(ui.ctx()).language_icons;
        let styles = LanguageStyle::ALL.map(|style| (style, style.name()));
        if ui.add(SegmentedControl::new(&mut style, styles)).changed() {
            let mut theme = (*Theme::current(ui.ctx())).clone();
            theme.language_icons = style;
            theme.apply(ui.ctx());
        }
        let color = Theme::colors(ui.ctx()).icon;
        let matches: Vec<Glyph> = Glyph::search(&self.icon_filter).collect();
        for category in Category::ALL {
            let glyphs: Vec<Glyph> = matches
                .iter()
                .copied()
                .filter(|g| g.category() == category)
                .collect();
            if glyphs.is_empty() {
                continue;
            }
            ui.add(SectionHeader::new(category.label()));
            egui::Grid::new(category.label())
                .num_columns(4)
                .spacing([24.0, 8.0])
                .show(ui, |ui| {
                    for (index, glyph) in glyphs.iter().enumerate() {
                        ui.horizontal(|ui| {
                            archipel_kit::icons::show(ui, *glyph, 16.0, color);
                            ui.label(glyph.name());
                        })
                        .response
                        .on_hover_text(glyph.keywords().join(", "));
                        if index % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
        }
        let query = self.icon_filter.to_lowercase();
        let languages: Vec<Language> = Language::ALL
            .iter()
            .copied()
            .filter(|l| {
                l.name().to_lowercase().contains(&query)
                    || l.extensions().iter().any(|e| e.contains(&query))
            })
            .collect();
        if !languages.is_empty() {
            ui.add(SectionHeader::new("Languages"));
            egui::Grid::new("languages")
                .num_columns(4)
                .spacing([24.0, 8.0])
                .show(ui, |ui| {
                    for (index, language) in languages.iter().enumerate() {
                        ui.horizontal(|ui| {
                            archipel_kit::icons::show(ui, *language, 16.0, color);
                            ui.label(language.name());
                        })
                        .response
                        .on_hover_text(
                            language
                                .extensions()
                                .iter()
                                .map(|e| format!(".{e}"))
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                        if index % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
        }
        if query.is_empty() || "symbol".contains(&query) {
            ui.add(SectionHeader::new("Symbols"));
            ui.horizontal_wrapped(|ui| {
                for kind in SymbolKind::ALL {
                    archipel_kit::icons::show(ui, kind, 16.0, color).on_hover_text(kind.name());
                }
            });
        }
        if matches.is_empty() && languages.is_empty() {
            EmptyState::new("No icons match")
                .hint("Try a keyword such as \"http\" or \"trash\".")
                .show(ui);
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
