//! Behavior and SVG snapshot tests. The snapshots in `docs/tui/img/` are the docs images.

use archipel_kit::ui::Status;
use archipel_kit_tui::components::*;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{lines, render, snapshot};
use archipel_kit_tui::{Glyph, Language, Look, Symbol, SymbolKind, glyphs};

fn looks() -> [(&'static str, Look); 2] {
    [("dark", Look::dark()), ("light", Look::light())]
}

/// Renders `draw` in a panel in both themes and checks the snapshots `<name>.<theme>`.
fn sheet(name: &str, width: u16, height: u16, draw: impl Fn(&Look, Rect, &mut Buffer)) {
    for (theme, look) in looks() {
        let buf = render(width, height, |area, buf| {
            let island = Island::new(&look);
            let inner = island.inner(area);
            island.render(area, buf);
            draw(&look, inner, buf);
        });
        snapshot(&format!("{name}.{theme}"), &buf);
    }
}

fn row(area: Rect, n: u16) -> Rect {
    Rect::new(area.x, area.y + n, area.width, 1)
}

/// Renders `buttons` left to right on row `y`.
fn button_row(area: Rect, y: u16, buttons: Vec<Button>, buf: &mut Buffer) {
    let mut x = area.x;
    for button in buttons {
        let width = button.width();
        button.render(Rect::new(x, area.y + y, width, 1), buf);
        x += width;
    }
}

#[test]
fn buttons() {
    sheet("button", 64, 5, |look, area, buf| {
        let buttons = vec![
            Button::primary(look, "Run").icon(Glyph::Run).key('r'),
            Button::secondary(look, "Cancel").key('c'),
            Button::ghost(look, "Details"),
            Button::danger(look, "Delete"),
        ];
        button_row(area, 0, buttons, buf);
        let buttons = vec![
            Button::primary(look, "Focused").focused(true),
            Button::primary(look, "Disabled").enabled(false),
            Button::ghost(look, "Disabled").enabled(false),
        ];
        button_row(area, 2, buttons, buf);
    });
}

#[test]
fn button_width_and_focus_markers() {
    let look = Look::dark();
    assert_eq!(Button::primary(&look, "Run").width(), 7);
    assert_eq!(Button::primary(&look, "Run").icon(Glyph::Run).width(), 9);
    let text = |button: Button| lines(&render(12, 1, |area, buf| button.render(area, buf)));
    assert_eq!(text(Button::secondary(&look, "OK")), ["  OK"]);
    assert_eq!(
        text(Button::secondary(&look, "OK").focused(true)),
        ["❯ OK ❮"]
    );
}

#[test]
fn choices() {
    sheet("choice", 48, 8, |look, area, buf| {
        JBCheckBox::new(look, true, "Checked").render(row(area, 0), buf);
        JBCheckBox::new(look, false, "Unchecked")
            .focused(true)
            .render(row(area, 1), buf);
        JBCheckBox::new(look, false, "Mixed")
            .mixed(true)
            .render(row(area, 2), buf);
        JBCheckBox::new(look, true, "Disabled")
            .enabled(false)
            .render(row(area, 3), buf);
        let half = Rect::new(area.x + 22, area.y, 20, area.height);
        JBRadioButton::new(look, true, "Selected").render(row(half, 0), buf);
        JBRadioButton::new(look, false, "Option").render(row(half, 1), buf);
        OnOffButton::new(look, true, "On").render(row(half, 3), buf);
        OnOffButton::new(look, false, "Off").render(row(half, 4), buf);
    });
}

#[test]
fn checkbox_marks() {
    let look = Look::dark();
    let mark = |checkbox: JBCheckBox| {
        let buf = render(10, 1, |area, buf| checkbox.render(area, buf));
        buf[(0, 0)].symbol().chars().next()
    };
    assert_eq!(
        mark(JBCheckBox::new(&look, true, "a")),
        Some(glyphs::CHECKBOX_ON)
    );
    assert_eq!(
        mark(JBCheckBox::new(&look, false, "a")),
        Some(glyphs::CHECKBOX_OFF)
    );
    let mixed = JBCheckBox::new(&look, false, "a").mixed(true);
    assert_eq!(mark(mixed), Some(glyphs::CHECKBOX_MIXED));
}

#[test]
fn text_fields() {
    sheet("text-field", 44, 9, |look, area, buf| {
        let field = |n: u16, h: u16| Rect::new(area.x, area.y + n, area.width, h);
        JBTextField::new(look, "")
            .placeholder("Search everywhere")
            .render(field(0, 1), buf);
        JBTextField::new(look, "archipel")
            .label("Name")
            .focused(true)
            .render(field(2, 1), buf);
        JBTextField::new(look, "8o80")
            .label("Port")
            .error(Some("Must be a number"))
            .render(field(4, 2), buf);
    });
}

#[test]
fn text_field_shows_label_value_and_error() {
    let look = Look::dark();
    let field = JBTextField::new(&look, "8o80")
        .label("Port")
        .error(Some("Must be a number"));
    assert_eq!(field.height(), 2);
    let buf = render(30, 2, |area, buf| field.render(area, buf));
    let text = lines(&buf);
    assert!(text[0].contains(" Port  8o80"), "{text:?}");
    assert!(text[1].ends_with("Must be a number"), "{text:?}");
}

#[test]
fn list_rows() {
    sheet("list-row", 40, 9, |look, area, buf| {
        let full = Rect::new(area.x - 1, area.y, area.width + 2, area.height);
        ColoredListCellRenderer::new(look, "kit")
            .icon(Glyph::Folder)
            .expanded(Some(true))
            .secondary("~/archipel")
            .render(row(full, 0), buf);
        ColoredListCellRenderer::new(look, "src")
            .icon(Glyph::Folder)
            .depth(1)
            .expanded(Some(true))
            .render(row(full, 1), buf);
        ColoredListCellRenderer::new(look, "main.rs")
            .icon(Language::Rust)
            .depth(2)
            .selected(true)
            .render(row(full, 2), buf);
        ColoredListCellRenderer::new(look, "theme.py")
            .icon(Language::Python)
            .depth(2)
            .render(row(full, 3), buf);
        ColoredListCellRenderer::new(look, "tests")
            .icon(Glyph::Folder)
            .depth(1)
            .expanded(Some(false))
            .render(row(full, 4), buf);
        ColoredListCellRenderer::new(look, "Theme")
            .icon(Symbol::new(SymbolKind::Class))
            .render(row(full, 5), buf);
        ColoredListCellRenderer::new(look, "apply(ctx)")
            .icon(Symbol::new(SymbolKind::Method))
            .depth(1)
            .render(row(full, 6), buf);
    });
}

#[test]
fn tab_bars() {
    let tabs = [
        (Some(Language::Rust.into()), "main.rs", false),
        (Some(Language::Toml.into()), "Cargo.toml", true),
        (Some(Glyph::FileText.into()), "README.md", false),
    ];
    for (theme, look) in looks() {
        let buf = render(60, 3, |area, buf| {
            Island::new(&look).render(area, buf);
            JBTabs::new(&look, &tabs, 0).render(Rect::new(0, 0, 60, 1), buf);
        });
        snapshot(&format!("tab-bar.{theme}"), &buf);
    }
}

#[test]
fn banners() {
    sheet("banner", 64, 9, |look, area, buf| {
        let actions = ["Retry", "Dismiss"];
        InlineBanner::new(look, Status::Info, "Indexing finished", &[]).render(row(area, 0), buf);
        InlineBanner::new(look, Status::Success, "Build succeeded", &[]).render(row(area, 2), buf);
        InlineBanner::new(look, Status::Warning, "Low disk space", &["Clean"])
            .render(row(area, 4), buf);
        InlineBanner::new(look, Status::Error, "Connection lost", &actions)
            .render(row(area, 6), buf);
    });
}

#[test]
fn misc() {
    sheet("misc", 52, 9, |look, area, buf| {
        SegmentedButton::new(look, &["Tree", "List", "Graph"], 1).render(row(area, 0), buf);
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
            let width = chip.width();
            chip.render(Rect::new(x, area.y + 6, width, 1), buf);
            x += width + 1;
        }
    });
}

