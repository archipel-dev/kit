//! The terminal editor, driven with synthetic events: rendering rules, keys, the mouse, find
//! and replace, completion, and a cross-check against the model the GUI editor runs on.

use archipel_kit::Tone;
use archipel_kit::editor::model::{
    ActionContext, Editor, EditorAction, Language, Os as ModelOs, editor_action_for,
};
use archipel_kit::editor::{
    ChangeKind, ChangeMarker, GutterArea, GutterIcon, HighlightSeverity, NavigationKind,
    RangeHighlight,
};
use archipel_kit::ui::{Completion, Completions};
use archipel_kit_tui::editor::{
    CodeHighlighter, CodeLanguage, EditorComponent, EditorOutput, EditorState, GutterClick,
    cursor_position,
};
use archipel_kit_tui::input::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use archipel_kit_tui::ratatui::{buffer::Buffer, layout::Rect, style::Modifier};
use archipel_kit_tui::runtime::CursorShape;
use archipel_kit_tui::testing::Harness;
use archipel_kit_tui::{App, Caps, Event, Glyph, Id, Look, Runtime};

const ID: Id = Id::new("code");

/// The gutter of a short file: sign column (3), two digits, one space.
const TEXT_X: u16 = 6;

fn words(text: &str, caret: usize) -> Option<Completions> {
    const WORDS: &[&str] = &["println", "print", "push", "String", "MAX_LEN", "len"];
    let chars: Vec<char> = text.chars().collect();
    let mut start = caret;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    if start == caret {
        return None;
    }
    let prefix: String = chars[start..caret]
        .iter()
        .collect::<String>()
        .to_lowercase();
    let items: Vec<Completion> = WORDS
        .iter()
        .filter(|w| w.to_lowercase().starts_with(&prefix) && w.len() > prefix.len())
        .map(|w| {
            let c = Completion::new(*w, *w);
            if w.starts_with('p') {
                c.detail("fn")
            } else {
                c
            }
        })
        .collect();
    (!items.is_empty()).then_some(Completions {
        start,
        end: caret,
        items,
    })
}

#[derive(Default, Clone)]
struct Cfg {
    viewer: bool,
    wrap: bool,
    folding: bool,
    whitespace: bool,
    virtual_text: bool,
    gutter_selects: bool,
    completer: bool,
    plain: bool,
}

struct Ed {
    editor: Editor,
    state: EditorState,
    cfg: Cfg,
    hl: CodeHighlighter,
    highlights: Vec<RangeHighlight>,
    icons: Vec<GutterIcon>,
    markers: Vec<ChangeMarker>,
    last: EditorOutput,
    clicks: Vec<GutterClick>,
}

macro_rules! component {
    ($s:expr) => {{
        let Ed {
            editor,
            cfg,
            hl,
            highlights,
            icons,
            markers,
            ..
        } = $s;
        let mut c = EditorComponent::new(ID, editor)
            .highlights(highlights)
            .gutter_icons(icons)
            .change_markers(markers)
            .viewer(cfg.viewer)
            .wrap(cfg.wrap)
            .folding(cfg.folding)
            .show_whitespace(cfg.whitespace)
            .virtual_text(cfg.virtual_text)
            .gutter_click_selects_line(cfg.gutter_selects)
            .keymap_os(ModelOs::WindowsLinux);
        if !cfg.plain {
            c = c.highlighter(hl);
        }
        if cfg.completer {
            c = c.completer(&words);
        }
        c
    }};
}

impl App for Ed {
    fn event(&mut self, rt: &mut Runtime, event: &Event) {
        let mut state = std::mem::take(&mut self.state);
        let out = component!(self).handle(rt, &mut state, event);
        self.state = state;
        self.clicks.extend(out.gutter_clicks.iter().copied());
        self.last = out;
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let mut state = std::mem::take(&mut self.state);
        component!(self).show(rt, &mut state, area, buf);
        self.state = state;
    }

    fn tick(&mut self, rt: &mut Runtime) {
        let mut state = std::mem::take(&mut self.state);
        component!(self).tick(rt, &mut state);
        self.state = state;
    }
}

fn runtime(look: Look) -> Runtime {
    let caps = Caps {
        kitty: false,
        os: archipel_kit::action_system::Os::Linux,
        ..Caps::legacy()
    };
    let mut rt = Runtime::with(
        look,
        archipel_kit_tui::input::terminal_keymap(caps.os),
        caps,
    );
    rt.focus.request(ID);
    rt
}

fn app(text: &str, cfg: Cfg) -> Ed {
    Ed {
        editor: Editor::new(text),
        state: EditorState::new(),
        cfg,
        hl: CodeHighlighter::new(CodeLanguage::Rust),
        highlights: Vec::new(),
        icons: Vec::new(),
        markers: Vec::new(),
        last: EditorOutput::default(),
        clicks: Vec::new(),
    }
}

fn harness_with(ed: Ed, look: Look, w: u16, h: u16) -> Harness<Ed> {
    let mut harness = Harness::with_runtime(ed, runtime(look), w, h);
    harness.draw();
    harness
}

fn harness(text: &str, cfg: Cfg) -> Harness<Ed> {
    harness_with(app(text, cfg), Look::dark(), 48, 10)
}

const CODE: &str =
    "fn main() {\n    let total = 42;\n    if total > 1 {\n        println!(\"big\");\n    }\n}\n";

fn text(h: &Harness<Ed>) -> &str {
    h.app.editor.document.text()
}

fn caret(h: &Harness<Ed>) -> usize {
    h.app.editor.carets.primary().offset
}

fn selection(h: &Harness<Ed>) -> Option<(usize, usize)> {
    let c = h.app.editor.carets.primary();
    c.has_selection()
        .then(|| (c.selection_start(), c.selection_end()))
}

fn key(h: &mut Harness<Ed>, code: KeyCode, mods: KeyModifiers) {
    h.press_with(code, mods);
}

fn mouse(h: &mut Harness<Ed>, kind: MouseEventKind, x: u16, y: u16, mods: KeyModifiers) {
    h.send(Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: mods,
    }));
}

