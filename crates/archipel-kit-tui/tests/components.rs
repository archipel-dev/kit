//! Behavior and SVG snapshot tests. The snapshots in `docs/tui/img/` are the docs images.

use archipel_kit::components::Severity;
use archipel_kit_tui::components::*;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{lines, render, snapshot};
use archipel_kit_tui::{Glyph, Language, Look, Symbol, SymbolKind, glyphs};

fn looks() -> [(&'static str, Look); 2] {
    [("dark", Look::dark()), ("light", Look::light())]
}

/// Renders `draw` on an island in both themes and checks the snapshots `<name>.<theme>`.
fn sheet(name: &str, width: u16, height: u16, draw: impl Fn(&Look, Rect, &mut Buffer)) {
    for (theme, look) in looks() {
        let buf = render(width, height, |area, buf| {
            Island::new(&look).render(area, buf);
            draw(&look, Rect::new(2, 1, width - 4, height - 2), buf);
        });
        snapshot(&format!("{name}.{theme}"), &buf);
    }
}

fn row(area: Rect, n: u16) -> Rect {
    Rect::new(area.x, area.y + n, area.width, 1)
}

#[test]
fn buttons() {
    sheet("button", 64, 5, |look, area, buf| {
        let mut x = area.x;
        for button in [
            Button::primary(look, "Run").icon(Glyph::Run),
            Button::secondary(look, "Cancel"),
            Button::ghost(look, "Details"),
            Button::danger(look, "Delete"),
        ] {
            let width = button.width();
            button.render(Rect::new(x, area.y, width, 1), buf);
            x += width + 1;
        }
        let mut x = area.x;
        for button in [
            Button::primary(look, "Focused").focused(true),
            Button::primary(look, "Disabled").enabled(false),
            Button::secondary(look, "Disabled").enabled(false),
        ] {
            let width = button.width();
            button.render(Rect::new(x, area.y + 2, width, 1), buf);
            x += width + 1;
        }
    });
}

#[test]
fn button_width_fits_label_and_icon() {
    let look = Look::dark();
    assert_eq!(Button::primary(&look, "Run").width(), 7);
    assert_eq!(Button::primary(&look, "Run").icon(Glyph::Run).width(), 9);
    let buf = render(12, 1, |area, buf| {
        Button::secondary(&look, "OK").render(area, buf)
    });
    assert_eq!(lines(&buf), ["\u{e0b6} OK \u{e0b4}"]);
}

#[test]
fn choices() {
    sheet("choice", 44, 8, |look, area, buf| {
        Checkbox::new(look, true, "Checked").render(row(area, 0), buf);
        Checkbox::new(look, false, "Unchecked").render(row(area, 1), buf);
        Checkbox::new(look, false, "Mixed")
            .mixed(true)
            .render(row(area, 2), buf);
        Checkbox::new(look, true, "Disabled")
            .enabled(false)
            .render(row(area, 3), buf);
        let half = Rect::new(area.x + 20, area.y, 20, area.height);
        Radio::new(look, true, "Selected").render(row(half, 0), buf);
        Radio::new(look, false, "Option").render(row(half, 1), buf);
        Toggle::new(look, true, "On").render(row(half, 3), buf);
        Toggle::new(look, false, "Off").render(row(half, 4), buf);
    });
}

#[test]
fn checkbox_marks() {
    let look = Look::dark();
    let mark = |checkbox: Checkbox| {
        let buf = render(10, 1, |area, buf| checkbox.render(area, buf));
        buf[(0, 0)].symbol().chars().next()
    };
    assert_eq!(
        mark(Checkbox::new(&look, true, "a")),
        Some(glyphs::CHECKBOX_ON)
    );
    assert_eq!(
        mark(Checkbox::new(&look, false, "a")),
        Some(glyphs::CHECKBOX_OFF)
    );
    let mixed = Checkbox::new(&look, false, "a").mixed(true);
    assert_eq!(mark(mixed), Some(glyphs::CHECKBOX_MIXED));
}

#[test]
fn text_fields() {
    sheet("text-field", 44, 9, |look, area, buf| {
        let field = TextField::new(look, "").placeholder("Search everywhere");
        field.render(row(area, 0), buf);
        let field = TextField::new(look, "archipel").focused(true);
        field.render(row(area, 2), buf);
        let field = TextField::new(look, "8o80")
            .label("Port")
            .error(Some("Must be a number"));
        let height = field.height();
        field.render(Rect::new(area.x, area.y + 4, area.width, height), buf);
    });
}

#[test]
fn text_field_height_counts_label_and_error() {
    let look = Look::dark();
    assert_eq!(TextField::new(&look, "").height(), 1);
    assert_eq!(
        TextField::new(&look, "")
            .label("L")
            .error(Some("E"))
            .height(),
        3
    );
}

#[test]
fn list_rows() {
    sheet("list-row", 40, 9, |look, area, buf| {
        ListRow::new(look, "kit")
            .icon(Glyph::Folder)
            .expanded(Some(true))
            .secondary("~/archipel")
            .render(row(area, 0), buf);
        ListRow::new(look, "src")
            .icon(Glyph::Folder)
            .depth(1)
            .expanded(Some(true))
            .render(row(area, 1), buf);
        ListRow::new(look, "main.rs")
            .icon(Language::Rust)
            .depth(2)
            .selected(true)
            .render(row(area, 2), buf);
        ListRow::new(look, "theme.py")
            .icon(Language::Python)
            .depth(2)
            .render(row(area, 3), buf);
        ListRow::new(look, "tests")
            .icon(Glyph::Folder)
            .depth(1)
            .expanded(Some(false))
            .render(row(area, 4), buf);
        ListRow::new(look, "Theme")
            .icon(Symbol::new(SymbolKind::Class))
            .render(row(area, 5), buf);
        ListRow::new(look, "apply(ctx)")
            .icon(Symbol::new(SymbolKind::Method))
            .depth(1)
            .render(row(area, 6), buf);
    });
}

#[test]
fn tab_bars() {
    let tabs = [
        (Some(Language::Rust.into()), "main.rs", false),
        (Some(Language::Toml.into()), "Cargo.toml", true),
        (Some(Glyph::FileText.into()), "README.md", false),
    ];
    sheet("tab-bar", 64, 3, |look, area, buf| {
        TabBar::new(look, &tabs, 0).render(area, buf);
    });
}

#[test]
fn banners() {
    sheet("banner", 64, 9, |look, area, buf| {
        let actions = ["Retry", "Dismiss"];
        Banner::new(look, Severity::Info, "Indexing finished", &[]).render(row(area, 0), buf);
        Banner::new(look, Severity::Success, "Build succeeded", &[]).render(row(area, 2), buf);
        Banner::new(look, Severity::Warning, "Low disk space", &["Clean"])
            .render(row(area, 4), buf);
        Banner::new(look, Severity::Error, "Connection lost", &actions).render(row(area, 6), buf);
    });
}

#[test]
fn misc() {
    sheet("misc", 48, 9, |look, area, buf| {
        SegmentedControl::new(look, &["Tree", "List", "Graph"], 1).render(row(area, 0), buf);
        ProgressBar::new(look, 0.6).render(row(area, 2), buf);
        Kbd::new(look, &["Ctrl", "Shift", "A"]).render(row(area, 4), buf);
        let mut x = area.x;
        for chip in [
            Chip::tag(look, "draft"),
            Chip::counter(look, "12", true),
            Chip::colored(
                look,
                "rust",
                archipel_kit::egui::Color32::from_rgb(0xDE, 0xA5, 0x84),
            ),
        ] {
            chip.render(Rect::new(x, area.y + 6, 12, 1), buf);
            x += 9;
        }
    });
}

#[test]
fn icons() {
    sheet("icons", 72, 9, |look, area, buf| {
        let (mut x, mut y) = (area.x, area.y);
        for language in Language::ALL.iter().copied().take(44) {
            if x + 2 > area.right() {
                x = area.x;
                y += 1;
            }
            LanguageIcon::new(look, language).render(Rect::new(x, y, 3, 1), buf);
            x += 3;
        }
        let (mut x, y) = (area.x, y + 2);
        for kind in SymbolKind::ALL {
            SymbolIcon::new(look, Symbol::new(kind)).render(Rect::new(x, y, 3, 1), buf);
            x += 3;
        }
        let (mut x, y) = (area.x, y + 2);
        for glyph in Glyph::ALL.iter().copied().take(22) {
            buf.set_string(
                x,
                y,
                look.glyph(glyph),
                look.body().fg(look.glyph_color(glyph)),
            );
            x += 3;
        }
    });
}

#[test]
fn every_icon_is_a_nerd_font_character() {
    let private_use = |ch: char| matches!(ch as u32, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD);
    for glyph in Glyph::ALL.iter().copied() {
        assert!(private_use(glyphs::nerd_font(glyph)), "{glyph:?}");
    }
    for language in Language::ALL.iter().copied() {
        assert!(private_use(glyphs::language(language)), "{language:?}");
    }
    for kind in SymbolKind::ALL {
        assert!(private_use(glyphs::symbol(kind)), "{kind:?}");
    }
}

#[test]
fn dark_language_colors_stay_readable() {
    let look = Look::dark();
    // Navy on the dark island is lifted toward the text color.
    assert_ne!(
        look.readable(Language::Lua.color()),
        Look::rgb(Language::Lua.color())
    );
    // Bright colors are kept.
    assert_eq!(look.readable(0xDEA584), Look::rgb(0xDEA584));
}

#[test]
fn widgets_clip_to_tiny_areas() {
    let look = Look::dark();
    for (w, h) in [(0, 0), (1, 1), (3, 1)] {
        render(w.max(1), h.max(1), |_, buf| {
            let area = Rect::new(0, 0, w, h);
            Button::primary(&look, "A long label").render(area, buf);
            TextField::new(&look, "value")
                .error(Some("error"))
                .render(area, buf);
            Banner::new(&look, Severity::Error, "text", &["Action"]).render(area, buf);
            ProgressBar::new(&look, 0.5).render(area, buf);
            ListRow::new(&look, "row")
                .depth(9)
                .icon(Language::Rust)
                .render(area, buf);
        });
    }
}

/// An IDE screen built from the components: the docs hero image.
#[test]
fn ide_screen() {
    for (theme, look) in looks() {
        let buf = render(96, 24, |area, buf| {
            Frame::new(&look).render(area, buf);
            let project = Rect::new(1, 1, 30, 22);
            let island = Island::new(&look).title("Project");
            let inner = island.inner(project);
            island.render(project, buf);
            let rows: [(&str, u16, Option<bool>, RowIcon); 8] = [
                ("kit", 0, Some(true), Glyph::Folder.into()),
                ("crates", 1, Some(true), Glyph::Folder.into()),
                ("archipel-kit", 2, Some(false), Glyph::Folder.into()),
                ("archipel-kit-tui", 2, Some(true), Glyph::Folder.into()),
                ("lib.rs", 3, None, Language::Rust.into()),
                ("testing.rs", 3, None, Language::Rust.into()),
                ("Cargo.toml", 1, None, Language::Toml.into()),
                ("README.md", 1, None, Language::Markdown.into()),
            ];
            for (i, (label, depth, expanded, icon)) in rows.into_iter().enumerate() {
                ListRow::new(&look, label)
                    .depth(depth)
                    .expanded(expanded)
                    .icon(icon)
                    .selected(i == 4)
                    .render(
                        Rect::new(project.x, inner.y + i as u16, project.width, 1),
                        buf,
                    );
            }
            let editor = Rect::new(32, 1, 63, 15);
            Island::new(&look).render(editor, buf);
            let tabs = [
                (Some(Language::Rust.into()), "lib.rs", false),
                (Some(Language::Rust.into()), "testing.rs", true),
            ];
            TabBar::new(&look, &tabs, 0).render(Rect::new(33, 2, 61, 1), buf);
            let c = &look.theme.colors;
            let code = [
                ("pub struct ", "Look", " {"),
                ("    pub ", "theme", ": Theme,"),
                ("    pub ", "dark", ": bool,"),
                ("}", "", ""),
            ];
            for (i, (a, b, d)) in code.iter().enumerate() {
                let y = 4 + i as u16;
                buf.set_string(
                    34,
                    y,
                    format!("{:>2}", i + 1),
                    look.text(c.text_placeholder),
                );
                let x = 38;
                buf.set_string(x, y, *a, look.text(c.text));
                buf.set_string(x + a.len() as u16, y, *b, look.strong());
                buf.set_string(x + (a.len() + b.len()) as u16, y, *d, look.text(c.text));
            }
            Banner::new(&look, Severity::Info, "Index ready", &["Open"])
                .render(Rect::new(32, 14, 63, 1), buf);
            let run = Rect::new(32, 17, 63, 6);
            let island = Island::new(&look).title("Run");
            let inner = island.inner(run);
            island.render(run, buf);
            let mut x = inner.x;
            for button in [
                Button::primary(&look, "Run").icon(Glyph::Run),
                Button::secondary(&look, "Stop").icon(Glyph::Stop),
            ] {
                let width = button.width();
                button.render(Rect::new(x, inner.y, width, 1), buf);
                x += width + 1;
            }
            ProgressBar::new(&look, 0.7)
                .render(Rect::new(x + 1, inner.y, inner.right() - x - 1, 1), buf);
            buf.set_string(
                inner.x,
                inner.y + 2,
                "Compiling archipel-kit-tui v0.1.0",
                look.text(c.text_secondary),
            );
        });
        snapshot(&format!("ide.{theme}"), &buf);
    }
}

#[test]
fn every_snapshot_is_shown_in_the_docs() {
    let page = std::fs::read_to_string("../../docs/tui/README.md").expect("docs page");
    for entry in std::fs::read_dir("../../docs/tui/img")
        .expect("img dir")
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            page.contains(&format!("img/{name}")),
            "{name} is not on docs/tui/README.md"
        );
    }
}