#[test]
fn status_bars() {
    for (theme, look) in looks() {
        let buf = render(72, 1, |area, buf| {
            StatusBar::new(&look, &[("⏎", "open"), ("/", "search"), ("q", "quit")])
                .mode("NORMAL")
                .right("main  Ln 12, Col 4  UTF-8")
                .render(area, buf);
        });
        snapshot(&format!("status-bar.{theme}"), &buf);
    }
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
            let style = look.body().fg(look.glyph_color(glyph));
            buf.set_string(x, y, look.glyph(glyph), style);
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
    for (w, h) in [(0, 0), (1, 1), (3, 1), (5, 2)] {
        render(w.max(1), h.max(1), |_, buf| {
            let area = Rect::new(0, 0, w, h);
            Island::new(&look).title("A long title").render(area, buf);
            Button::primary(&look, "A long label").render(area, buf);
            JBTextField::new(&look, "value")
                .label("Label")
                .error(Some("error"))
                .render(area, buf);
            InlineBanner::new(&look, Status::Error, "text", &["Action"]).render(area, buf);
            ProgressBar::new(&look, 0.5).render(area, buf);
            StatusBar::new(&look, &[("q", "quit")])
                .mode("N")
                .right("r")
                .render(area, buf);
            ColoredListCellRenderer::new(&look, "row")
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
        let buf = render(100, 26, |area, buf| {
            Frame::new(&look).render(area, buf);
            let c = &look.theme.colors;
            // Project tree, focused.
            let project = Rect::new(0, 0, 30, 25);
            let island = Island::new(&look).title("Project").focused(true);
            let inner = island.inner(project);
            island.render(project, buf);
            let rows: [(&str, u16, Option<bool>, RowIcon); 9] = [
                ("kit", 0, Some(true), Glyph::Folder.into()),
                ("crates", 1, Some(true), Glyph::Folder.into()),
                ("archipel-kit", 2, Some(false), Glyph::Folder.into()),
                ("archipel-kit-tui", 2, Some(true), Glyph::Folder.into()),
                ("components.rs", 3, None, Language::Rust.into()),
                ("lib.rs", 3, None, Language::Rust.into()),
                ("Cargo.toml", 1, None, Language::Toml.into()),
                ("README.md", 1, None, Language::Markdown.into()),
                ("flake.nix", 1, None, Language::Nix.into()),
            ];
            for (i, (label, depth, expanded, icon)) in rows.into_iter().enumerate() {
                ColoredListCellRenderer::new(&look, label)
                    .depth(depth)
                    .expanded(expanded)
                    .icon(icon)
                    .selected(i == 5)
                    .render(
                        Rect::new(project.x, inner.y + i as u16, project.width, 1),
                        buf,
                    );
            }
            // Tab line and editor.
            let tabs = [
                (Some(Language::Rust.into()), "lib.rs", false),
                (Some(Language::Rust.into()), "components.rs", true),
                (Some(Language::Toml.into()), "Cargo.toml", false),
            ];
            JBTabs::new(&look, &tabs, 0).render(Rect::new(31, 0, 69, 1), buf);
            let editor = Rect::new(31, 1, 69, 15);
            Island::new(&look).render(editor, buf);
            let code = [
                ("pub struct ", "Look", " {"),
                ("    pub ", "theme", ": Theme,"),
                ("}", "", ""),
                ("", "", ""),
                ("impl ", "Look", " {"),
                ("    pub fn ", "dark", "() -> Self {"),
                ("        Self::new(Theme::dark())", "", ""),
                ("    }", "", ""),
                ("}", "", ""),
            ];
            for (i, (a, b, d)) in code.iter().enumerate() {
                let y = 2 + i as u16;
                let number = format!("{:>3}", i + 1);
                buf.set_string(32, y, number, look.text(c.text_placeholder));
                let x = 37;
                buf.set_string(x, y, *a, look.text(c.text));
                buf.set_string(x + a.len() as u16, y, *b, look.strong());
                buf.set_string(x + (a.len() + b.len()) as u16, y, *d, look.text(c.text));
            }
            InlineBanner::new(&look, Status::Info, "Index ready", &["Open log"])
                .render(Rect::new(32, 14, 67, 1), buf);
            // Run tool window.
            let run = Rect::new(31, 17, 69, 8);
            let island = Island::new(&look).title("Run");
            let inner = island.inner(run);
            island.render(run, buf);
            let buttons = vec![
                Button::primary(&look, "Rerun").icon(Glyph::Run).key('r'),
                Button::secondary(&look, "Stop").icon(Glyph::Stop).key('s'),
            ];
            button_row(inner, 0, buttons, buf);
            ProgressBar::new(&look, 0.7)
                .render(Rect::new(inner.x + 22, inner.y, inner.width - 22, 1), buf);
            for (i, line) in [
                "   Compiling archipel-kit-tui v0.1.0",
                "    Finished `dev` profile in 2.41s",
                "     Running `target/debug/examples/gallery`",
            ]
            .iter()
            .enumerate()
            {
                let color = if i == 1 { c.success } else { c.text_secondary };
                buf.set_string(inner.x, inner.y + 2 + i as u16, *line, look.text(color));
            }
            StatusBar::new(&look, &[("⏎", "open"), ("/", "search"), ("?", "help")])
                .mode("PROJECT")
                .right("main  12:4  UTF-8")
                .render(Rect::new(0, 25, 100, 1), buf);
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