fn cell(h: &Harness<Ed>, x: u16, y: u16) -> &archipel_kit_tui::ratatui::buffer::Cell {
    &h.buffer()[(x, y)]
}

// ---- rendering -------------------------------------------------------------------------

#[test]
fn the_current_line_number_is_bright_and_bold_the_others_grey() {
    let mut h = harness(CODE, Cfg::default());
    key(&mut h, KeyCode::Down, KeyModifiers::NONE);
    let p = h.rt.look.palette;
    let (num, other) = (cell(&h, 4, 1), cell(&h, 4, 0));
    assert_eq!(num.symbol(), "2");
    assert_eq!(num.fg, p.white);
    assert!(num.modifier.contains(Modifier::BOLD));
    assert_eq!(other.fg, p.grey_fg);
}

#[test]
fn the_cursor_line_is_on_black2_and_the_rest_on_black() {
    let h = harness(CODE, Cfg::default());
    let p = h.rt.look.palette;
    assert_eq!(cell(&h, 30, 0).bg, p.black2);
    assert_eq!(cell(&h, 30, 1).bg, p.black);
    assert_eq!(cell(&h, 2, 0).bg, p.black2, "the gutter row too");
}

#[test]
fn the_primary_caret_is_the_terminal_cursor_a_bar_and_a_block_when_overwriting() {
    let mut h = harness(CODE, Cfg::default());
    key(&mut h, KeyCode::Right, KeyModifiers::NONE);
    let cursor = h.rt.cursor.expect("cursor");
    assert_eq!(
        (cursor.x, cursor.y, cursor.shape),
        (TEXT_X + 1, 0, CursorShape::Bar)
    );
    key(&mut h, KeyCode::Insert, KeyModifiers::NONE);
    assert_eq!(h.rt.cursor.map(|c| c.shape), Some(CursorShape::Block));
    h.rt.focus.clear();
    h.draw();
    assert_eq!(h.rt.cursor, None, "no cursor without focus");
}

#[test]
fn syntax_colors_come_from_the_theme_editor_scheme() {
    let h = harness(CODE, Cfg::default());
    let look = &h.rt.look;
    let keyword = look.color(look.theme.editor.keyword);
    assert_eq!(cell(&h, TEXT_X, 0).symbol(), "f");
    assert_eq!(cell(&h, TEXT_X, 0).fg, keyword);
    let number = look.color(look.theme.editor.number);
    // `42` on line 2.
    assert_eq!(cell(&h, TEXT_X + 16, 1).symbol(), "4");
    assert_eq!(cell(&h, TEXT_X + 16, 1).fg, number);
}

#[test]
fn indent_guides_are_thin_lines_in_the_line_color() {
    let h = harness(CODE, Cfg::default());
    let p = h.rt.look.palette;
    assert_eq!(cell(&h, TEXT_X, 1).symbol(), "│");
    assert_eq!(cell(&h, TEXT_X, 3).symbol(), "│");
    assert_eq!(cell(&h, TEXT_X + 4, 3).symbol(), "│");
    // The caret is on the header of the body: its guide is the selected one.
    let selected =
        h.rt.look
            .color(h.rt.look.theme.editor.indent_guide_selected);
    assert_eq!(cell(&h, TEXT_X, 3).fg, selected);
    assert_eq!(cell(&h, TEXT_X + 4, 3).fg, p.line);
}

#[test]
fn the_selection_is_a_cell_background_in_the_visual_color() {
    let mut h = harness(CODE, Cfg::default());
    key(&mut h, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut h, KeyCode::Right, KeyModifiers::SHIFT);
    let p = h.rt.look.palette;
    assert_eq!(cell(&h, TEXT_X, 0).bg, p.one_bg3);
    assert_eq!(cell(&h, TEXT_X + 1, 0).bg, p.one_bg3);
    assert_ne!(cell(&h, TEXT_X + 2, 0).bg, p.one_bg3);
}

#[test]
fn secondary_carets_are_reversed_cells() {
    let mut h = harness(CODE, Cfg::default());
    // Ctrl+K, Ctrl+Down: clone caret below (the fallback of the double-Ctrl gesture).
    h.ctrl('k');
    key(&mut h, KeyCode::Down, KeyModifiers::CONTROL);
    assert_eq!(h.app.editor.carets.caret_count(), 2);
    let primary = h.app.editor.carets.primary().offset;
    assert!(primary > 10, "the clone below is the primary");
    assert!(cell(&h, TEXT_X, 0).modifier.contains(Modifier::REVERSED));
    assert_eq!(h.rt.cursor.map(|c| (c.x, c.y)), Some((TEXT_X, 1)));
}

#[test]
fn whitespace_shows_dots_and_arrows() {
    let mut ed = app(
        "a b\n\tc",
        Cfg {
            whitespace: true,
            ..Cfg::default()
        },
    );
    ed.editor.settings.tab_size = 4;
    let h = harness_with(ed, Look::dark(), 30, 4);
    assert_eq!(cell(&h, TEXT_X + 1, 0).symbol(), "·");
    assert_eq!(cell(&h, TEXT_X, 1).symbol(), "→");
    assert_eq!(cell(&h, TEXT_X + 4, 1).symbol(), "c");
}

#[test]
fn the_right_margin_column_is_tinted() {
    let mut ed = app("abc\n", Cfg::default());
    ed.editor.settings.right_margin = 10;
    let h = harness_with(ed, Look::dark(), 30, 4);
    let p = h.rt.look.palette;
    assert_eq!(cell(&h, TEXT_X + 10, 1).bg, p.one_bg);
    assert_eq!(cell(&h, TEXT_X + 10, 0).bg, p.one_bg2, "on the cursor line");
    assert_eq!(cell(&h, TEXT_X + 9, 1).bg, p.black);
}

#[test]
fn the_matched_brace_and_its_partner_are_highlighted() {
    let mut h = harness(CODE, Cfg::default());
    key(&mut h, KeyCode::End, KeyModifiers::NONE);
    let brace = h.rt.look.color(h.rt.look.theme.editor.matched_brace);
    assert_eq!(cell(&h, TEXT_X + 10, 0).bg, brace);
    assert_eq!(cell(&h, TEXT_X, 5).symbol(), "}");
    assert_eq!(cell(&h, TEXT_X, 5).bg, brace);
}

