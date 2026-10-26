//! Behavior and SVG snapshot tests of the render-only widgets. The snapshots in
//! `docs/tui/img/` are the docs images.

use archipel_kit::ui::Status;
use archipel_kit_tui::components::*;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{lines, render, snapshot};
use archipel_kit_tui::{Glyph, Language, Look, Mode, Symbol, SymbolKind, Theme, glyphs};

fn looks() -> [(&'static str, Look); 2] {
    [("dark", Look::dark()), ("light", Look::light())]
}

/// Renders `draw` on an editor pane in both themes and checks the snapshots `<name>.<theme>`.
fn sheet(name: &str, width: u16, height: u16, draw: impl Fn(&Look, Rect, &mut Buffer)) {
    for (theme, look) in looks() {
        let buf = render(width, height, |area, buf| {
            Pane::new(&look).render(area, buf);
            let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
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
    sheet("button", 64, 6, |look, area, buf| {
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
    sheet("choice", 48, 9, |look, area, buf| {
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
    sheet("text-field", 46, 10, |look, area, buf| {
        let field = |n: u16, h: u16| Rect::new(area.x, area.y + n, area.width, h);
        JBTextField::new(look, "")
            .placeholder("Search everywhere")
            .render(field(0, 1), buf);
        JBTextField::new(look, "archipel")
            .label("Name")
            .focused(true)
            .caret(4)
            .selection(Some(5..8))
            .render(field(2, 1), buf);
        JBTextField::new(look, "8o80")
            .label("Port")
            .error(Some("Must be a number"))
            .render(field(4, 2), buf);
        JBTextField::new(look, "find me")
            .clear_button(true)
            .render(field(7, 1), buf);
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
    for (theme, look) in looks() {
        let buf = render(40, 9, |area, buf| {
            Pane::new(&look).kind(PaneKind::Sidebar).render(area, buf);
            let bg = look.palette.darker_black;
            let rows: [(&str, u16, Option<bool>, RowIcon, bool); 7] = [
                ("kit", 0, Some(true), RowIcon::Folder(true), false),
                ("src", 1, Some(true), RowIcon::Folder(true), false),
                ("main.rs", 2, None, Language::Rust.into(), true),
                ("theme.py", 2, None, Language::Python.into(), false),
                ("tests", 1, Some(false), RowIcon::Folder(false), false),
                (
                    "Theme",
                    0,
                    None,
                    Symbol::new(SymbolKind::Class).into(),
                    false,
                ),
                (
                    "apply(ctx)",
                    1,
                    None,
                    Symbol::new(SymbolKind::Method).into(),
                    false,
                ),
            ];
            for (i, (label, depth, expanded, icon, selected)) in rows.into_iter().enumerate() {
                let mut cell = ColoredListCellRenderer::new(&look, label)
                    .icon(icon)
                    .depth(depth)
                    .expanded(expanded)
                    .selected(selected)
                    .guides(true)
                    .on(bg);
                if i == 0 {
                    cell = cell.secondary("~/archipel");
                }
                cell.render(row(area, i as u16), buf);
            }
            ColoredListCellRenderer::new(&look, "apply(ctx)")
                .highlights(&[0..2, 3..4])
                .on(bg)
                .render(row(area, 8), buf);
        });
        snapshot(&format!("list-row.{theme}"), &buf);
    }
}

fn tabs() -> Vec<TabInfo> {
    vec![
        TabInfo::new("main.rs").icon(Language::Rust),
        TabInfo::new("Cargo.toml")
            .icon(Language::Toml)
            .modified(true),
        TabInfo::new("README.md").icon(Glyph::FileText),
    ]
}

#[test]
fn tab_bars() {
    for (theme, look) in looks() {
        let tabs = tabs();
        let buf = render(60, 3, |area, buf| {
            Pane::new(&look).render(area, buf);
            JBTabs::new(&look, &tabs, 0).render(Rect::new(0, 0, 60, 1), buf);
        });
        snapshot(&format!("tab-bar.{theme}"), &buf);
    }
}

#[test]
fn tabs_layout_covers_tabs_and_close_icons() {
    let look = Look::dark();
    let tabs = tabs();
    let layout = JBTabs::new(&look, &tabs, 0).layout(Rect::new(0, 0, 80, 1));
    assert_eq!(layout.tabs.len(), 3);
    assert_eq!(layout.closes.len(), 3);
    assert!(!layout.clipped_right);
    let narrow = JBTabs::new(&look, &tabs, 0).layout(Rect::new(0, 0, 30, 1));
    assert!(narrow.clipped_right);
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

fn git() -> GitStatus {
    GitStatus {
        branch: "main".into(),
        added: 3,
        changed: 2,
        removed: 1,
    }
}

#[test]
fn status_bars() {
    for (theme, look) in looks() {
        let git = git();
        let buf = render(110, 1, |area, buf| {
            StatusBar::new(&look)
                .mode(Mode::Normal.label(), Mode::Normal.color(&look.palette))
                .file(Language::Rust, "components.rs")
                .git(&git)
                .center("indexing 62%")
                .diagnostics(Diagnostics {
                    errors: 1,
                    warnings: 2,
                    hints: 0,
                    info: 3,
                })
                .lsp("rust-analyzer")
                .cwd("kit")
                .cursor("Ln 12, Col 8")
                .render(area, buf);
        });
        snapshot(&format!("status-bar.{theme}"), &buf);
    }
}

#[test]
fn a_narrow_status_bar_drops_the_least_important_slots_first() {
    use archipel_kit_tui::components::StatusSlot::{
        self, Center, Cursor, Cwd, File, Git, Lsp, Mode,
    };
    let look = Look::dark();
    let branch = git();
    let slots_at = |width: u16| -> Vec<_> {
        StatusBar::new(&look)
            .mode("NORMAL", look.palette.nord_blue)
            .file(Language::Rust, "components.rs")
            .git(&branch)
            .center("indexing 62%")
            .diagnostics(Diagnostics::default())
            .lsp("rust-analyzer")
            .cwd("kit")
            .cursor("Ln 12, Col 8")
            .slots(Rect::new(0, 0, width, 1))
            .into_iter()
            .map(|(slot, _)| slot)
            .collect()
    };
    let wide = slots_at(160);
    for slot in [
        Mode,
        File,
        Git,
        Center,
        StatusSlot::Diagnostics,
        Lsp,
        Cwd,
        Cursor,
    ] {
        assert!(wide.contains(&slot), "{slot:?} at 160");
    }
    // Medium: the message and the language server go first.
    let medium = slots_at(95);
    assert!(!medium.contains(&Lsp), "{medium:?}");
    for slot in [Mode, File, Cursor, Cwd] {
        assert!(medium.contains(&slot), "{slot:?} stays at 95: {medium:?}");
    }
    // Narrow: only the mode block and the position remain.
    let narrow = slots_at(34);
    assert!(
        narrow.contains(&Mode) && narrow.contains(&Cursor),
        "{narrow:?}"
    );
    assert!(!narrow.contains(&File) || narrow.len() <= 3, "{narrow:?}");
    // No width makes the language server text run into its neighbor.
    for width in 1..=140u16 {
        let buf = render(width, 1, |area, buf| {
            StatusBar::new(&look)
                .mode("NORMAL", look.palette.nord_blue)
                .file(Language::Rust, "components.rs")
                .git(&branch)
                .center("indexing 62%")
                .diagnostics(Diagnostics::default())
                .lsp("rust-analyzer")
                .cwd("kit")
                .cursor("Ln 12, Col 8")
                .render(area, buf);
        });
        let text = lines(&buf).join("");
        assert!(
            !text.contains("rust-analyze ") && !text.contains("rust-analyz "),
            "{width}: {text}"
        );
    }
}

#[test]
fn status_bar_slots_do_not_overlap() {
    let look = Look::dark();
    let git = git();
    for width in [1u16, 10, 20, 40, 80, 120] {
        let bar = StatusBar::new(&look)
            .mode("NORMAL", look.palette.nord_blue)
            .file(Language::Rust, "a.rs")
            .git(&git)
            .center("msg")
            .cursor("1:1");
        let area = Rect::new(0, 0, width, 1);
        let slots = bar.slots(area);
        for (i, (_, a)) in slots.iter().enumerate() {
            for (_, b) in &slots[i + 1..] {
                assert!(a.intersection(*b).is_empty(), "{slots:?} at {width}");
            }
        }
    }
}

#[test]
fn icons() {
    sheet("icons", 72, 10, |look, area, buf| {
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
    // Navy on the dark editor is lifted toward the text color.
    assert_ne!(
        look.readable(Language::Lua.color()),
        Look::rgb(Language::Lua.color())
    );
    // Bright colors are kept.
    assert_eq!(look.readable(0xDEA584), Look::rgb(0xDEA584));
}

#[test]
fn palette_has_distinct_surfaces_in_every_theme() {
    for info in Theme::all() {
        let look = Look::new((info.constructor)());
        let p = &look.palette;
        let surfaces = [p.black, p.black2, p.one_bg, p.one_bg2, p.one_bg3];
        for (i, a) in surfaces.iter().enumerate() {
            for b in &surfaces[i + 1..] {
                assert_ne!(a, b, "{}: surfaces collide: {surfaces:?}", info.id);
            }
        }
    }
}

#[test]
fn widgets_clip_to_tiny_areas() {
    let look = Look::dark();
    let tabs = tabs();
    let git = git();
    for (w, h) in [(0, 0), (1, 1), (3, 1), (5, 2)] {
        render(w.max(1), h.max(1), |_, buf| {
            let area = Rect::new(0, 0, w, h);
            Pane::new(&look).title("A long title").render(area, buf);
            Separator::vertical(&look).render(area, buf);
            Button::primary(&look, "A long label").render(area, buf);
            JBTextField::new(&look, "value")
                .label("Label")
                .error(Some("error"))
                .focused(true)
                .clear_button(true)
                .render(area, buf);
            InlineBanner::new(&look, Status::Error, "text", &["Action"]).render(area, buf);
            ProgressBar::new(&look, 0.5).render(area, buf);
            JBTabs::new(&look, &tabs, 1).render(area, buf);
            StatusBar::new(&look)
                .mode("N", look.palette.nord_blue)
                .file(Language::Rust, "file.rs")
                .git(&git)
                .diagnostics(Diagnostics::default())
                .lsp("x")
                .cwd("y")
                .cursor("z")
                .render(area, buf);
            ColoredListCellRenderer::new(&look, "row")
                .depth(9)
                .icon(Language::Rust)
                .render(area, buf);
        });
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