#[test]
fn the_occurrences_of_the_selected_word_are_highlighted() {
    let mut h = harness(CODE, Cfg::default());
    key(&mut h, KeyCode::Down, KeyModifiers::NONE);
    // Select `total` with a double click.
    h.double_click(TEXT_X + 9, 1);
    assert_eq!(selection(&h), Some((20, 25)));
    let occurrence = h.rt.look.color(h.rt.look.theme.editor.rename_occurrence);
    assert_eq!(cell(&h, TEXT_X + 7, 2).symbol(), "t");
    assert_eq!(cell(&h, TEXT_X + 7, 2).bg, occurrence);
}

#[test]
fn diagnostics_are_underlined_in_the_severity_color_and_drawn_as_undercurl() {
    let mut ed = app(CODE, Cfg::default());
    ed.highlights = vec![
        RangeHighlight::new(20..25, HighlightSeverity::Error).message("unused variable"),
        RangeHighlight::new(3..7, HighlightSeverity::Warning),
    ];
    let h = harness_with(ed, Look::dark(), 48, 10);
    let ed_colors = h.rt.look.theme.editor;
    let c = cell(&h, TEXT_X + 8, 1);
    assert!(c.modifier.contains(Modifier::UNDERLINED));
    assert_eq!(c.underline_color, h.rt.look.color(ed_colors.error_effect));
    assert_eq!(
        cell(&h, TEXT_X + 3, 0).underline_color,
        h.rt.look.color(ed_colors.warning_effect)
    );
    assert!(
        !cell(&h, TEXT_X + 13, 1)
            .modifier
            .contains(Modifier::UNDERLINED)
    );
    let curls = archipel_kit_tui::runtime::undercurl_sequences(h.buffer());
    assert!(curls.contains("\x1b[4:3m\x1b[58:2::"), "{curls:?}");
    // The scrollbar column has a mark for each.
    let stripe = 47;
    assert_eq!(cell(&h, stripe, 0).symbol(), "▐");
    assert_eq!(
        cell(&h, stripe, 1).fg,
        h.rt.look.color(ed_colors.error_stripe)
    );
}

#[test]
fn virtual_text_shows_the_message_after_the_line() {
    let mut ed = app(
        CODE,
        Cfg {
            virtual_text: true,
            ..Cfg::default()
        },
    );
    ed.highlights = vec![RangeHighlight::new(20..25, HighlightSeverity::Error).message("unused")];
    let h = harness_with(ed, Look::dark(), 48, 10);
    assert!(h.screen()[1].contains("■ unused"), "{}", h.text());
}

#[test]
fn the_sign_column_shows_change_markers_and_icons() {
    let mut ed = app(CODE, Cfg::default());
    ed.markers = vec![
        ChangeMarker::new(1..2, ChangeKind::Added),
        ChangeMarker::new(2..3, ChangeKind::Modified),
        ChangeMarker::new(4..4, ChangeKind::Deleted),
    ];
    ed.icons = vec![GutterIcon::new(3, Glyph::Breakpoint, "Breakpoint").tone(Tone::Error)];
    let h = harness_with(ed, Look::dark(), 48, 10);
    let p = h.rt.look.palette;
    assert_eq!((cell(&h, 0, 1).symbol(), cell(&h, 0, 1).fg), ("▎", p.green));
    assert_eq!((cell(&h, 0, 2).symbol(), cell(&h, 0, 2).fg), ("▎", p.blue));
    assert_eq!(cell(&h, 0, 4).symbol(), "▔");
    assert_eq!(cell(&h, 1, 3).fg, p.red);
    assert_eq!(
        cell(&h, 1, 3).symbol(),
        archipel_kit_tui::glyphs::nerd_font(Glyph::Breakpoint).to_string()
    );
}

#[test]
fn soft_wrap_breaks_long_lines_into_rows() {
    let line = "word ".repeat(20);
    let h = harness_with(
        app(
            &format!("{line}\nnext"),
            Cfg {
                wrap: true,
                ..Cfg::default()
            },
        ),
        Look::dark(),
        30,
        10,
    );
    let screen = h.screen();
    // 24 text cells per row: 100 characters take 5 rows; the number shows on the first.
    assert!(screen[0].starts_with("    1 word word"), "{screen:?}");
    assert!(screen[1].starts_with("       word"), "{screen:?}");
    assert_eq!(screen[1].len(), 30, "{screen:?}");
    assert!(screen[5].starts_with("    2 next"), "{screen:?}");
}

fn folded() -> Ed {
    let mut ed = app(
        CODE,
        Cfg {
            folding: true,
            ..Cfg::default()
        },
    );
    let doc = &ed.editor.document;
    // The body of `if`, collapsed.
    let open = CODE.find("1 {").expect("in the sample") + 2;
    let close = CODE.rfind("    }").expect("in the sample") + 5;
    ed.editor
        .folding
        .add_region(doc, open, close, "{...}", false);
    ed.editor
        .folding
        .add_region(doc, 10, CODE.len() - 1, "{...}", true);
    ed
}

#[test]
fn a_collapsed_region_is_a_pill_and_a_click_expands_it() {
    let mut h = harness_with(folded(), Look::dark(), 48, 10);
    let screen = h.screen();
    assert!(screen[2].contains("if total > 1 {…}"), "{screen:?}");
    assert!(
        screen[3].contains('}'),
        "the line after the region: {screen:?}"
    );
    let (x, y) = h.find("{…}").expect("pill");
    assert_eq!(cell(&h, x, y).bg, h.rt.look.palette.one_bg2);
    h.click(x, y);
    assert!(h.screen()[3].contains("println!"), "{}", h.text());
}

#[test]
fn a_click_on_a_fold_marker_toggles_the_region() {
    let mut h = harness_with(folded(), Look::dark(), 48, 10);
    // Gutter: sign 3, digits 2, space 1, fold 2.
    let fold_x = 6;
    let chevron = archipel_kit_tui::glyphs::nerd_font(Glyph::ChevronRight).to_string();
    assert_eq!(cell(&h, fold_x, 2).symbol(), chevron);
    h.click(fold_x, 2);
    assert!(h.text().contains("println!"));
    h.click(fold_x, 0);
    assert_eq!(h.screen().len(), 10);
    assert!(h.screen()[0].ends_with("fn main() {…}"), "{}", h.text());
}

// ---- keys --------------------------------------------------------------------------------

#[test]
fn typing_enter_backspace_undo_and_redo_go_through_the_model() {
    let mut h = harness(
        "",
        Cfg {
            plain: true,
            ..Cfg::default()
        },
    );
    h.type_text("ab(");
    assert_eq!(text(&h), "ab()", "auto-paired by the model");
    key(&mut h, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(text(&h), "ab");
    key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
    h.type_text("c");
    assert_eq!(text(&h), "ab\nc");
    h.ctrl('z');
    assert_eq!(text(&h), "ab\n");
    // Ctrl+Shift+Z is Ctrl+Z in a legacy terminal: Ctrl+K, Z redoes.
    h.ctrl('k');
    h.type_text("z");
    assert_eq!(text(&h), "ab\nc");
    assert_eq!(cursor_position(&h.app.editor), (2, 2));
}

#[test]
fn keys_produce_the_same_model_state_as_the_gui_keymap() {
    let keys: Vec<(
        KeyCode,
        KeyModifiers,
        archipel_kit::egui::Key,
        archipel_kit::editor::model::KeyModifiers,
    )> = {
        use archipel_kit::editor::model::KeyModifiers as M;
        use archipel_kit::egui::Key as K;
        vec![
            (KeyCode::Down, KeyModifiers::NONE, K::ArrowDown, M::NONE),
            (
                KeyCode::Right,
                KeyModifiers::CONTROL,
                K::ArrowRight,
                M::CTRL,
            ),
            (
                KeyCode::Right,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                K::ArrowRight,
                M::CTRL.plus_shift(),
            ),
            (KeyCode::Char('d'), KeyModifiers::CONTROL, K::D, M::CTRL),
            (
                KeyCode::Up,
                KeyModifiers::ALT | KeyModifiers::SHIFT,
                K::ArrowUp,
                M::ALT.plus_shift(),
            ),
            (KeyCode::Char('j'), KeyModifiers::ALT, K::J, M::ALT),
            (KeyCode::Char('j'), KeyModifiers::ALT, K::J, M::ALT),
            (KeyCode::End, KeyModifiers::SHIFT, K::End, M::SHIFT),
            (KeyCode::Tab, KeyModifiers::NONE, K::Tab, M::NONE),
            (KeyCode::Char('y'), KeyModifiers::CONTROL, K::Y, M::CTRL),
            (KeyCode::Char('z'), KeyModifiers::CONTROL, K::Z, M::CTRL),
            (KeyCode::Home, KeyModifiers::CONTROL, K::Home, M::CTRL),
            (KeyCode::Char('/'), KeyModifiers::CONTROL, K::Slash, M::CTRL),
        ]
    };
    let mut h = harness(CODE, Cfg::default());
    let mut model = Editor::new(CODE);
    let language = Language::c_like();
    for (code, mods, gui_key, gui_mods) in keys {
        key(&mut h, code, mods);
        let action = editor_action_for(gui_key, gui_mods, ModelOs::WindowsLinux)
            .unwrap_or_else(|| panic!("{gui_key:?} is bound"));
        let ctx = ActionContext::new(&language).with_viewport(0, 10);
        model.execute(&action, &ctx);
        assert_eq!(text(&h), model.document.text(), "after {action:?}");
        assert_eq!(
            h.app.editor.carets.snapshot(),
            model.carets.snapshot(),
            "after {action:?}"
        );
    }
    // Typed text is EditorAction::Type, character by character.
    h.type_text("x(");
    let ctx = ActionContext::new(&language);
    model.execute(&EditorAction::Type('x'), &ctx);
    model.execute(&EditorAction::Type('('), &ctx);
    assert_eq!(text(&h), model.document.text());
}

#[test]
fn copy_publishes_osc52_and_a_line_copied_without_selection_pastes_above() {
    let mut h = harness(
        "one\ntwo\n",
        Cfg {
            plain: true,
            ..Cfg::default()
        },
    );
    h.ctrl('c');
    assert_eq!(h.rt.clipboard.get(), "one\n");
    let sequence = h.rt.clipboard.take_sequence().expect("osc 52");
    assert!(sequence.starts_with("\x1b]52;c;"));
    key(&mut h, KeyCode::Right, KeyModifiers::NONE);
    // The terminal's own paste of the same text keeps the whole-line rule: the line goes in
    // whole, not into the middle of the caret line.
    h.send(Event::Paste("one\n".to_owned()));
    assert!(
        text(&h).lines().all(|l| l == "one" || l == "two"),
        "{:?}",
        text(&h)
    );
    assert_eq!(text(&h).matches("one").count(), 2);
    h.send(Event::Paste("X".to_owned()));
    assert!(text(&h).contains('X') && text(&h).lines().any(|l| l.len() == 4));
}

#[test]
fn ctrl_v_pastes_the_clipboard() {
    let mut h = harness(
        "ab",
        Cfg {
            plain: true,
            ..Cfg::default()
        },
    );
    h.rt.clipboard.set("Z");
    h.ctrl('v');
    assert_eq!(text(&h), "Zab");
}

#[test]
fn the_fallback_chords_run_editor_actions() {
    let mut h = harness(
        "a\nb\nc",
        Cfg {
            plain: true,
            ..Cfg::default()
        },
    );
    key(&mut h, KeyCode::Down, KeyModifiers::NONE);
    // Ctrl+K, Up: Move Line Up (Alt+Shift+Up).
    h.ctrl('k');
    assert!(h.text().contains("Move Line Up") || h.rt.awaiting_second_stroke());
    key(&mut h, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(text(&h), "b\na\nc");
    assert_eq!(
        caret(&h),
        0,
        "the Up of the chord did not also move the caret"
    );
    // Ctrl+K, E: Extend Selection (Ctrl+W is the window leader).
    h.ctrl('k');
    h.type_text("e");
    assert_eq!(selection(&h), Some((0, 1)));
    // Alt+Up extends too, Alt+Down shrinks.
    key(&mut h, KeyCode::Up, KeyModifiers::ALT);
    assert!(selection(&h).is_some_and(|(s, e)| e - s > 1));
    key(&mut h, KeyCode::Down, KeyModifiers::ALT);
    assert_eq!(selection(&h), Some((0, 1)));
    // Ctrl+/ arrives as Ctrl+7.
    let mut h = harness("x", Cfg::default());
    key(&mut h, KeyCode::Char('7'), KeyModifiers::CONTROL);
    assert_eq!(text(&h), "//x");
}

#[test]
fn a_viewer_moves_and_selects_but_does_not_edit() {
    let mut h = harness(
        "abc",
        Cfg {
            viewer: true,
            ..Cfg::default()
        },
    );
    h.type_text("x");
    key(&mut h, KeyCode::Backspace, KeyModifiers::NONE);
    h.send(Event::Paste("p".to_owned()));
    assert_eq!(text(&h), "abc");
    key(&mut h, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(selection(&h), Some((0, 1)));
    h.ctrl('c');
    assert_eq!(h.rt.clipboard.get(), "a");
}

#[test]
fn escape_falls_through_when_there_is_nothing_to_clear() {
    let mut h = harness("abc", Cfg::default());
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!h.app.last.consumed);
    key(&mut h, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    assert!(h.app.last.consumed);
    assert_eq!(selection(&h), None);
}

#[test]
fn page_down_and_the_caret_keep_the_view_following() {
    let long: String = (0..100).map(|i| format!("line {i}\n")).collect();
    let mut h = harness(&long, Cfg::default());
    key(&mut h, KeyCode::PageDown, KeyModifiers::NONE);
    key(&mut h, KeyCode::PageDown, KeyModifiers::NONE);
    let line = cursor_position(&h.app.editor).0;
    assert!(line > 10);
    assert!(h.text().contains(&format!("line {}", line - 1)));
    key(&mut h, KeyCode::End, KeyModifiers::CONTROL);
    assert!(h.text().contains("line 99"));
    assert!(h.rt.cursor.is_some(), "the caret is in view");
}

// ---- mouse -------------------------------------------------------------------------------

#[test]
fn a_click_places_the_caret_double_selects_a_word_triple_a_line() {
    let mut h = harness(CODE, Cfg::default());
    h.click(TEXT_X + 8, 1);
    assert_eq!(caret(&h), 12 + 8);
    h.double_click(TEXT_X + 9, 1);
    assert_eq!(selection(&h), Some((20, 25)));
    h.click(TEXT_X + 9, 1);
    assert_eq!(
        selection(&h),
        Some((12, 32)),
        "the third click selects the line"
    );
}

#[test]
fn a_drag_selects_and_shift_click_extends() {
    let mut h = harness(CODE, Cfg::default());
    h.drag((TEXT_X, 0), (TEXT_X + 2, 1));
    assert_eq!(selection(&h), Some((0, 14)));
    h.advance(1.0);
    h.click_with(TEXT_X + 4, 2, KeyModifiers::SHIFT);
    assert_eq!(selection(&h), Some((0, 36)));
}

#[test]
fn alt_shift_click_adds_a_caret_and_removes_it_again() {
    let mut h = harness(CODE, Cfg::default());
    let mods = KeyModifiers::ALT | KeyModifiers::SHIFT;
    h.click_with(TEXT_X + 4, 1, mods);
    assert_eq!(h.app.editor.carets.caret_count(), 2);
    h.advance(1.0);
    h.click_with(TEXT_X + 4, 1, mods);
    assert_eq!(h.app.editor.carets.caret_count(), 1);
}

#[test]
fn alt_drag_selects_a_column() {
    let mut h = harness(CODE, Cfg::default());
    mouse(
        &mut h,
        MouseEventKind::Down(MouseButton::Left),
        TEXT_X + 4,
        1,
        KeyModifiers::ALT,
    );
    mouse(
        &mut h,
        MouseEventKind::Drag(MouseButton::Left),
        TEXT_X + 7,
        3,
        KeyModifiers::ALT,
    );
    mouse(
        &mut h,
        MouseEventKind::Up(MouseButton::Left),
        TEXT_X + 7,
        3,
        KeyModifiers::ALT,
    );
    assert_eq!(h.app.editor.carets.caret_count(), 3);
    assert_eq!(
        h.app.editor.carets.selected_texts(&h.app.editor.document),
        ["let", "if ", "   "]
    );
    let p = h.rt.look.palette;
    assert_eq!(cell(&h, TEXT_X + 5, 2).bg, p.one_bg3);
}

#[test]
fn ctrl_click_asks_for_navigation() {
    let mut h = harness(CODE, Cfg::default());
    mouse(
        &mut h,
        MouseEventKind::Down(MouseButton::Left),
        TEXT_X + 8,
        1,
        KeyModifiers::CONTROL,
    );
    let nav = h.app.last.navigation.expect("navigation");
    assert_eq!((nav.offset, nav.kind), (20, NavigationKind::Declaration));
}

#[test]
fn the_wheel_scrolls_three_rows() {
    let long: String = (0..100).map(|i| format!("line {i}\n")).collect();
    let mut h = harness(&long, Cfg::default());
    h.scroll(TEXT_X, 3, 2);
    assert_eq!(h.app.state.first_row(), 6);
    assert!(h.screen()[0].contains("line 6"));
}

#[test]
fn a_drag_beyond_the_bottom_autoscrolls_on_the_timer() {
    let long: String = (0..100).map(|i| format!("line {i}\n")).collect();
    let mut h = harness(&long, Cfg::default());
    mouse(
        &mut h,
        MouseEventKind::Down(MouseButton::Left),
        TEXT_X,
        0,
        KeyModifiers::NONE,
    );
    mouse(
        &mut h,
        MouseEventKind::Drag(MouseButton::Left),
        TEXT_X,
        9,
        KeyModifiers::NONE,
    );
    mouse(
        &mut h,
        MouseEventKind::Drag(MouseButton::Left),
        TEXT_X,
        12,
        KeyModifiers::NONE,
    );
    let before = h.app.state.first_row();
    assert!(h.app.state.deadline().is_some());
    for _ in 0..10 {
        h.advance(0.1);
    }
    assert!(
        h.app.state.first_row() > before + 5,
        "{}",
        h.app.state.first_row()
    );
    mouse(
        &mut h,
        MouseEventKind::Up(MouseButton::Left),
        TEXT_X,
        12,
        KeyModifiers::NONE,
    );
    assert!(h.app.state.deadline().is_none());
    assert!(selection(&h).is_some_and(|(s, e)| s == 0 && e > 100));
}

#[test]
fn a_click_on_a_line_number_reports_it_a_drag_selects_lines() {
    let mut h = harness(CODE, Cfg::default());
    h.click(4, 2);
    assert_eq!(
        h.app.clicks.last().map(|c| (c.line, c.area)),
        Some((2, GutterArea::LineNumber))
    );
    assert_eq!(selection(&h), None, "the new UI: the click is the caller's");
    h.drag((4, 1), (4, 2));
    assert_eq!(selection(&h), Some((12, 51)));
}

#[test]
fn with_the_classic_rule_a_click_on_a_line_number_selects_the_line() {
    let mut h = harness(
        CODE,
        Cfg {
            gutter_selects: true,
            ..Cfg::default()
        },
    );
    h.click(4, 1);
    assert_eq!(selection(&h), Some((12, 32)));
    h.advance(1.0);
    h.click_with(4, 3, KeyModifiers::SHIFT);
    assert_eq!(selection(&h), Some((12, 76)));
}

#[test]
fn a_click_on_a_gutter_icon_is_reported() {
    let mut ed = app(CODE, Cfg::default());
    ed.icons = vec![GutterIcon::new(1, Glyph::Bookmark, "Bookmark")];
    let mut h = harness_with(ed, Look::dark(), 48, 10);
    h.click(1, 1);
    assert_eq!(
        h.app.clicks.last().map(|c| (c.line, c.area)),
        Some((1, GutterArea::Icon(0)))
    );
}

#[test]
fn right_press_moves_the_caret_unless_on_the_selection_and_asks_for_a_menu() {
    let mut h = harness(CODE, Cfg::default());
    h.right_click(TEXT_X + 3, 1);
    assert_eq!(caret(&h), 15);
    assert_eq!(h.app.last.context_menu, None, "reported on the press");
    mouse(
        &mut h,
        MouseEventKind::Down(MouseButton::Right),
        TEXT_X + 3,
        2,
        KeyModifiers::NONE,
    );
    assert_eq!(h.app.last.context_menu, Some((TEXT_X + 3, 2)));
}

// ---- find and replace ----------------------------------------------------------------------

#[test]
fn ctrl_f_seeds_the_query_typing_selects_the_first_match_and_shows_n_of_m() {
    let mut h = harness(CODE, Cfg::default());
    h.ctrl('f');
    assert!(h.app.state.find_open());
    assert!(h.screen()[0].contains("Find"));
    h.type_text("total");
    assert_eq!(selection(&h), Some((20, 25)));
    assert!(h.screen()[0].contains("1/2"), "{}", h.text());
    key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(selection(&h), Some((39, 44)));
    assert!(h.screen()[0].contains("2/2"));
    // Past the last match: a hint first, the second Enter wraps.
    key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(selection(&h), Some((39, 44)));
    let hint = h.app.last.hint.clone().unwrap_or_default();
    assert_eq!(hint, "\"total\" not found, press F3 to search from the top");
    assert!(h.text().contains("\"total\" not found"), "{}", h.text());
    key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(selection(&h), Some((20, 25)));
    // Up in the query: previous.
    key(&mut h, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(selection(&h), Some((20, 25)));
    // A selection seeds the query.
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!h.app.state.find_open());
    h.double_click(TEXT_X, 0);
    h.ctrl('f');
    assert!(h.screen()[0].contains("fn"), "{}", h.text());
}

#[test]
fn match_case_words_and_regex_toggle_with_alt() {
    let mut h = harness("Ab ab abc", Cfg::default());
    h.ctrl('f');
    h.type_text("ab");
    assert!(h.text().contains("1/3"), "{}", h.text());
    h.alt('c');
    assert!(h.text().contains("1/2"), "{}", h.text());
    h.alt('w');
    assert!(h.text().contains("1/1"), "{}", h.text());
    h.alt('w');
    h.alt('c');
    h.alt('x');
    key(&mut h, KeyCode::Backspace, KeyModifiers::NONE);
    key(&mut h, KeyCode::Backspace, KeyModifiers::NONE);
    h.type_text("a.c");
    assert!(h.text().contains("1/1"), "{}", h.text());
    h.type_text("(");
    assert!(h.text().contains("Bad pattern"), "{}", h.text());
}

#[test]
fn f3_steps_from_the_editor_and_escape_closes_the_bar() {
    let mut h = harness(CODE, Cfg::default());
    h.ctrl('f');
    h.type_text("total");
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!h.app.state.find_open());
    key(&mut h, KeyCode::F(3), KeyModifiers::NONE);
    assert_eq!(selection(&h), Some((39, 44)));
    key(&mut h, KeyCode::F(3), KeyModifiers::SHIFT);
    assert_eq!(selection(&h), Some((20, 25)));
}

#[test]
fn replace_and_replace_all_are_one_undo_step_each() {
    let mut h = harness("a b a b a", Cfg::default());
    h.ctrl('r');
    h.type_text("a");
    key(&mut h, KeyCode::Tab, KeyModifiers::NONE);
    h.type_text("x");
    key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(text(&h), "x b a b a");
    assert_eq!(selection(&h), Some((4, 5)), "the next match is selected");
    h.alt('a');
    assert_eq!(text(&h), "x b x b x");
    assert!(h.text().contains("2 occurrences replaced"), "{}", h.text());
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    h.ctrl('z');
    assert_eq!(text(&h), "x b a b a");
    h.ctrl('z');
    assert_eq!(text(&h), "a b a b a");
}

#[test]
fn alt_enter_selects_all_occurrences_and_the_history_remembers_queries() {
    let mut h = harness("ab ab ab", Cfg::default());
    h.ctrl('f');
    h.type_text("ab");
    key(&mut h, KeyCode::Enter, KeyModifiers::ALT);
    assert_eq!(h.app.editor.carets.caret_count(), 3);
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(h.app.state.history.queries(), ["ab"]);
    h.ctrl('f');
    key(&mut h, KeyCode::Backspace, KeyModifiers::NONE);
    key(&mut h, KeyCode::Down, KeyModifiers::ALT);
    assert!(h.screen()[1].contains("ab"), "{}", h.text());
    key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
    assert!(h.screen()[0].contains("ab"));
}

#[test]
fn the_bar_buttons_take_clicks() {
    let mut h = harness_with(app("Ab ab", Cfg::default()), Look::dark(), 60, 6);
    h.ctrl('f');
    h.type_text("ab");
    let (x, y) = h.find(" Cc ").expect("match case chip");
    h.click(x + 1, y);
    assert!(h.text().contains("1/1"), "{}", h.text());
    let (x, y) = h.find("↓").expect("next");
    h.click(x, y);
    let close = archipel_kit_tui::glyphs::nerd_font(Glyph::Close).to_string();
    let (x, y) = h.find(&close).expect("close");
    h.click(x, y);
    assert!(!h.app.state.find_open());
}

// ---- completion ----------------------------------------------------------------------------

#[test]
fn typing_opens_the_completion_menu_and_the_lookup_keys_work() {
    let mut h = harness(
        "",
        Cfg {
            completer: true,
            ..Cfg::default()
        },
    );
    h.type_text("p");
    assert!(h.app.state.completion_open());
    let screen = h.text();
    assert!(
        screen.contains("println") && screen.contains("push"),
        "{screen}"
    );
    // Down, Down, Down wraps to the first.
    key(&mut h, KeyCode::Down, KeyModifiers::NONE);
    key(&mut h, KeyCode::Down, KeyModifiers::NONE);
    key(&mut h, KeyCode::Down, KeyModifiers::NONE);
    key(&mut h, KeyCode::Up, KeyModifiers::NONE);
    key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(text(&h), "push");
    assert!(!h.app.state.completion_open());
    // Tab replaces the identifier after the caret.
    h.press(KeyCode::Home);
    h.type_text("pr");
    assert_eq!(text(&h), "prpush");
    key(&mut h, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(text(&h), "println");
    // Backspace only narrows an open list; it does not open one. Escape closes a list;
    // Ctrl+Space asks again, and a lone answer goes in at once as one undo step.
    key(&mut h, KeyCode::Backspace, KeyModifiers::NONE);
    assert!(!h.app.state.completion_open());
    key(&mut h, KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert_eq!(text(&h), "println");
    h.ctrl('z');
    assert_eq!(text(&h), "printl");
    h.press(KeyCode::End);
    for _ in 0..3 {
        key(&mut h, KeyCode::Backspace, KeyModifiers::NONE);
    }
    h.type_text("n");
    assert_eq!(text(&h), "prin");
    assert!(h.app.state.completion_open());
    key(&mut h, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!h.app.state.completion_open());
    key(&mut h, KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert!(h.app.state.completion_open(), "Ctrl+Space asks again");
}

#[test]
fn the_matched_characters_are_highlighted_and_a_click_picks() {
    let mut h = harness(
        "",
        Cfg {
            completer: true,
            ..Cfg::default()
        },
    );
    h.type_text("le");
    let (x, y) = h.find("len").expect("menu row");
    assert!(cell(&h, x, y).modifier.contains(Modifier::BOLD));
    assert!(!cell(&h, x + 2, y).modifier.contains(Modifier::BOLD));
    h.click(x, y);
    assert_eq!(text(&h), "len");
}

// ---- robustness ----------------------------------------------------------------------------

#[test]
fn no_panic_on_tiny_sizes_and_random_input() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let codes = [
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Enter,
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Esc,
        KeyCode::Insert,
        KeyCode::F(3),
        KeyCode::Char('a'),
        KeyCode::Char('{'),
        KeyCode::Char('"'),
        KeyCode::Char('é'),
        KeyCode::Char('漢'),
        KeyCode::Char('\t'),
        KeyCode::Char('f'),
        KeyCode::Char('r'),
        KeyCode::Char('z'),
        KeyCode::Char('k'),
        KeyCode::Char('j'),
        KeyCode::Char(' '),
        KeyCode::Char('+'),
        KeyCode::Char('-'),
    ];
    let mods = [
        KeyModifiers::NONE,
        KeyModifiers::SHIFT,
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ];
    for (w, hgt) in [(1, 1), (2, 1), (5, 2), (8, 3), (13, 4), (40, 8)] {
        for cfg in [
            Cfg::default(),
            Cfg {
                wrap: true,
                folding: true,
                whitespace: true,
                completer: true,
                ..Cfg::default()
            },
            Cfg {
                viewer: true,
                virtual_text: true,
                ..Cfg::default()
            },
        ] {
            let mut ed = folded();
            ed.cfg = Cfg {
                folding: true,
                ..cfg
            };
            ed.highlights =
                vec![RangeHighlight::new(2..400, HighlightSeverity::Error).message("x")];
            let mut h = harness_with(ed, Look::dark(), w, hgt);
            for _ in 0..300 {
                let r = next();
                match r % 10 {
                    0..=6 => {
                        let code = codes[(r >> 8) as usize % codes.len()];
                        let m = mods[(r >> 16) as usize % mods.len()];
                        h.send(Event::Key(KeyEvent::new(code, m)));
                    }
                    7 => {
                        let kinds = [
                            MouseEventKind::Down(MouseButton::Left),
                            MouseEventKind::Drag(MouseButton::Left),
                            MouseEventKind::Up(MouseButton::Left),
                            MouseEventKind::Down(MouseButton::Middle),
                            MouseEventKind::Up(MouseButton::Middle),
                            MouseEventKind::ScrollDown,
                            MouseEventKind::ScrollUp,
                            MouseEventKind::Down(MouseButton::Right),
                        ];
                        let kind = kinds[(r >> 8) as usize % kinds.len()];
                        let m = mods[(r >> 20) as usize % mods.len()];
                        mouse(
                            &mut h,
                            kind,
                            (r >> 32) as u16 % (w + 2),
                            (r >> 40) as u16 % (hgt + 2),
                            m,
                        );
                    }
                    8 => {
                        h.send(Event::Paste("x\r\ny\t漢".to_owned()));
                    }
                    _ => {
                        h.advance(0.15);
                    }
                }
            }
        }
    }
}

// ---- snapshots ---------------------------------------------------------------------------

fn both(name: &str, make: impl Fn(Look) -> Harness<Ed>) {
    for (look, mode) in [(Look::dark(), "dark"), (Look::light(), "light")] {
        let h = make(look);
        h.snapshot(&format!("{name}.{mode}"));
    }
}

const SAMPLE: &str = "use std::fmt;\n\n/// A point on the grid.\npub struct Point {\n    x: i32,\n    y: i32,\n}\n\nimpl Point {\n    pub fn norm(&self) -> f64 {\n        let sum = self.x * self.x + self.y * self.y;\n        (sum as f64).sqrt() // the length\n    }\n}\n";

#[test]
fn snapshot_editor_with_syntax_cursorline_and_guides() {
    both("editor", |look| {
        let mut ed = app(SAMPLE, Cfg::default());
        ed.markers = vec![
            ChangeMarker::new(4..6, ChangeKind::Modified),
            ChangeMarker::new(10..11, ChangeKind::Added),
        ];
        ed.icons = vec![GutterIcon::new(10, Glyph::Breakpoint, "Breakpoint").tone(Tone::Error)];
        ed.editor.settings.right_margin = 56;
        let mut h = harness_with(ed, look, 64, 15);
        h.click(TEXT_X + 14, 10);
        h
    });
}

#[test]
fn snapshot_multiple_carets() {
    both("editor-carets", |look| {
        let mut h = harness_with(app(SAMPLE, Cfg::default()), look, 64, 10);
        h.click(TEXT_X + 4, 4);
        h.ctrl('k');
        key(&mut h, KeyCode::Down, KeyModifiers::CONTROL);
        key(
            &mut h,
            KeyCode::Right,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        h
    });
}

#[test]
fn snapshot_column_selection() {
    both("editor-column", |look| {
        let mut h = harness_with(app(SAMPLE, Cfg::default()), look, 64, 10);
        mouse(
            &mut h,
            MouseEventKind::Down(MouseButton::Left),
            TEXT_X + 4,
            3,
            KeyModifiers::ALT,
        );
        mouse(
            &mut h,
            MouseEventKind::Drag(MouseButton::Left),
            TEXT_X + 9,
            5,
            KeyModifiers::ALT,
        );
        mouse(
            &mut h,
            MouseEventKind::Up(MouseButton::Left),
            TEXT_X + 9,
            5,
            KeyModifiers::ALT,
        );
        h
    });
}

#[test]
fn snapshot_folded_region() {
    both("editor-folded", |look| {
        let mut ed = app(
            SAMPLE,
            Cfg {
                folding: true,
                ..Cfg::default()
            },
        );
        let doc = &ed.editor.document;
        let open = SAMPLE.find("Point {").expect("in the sample") + 6;
        let close = SAMPLE.find("}\n\nimpl").expect("in the sample") + 1;
        ed.editor
            .folding
            .add_region(doc, open, close, "{...}", false);
        let body = SAMPLE.find("f64 {").expect("in the sample") + 4;
        let end = SAMPLE.rfind("    }").expect("in the sample") + 5;
        ed.editor.folding.add_region(doc, body, end, "{...}", true);
        let mut h = harness_with(ed, look, 64, 10);
        h.click(TEXT_X + 6, 9);
        h
    });
}

#[test]
fn snapshot_diagnostics() {
    both("editor-diagnostics", |look| {
        let mut ed = app(
            SAMPLE,
            Cfg {
                virtual_text: true,
                ..Cfg::default()
            },
        );
        let unused = SAMPLE.find("fmt").expect("in the sample");
        let sum = SAMPLE.find("sum as").expect("in the sample");
        ed.highlights = vec![
            RangeHighlight::new(unused..unused + 3, HighlightSeverity::Warning)
                .message("unused import"),
            RangeHighlight::new(sum..sum + 3, HighlightSeverity::Error).message("mismatched types"),
        ];
        let mut h = harness_with(ed, look, 64, 14);
        h.click(TEXT_X, 13);
        h
    });
}

#[test]
fn snapshot_find_and_replace() {
    both("editor-find", |look| {
        let mut h = harness_with(app(SAMPLE, Cfg::default()), look, 64, 12);
        h.ctrl('r');
        h.type_text("self");
        key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
        key(&mut h, KeyCode::Tab, KeyModifiers::NONE);
        h.type_text("this");
        h
    });
}

#[test]
fn snapshot_completion_menu() {
    both("editor-completion", |look| {
        let mut h = harness_with(
            app(
                SAMPLE,
                Cfg {
                    completer: true,
                    ..Cfg::default()
                },
            ),
            look,
            64,
            16,
        );
        h.click(TEXT_X + 44, 10);
        key(&mut h, KeyCode::End, KeyModifiers::NONE);
        key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
        h.type_text("pr");
        key(&mut h, KeyCode::Down, KeyModifiers::NONE);
        h
    });
}

/// A caret moved from outside (the IDE core typed or jumped) on the last row of a long wrapped
/// line is scrolled into view by row, not only by line (TUI-BUG-019).
#[test]
fn scroll_to_caret_shows_a_caret_far_down_a_wrapped_line() {
    let line = "x".repeat(5000);
    let mut h = harness_with(
        app(
            &format!("{line} end"),
            Cfg {
                wrap: true,
                ..Cfg::default()
            },
        ),
        Look::dark(),
        30,
        10,
    );
    let end = h.app.editor.document.len_chars();
    {
        let Ed { editor, .. } = &mut h.app;
        let doc = editor.document.clone();
        editor.carets.set_caret(&doc, end);
    }
    h.app.state.scroll_to_caret();
    h.draw();
    let screen = h.screen();
    assert!(
        screen.iter().any(|row| row.contains("end")),
        "the caret row is shown: {screen:?}"
    );
    assert!(h.app.state.first_row() > 0, "scrolled down");
}
