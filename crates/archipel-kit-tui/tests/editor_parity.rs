//! The terminal editor against the GUI `EditorComponent`: every feature the GUI editor has,
//! driven with synthetic events and read back from the buffer. The areas follow
//! `docs/tui-editor-parity.md`: builder options, outputs, typing assists, hover floats,
//! find, completion, highlighting.

use archipel_kit::Tone;
use archipel_kit::editor::logic::lookup::is_identifier_char;
use archipel_kit::editor::model::{Editor, Os as ModelOs, SmartBackspace};
use archipel_kit::editor::{
    ChangeKind, ChangeMarker, GutterIcon, HighlightSeverity, JavaScript, NavigationKind,
    RangeHighlight, TemplateHighlighter, TemplateToken, TextAttributesKey, template_at,
};
use archipel_kit::ui::{Completer, Completion, Completions};
use archipel_kit_tui::editor::{
    CodeHighlighter, CodeLanguage, EditorComponent, EditorOutput, EditorState,
};
use archipel_kit_tui::input::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use archipel_kit_tui::ratatui::{buffer::Buffer, layout::Rect, style::Modifier};
use archipel_kit_tui::testing::Harness;
use archipel_kit_tui::{App, Caps, Event, Glyph, Id, Look, Runtime};
use std::cell::RefCell;

const ID: Id = Id::new("code");

/// Text starts after the sign column (3), two digits and a space.
const TEXT_X: u16 = 6;

// ---- completers ------------------------------------------------------------------------------

/// A completer with the knobs the tests turn.
#[derive(Default)]
struct Words {
    /// Items chosen before, most recent last (the caller's `LookupStats`).
    recent: Vec<&'static str>,
    docs: bool,
    /// Extra items a second Ctrl+Space adds.
    more: Vec<&'static str>,
    extended_calls: RefCell<u32>,
    advertise: bool,
    /// Seconds between a typed identifier character and the list opening.
    delay: f32,
}

const WORDS: &[&str] = &[
    "getString",
    "get",
    "size",
    "Gap",
    "println",
    "print",
    "push",
];

fn run_start(text: &str, caret: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut start = caret.min(chars.len());
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }
    start
}

impl Words {
    fn items(&self, extra: bool) -> Vec<Completion> {
        let mut items: Vec<Completion> = WORDS
            .iter()
            .map(|w| {
                let mut c = Completion::new(*w, *w);
                if self.docs && *w == "getString" {
                    c = c.documentation("Reads a string from the stream.");
                }
                if *w == "println" {
                    c = c.bold().tail_text("(x)").type_text("fn");
                }
                if *w == "push" {
                    c = c.strikeout();
                }
                c
            })
            .collect();
        if extra {
            items.extend(self.more.iter().map(|w| Completion::new(*w, *w)));
        }
        items
    }

    fn answer(&self, text: &str, caret: usize, extra: bool) -> Option<Completions> {
        let start = run_start(text, caret);
        (start < caret).then(|| Completions {
            start,
            end: caret,
            items: self.items(extra),
        })
    }
}

impl Completer for Words {
    fn complete(&self, text: &str, caret: usize) -> Option<Completions> {
        self.answer(text, caret, false)
    }

    fn complete_on_request(&self, text: &str, caret: usize) -> Option<Completions> {
        let start = run_start(text, caret);
        Some(Completions {
            start,
            end: caret,
            items: self.items(false),
        })
    }

    fn complete_extended(&self, text: &str, caret: usize) -> Option<Completions> {
        *self.extended_calls.borrow_mut() += 1;
        let start = run_start(text, caret);
        Some(Completions {
            start,
            end: caret,
            items: self.items(true),
        })
    }

    fn recency(&self, lookup: &str) -> u64 {
        self.recent
            .iter()
            .position(|r| *r == lookup)
            .map_or(0, |p| p as u64 + 1)
    }

    fn advertisement(&self) -> Option<String> {
        self.advertise
            .then(|| "Press Ctrl+Space again for more".to_owned())
    }

    fn auto_popup_delay(&self) -> f32 {
        self.delay
    }
}

/// `{{variable}}` completion over the shared template detection.
struct Variables;

impl Completer for Variables {
    fn complete(&self, text: &str, caret: usize) -> Option<Completions> {
        let byte = text
            .char_indices()
            .nth(caret)
            .map_or(text.len(), |(b, _)| b);
        let token = template_at(text, byte)?;
        let start = text[..token.range.start].chars().count();
        let end = text[..token.range.end].chars().count().max(caret);
        Some(Completions {
            start,
            end,
            items: ["token", "host", "port"]
                .iter()
                .map(|n| {
                    Completion::new(format!("{{{{{n}}}}}"), format!("{{{{{n}}}}}"))
                        .lookup_string(*n)
                })
                .collect(),
        })
    }
}

// ---- the app under test ----------------------------------------------------------------------

#[derive(Clone)]
struct Cfg {
    viewer: bool,
    wrap: bool,
    folding: bool,
    bracket_folding: bool,
    enabled: bool,
    tab_indents: bool,
    stick: bool,
    scroll_to_line: Option<usize>,
    whitespace: bool,
    virtual_text: bool,
    plain: bool,
    os: ModelOs,
    placeholder: Option<&'static str>,
    completer: bool,
    variables: bool,
    template: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            viewer: false,
            wrap: false,
            folding: false,
            bracket_folding: false,
            enabled: true,
            tab_indents: true,
            stick: false,
            scroll_to_line: None,
            whitespace: false,
            virtual_text: false,
            plain: false,
            os: ModelOs::WindowsLinux,
            placeholder: None,
            completer: false,
            variables: false,
            template: false,
        }
    }
}

struct Ed {
    editor: Editor,
    state: EditorState,
    cfg: Cfg,
    hl: CodeHighlighter,
    words: Words,
    highlights: Vec<RangeHighlight>,
    icons: Vec<GutterIcon>,
    markers: Vec<ChangeMarker>,
    /// What the last event returned.
    last: EditorOutput,
    /// What the last frame returned.
    shown: EditorOutput,
}

fn resolver(name: &str) -> TemplateToken {
    if name == "known" {
        TemplateToken::Resolved
    } else {
        TemplateToken::Unresolved
    }
}

macro_rules! component {
    ($s:expr) => {{
        let Ed {
            editor,
            cfg,
            hl,
            words,
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
            .bracket_folding(cfg.bracket_folding)
            .enabled(cfg.enabled)
            .tab_indents(cfg.tab_indents)
            .stick_to_bottom(cfg.stick)
            .scroll_to_line(cfg.scroll_to_line)
            .show_whitespace(cfg.whitespace)
            .virtual_text(cfg.virtual_text)
            .keymap_os(cfg.os);
        if let Some(text) = cfg.placeholder {
            c = c.placeholder(text);
        }
        if !cfg.plain {
            c = c.highlighter(hl);
        }
        if cfg.completer {
            c = c.completer(words);
        }
        if cfg.variables {
            c = c.completer(&Variables);
        }
        c
    }};
}

impl App for Ed {
    fn event(&mut self, rt: &mut Runtime, event: &Event) {
        let mut state = std::mem::take(&mut self.state);
        let out = component!(self).handle(rt, &mut state, event);
        self.state = state;
        self.last = out;
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let mut state = std::mem::take(&mut self.state);
        if self.cfg.template {
            let template = TemplateHighlighter::new(resolver).inner(JavaScript);
            let mut c = EditorComponent::new(ID, &mut self.editor).highlighter(&template);
            if self.cfg.variables {
                c = c.completer(&Variables);
            }
            self.shown = c.show(rt, &mut state, area, buf);
        } else {
            self.shown = component!(self).show(rt, &mut state, area, buf);
        }
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
        words: Words::default(),
        highlights: Vec::new(),
        icons: Vec::new(),
        markers: Vec::new(),
        last: EditorOutput::default(),
        shown: EditorOutput::default(),
    }
}

fn harness_look(ed: Ed, look: Look, w: u16, h: u16) -> Harness<Ed> {
    let mut harness = Harness::with_runtime(ed, runtime(look), w, h);
    harness.draw();
    harness
}

fn harness_with(ed: Ed, w: u16, h: u16) -> Harness<Ed> {
    harness_look(ed, Look::dark(), w, h)
}

fn harness(text: &str, cfg: Cfg) -> Harness<Ed> {
    harness_with(app(text, cfg), 48, 10)
}

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

const CODE: &str =
    "fn main() {\n    let total = 42;\n    if total > 1 {\n        println!(\"big\");\n    }\n}\n";

// ==== builder options ========================================================================

/// With the fold column on, the text starts two cells later (sign 3, digits 2, space, fold 2).
const FOLD_X: u16 = 6;
const TEXT_X_FOLDING: u16 = 8;

#[test]
fn bracket_folding_makes_regions_that_follow_the_text() {
    let json = "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": 3\n}\n";
    let mut h = harness(
        json,
        Cfg {
            folding: true,
            bracket_folding: true,
            ..Cfg::default()
        },
    );
    // The object and the array are regions, shown with chevrons in the fold column.
    assert_eq!(h.app.editor.folding.regions().len(), 2);
    let down = h.rt.look.glyph(Glyph::ChevronDown);
    assert_eq!(cell(&h, FOLD_X, 0).symbol(), down);
    // Collapsing the array shows its placeholder.
    h.click(FOLD_X, 1);
    assert!(h.screen()[1].contains("[…]"), "{}", h.text());
    // Editing keeps what was collapsed.
    h.click(TEXT_X_FOLDING + 1, 0);
    h.type_text("x");
    assert!(h.screen()[1].contains("[…]"), "{}", h.text());
    assert!(!h.app.editor.folding.regions()[1].expanded);
}

#[test]
fn the_fold_chords_collapse_and_expand_the_region_at_the_caret() {
    // Ctrl+- and Ctrl++ cannot be sent by a legacy terminal: Ctrl+K then - and =.
    let mut h = harness(
        "fn f() {\n    a;\n}\n",
        Cfg {
            folding: true,
            bracket_folding: true,
            ..Cfg::default()
        },
    );
    // The caret inside the body.
    h.click(TEXT_X_FOLDING + 4, 1);
    h.ctrl('k');
    h.type_text("-");
    assert!(h.screen()[0].contains("{…}"), "{}", h.text());
    assert!(!h.text().contains("a;"));
    h.ctrl('k');
    h.type_text("=");
    assert!(h.text().contains("a;"), "{}", h.text());
}

#[test]
fn without_bracket_folding_the_regions_are_the_callers() {
    let h = harness(
        "{\n  1\n}\n",
        Cfg {
            folding: true,
            ..Cfg::default()
        },
    );
    assert!(h.app.editor.folding.regions().is_empty());
}

#[test]
fn a_disabled_editor_is_dimmed_and_takes_no_input() {
    let mut h = harness(
        CODE,
        Cfg {
            enabled: false,
            ..Cfg::default()
        },
    );
    let grey = h.rt.look.palette.grey;
    assert_eq!(cell(&h, TEXT_X, 0).symbol(), "f");
    assert_eq!(cell(&h, TEXT_X, 0).fg, grey);
    h.type_text("zzz");
    h.click(TEXT_X + 3, 1);
    h.press(KeyCode::Enter);
    assert_eq!(text(&h), CODE, "typing, clicks and keys do nothing");
    assert!(!h.app.last.consumed);
    assert!(h.rt.cursor.is_none(), "no terminal cursor");
}

#[test]
fn tab_indents_by_default_and_is_left_to_the_application_when_off() {
    let mut h = harness("a", Cfg::default());
    h.press(KeyCode::Tab);
    assert_eq!(text(&h), "    a");
    let mut h = harness(
        "a",
        Cfg {
            tab_indents: false,
            ..Cfg::default()
        },
    );
    h.press(KeyCode::Tab);
    assert_eq!(text(&h), "a");
    assert!(
        !h.app.last.consumed,
        "the key is left for the focus traversal"
    );
    key(&mut h, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(text(&h), "a");
}

fn log(lines: usize) -> String {
    (0..lines).map(|i| format!("line {i}\n")).collect()
}

#[test]
fn stick_to_bottom_follows_a_growing_document_until_the_user_scrolls_up() {
    let mut h = harness(
        &log(30),
        Cfg {
            stick: true,
            viewer: true,
            ..Cfg::default()
        },
    );
    // Starts at the end, with no room past the last row.
    let first = h.app.state.first_row();
    assert!(first >= 21, "first row {first}");
    assert!(h.text().contains("line 29"), "{}", h.text());
    // The document grows: the view follows.
    let end = h.app.editor.document.len();
    h.app.editor.replace_range(end, end, "line 30\nline 31\n");
    h.draw();
    assert!(h.text().contains("line 31"), "{}", h.text());
    // Scrolling up stops the following.
    h.scroll(TEXT_X + 2, 3, -6);
    let held = h.app.state.first_row();
    let end = h.app.editor.document.len();
    h.app.editor.replace_range(end, end, "line 32\n");
    h.draw();
    assert_eq!(h.app.state.first_row(), held);
    assert!(!h.text().contains("line 32"));
}

#[test]
fn scroll_to_line_centers_the_line() {
    let h = harness(
        &log(60),
        Cfg {
            scroll_to_line: Some(40),
            ..Cfg::default()
        },
    );
    let (_, y) = h.find("line 40").expect("line 40 is on screen");
    assert!((3..=6).contains(&y), "centered in 10 rows, got row {y}");
}

#[test]
fn a_placeholder_shows_while_the_document_is_empty() {
    let mut h = harness(
        "",
        Cfg {
            placeholder: Some("Request body"),
            ..Cfg::default()
        },
    );
    assert!(h.text().contains("Request body"), "{}", h.text());
    let grey = h.rt.look.palette.grey_fg;
    let (x, y) = h.find("Request body").expect("placeholder");
    assert_eq!(cell(&h, x, y).fg, grey);
    h.type_text("x");
    assert!(!h.text().contains("Request body"));
}

#[test]
fn a_viewer_selects_and_copies_but_does_not_edit() {
    let mut h = harness(
        "one two",
        Cfg {
            viewer: true,
            completer: true,
            ..Cfg::default()
        },
    );
    h.ctrl('a');
    assert_eq!(selection(&h), Some((0, 7)));
    h.ctrl('c');
    assert_eq!(h.rt.clipboard.get(), "one two");
    h.type_text("x");
    h.press(KeyCode::Backspace);
    h.ctrl('v');
    assert_eq!(text(&h), "one two");
    h.ctrl(' ');
    assert!(!h.app.state.completion_open(), "a viewer completes nothing");
}

// ==== what a frame and an event report =======================================================

#[test]
fn the_frame_reports_selection_caret_cell_line_count_visible_lines_and_matches() {
    let mut h = harness(&log(40), Cfg::default());
    assert_eq!(h.app.shown.line_count, 41);
    assert_eq!(h.app.shown.visible_lines, 0..10);
    assert_eq!(h.app.shown.selection, None);
    assert_eq!(h.app.shown.caret_cell, Some((TEXT_X, 0)));
    // Select a word: `line` of line 0.
    h.double_click(TEXT_X + 1, 0);
    assert_eq!(h.app.shown.selection, Some(0..4));
    h.scroll(TEXT_X + 2, 3, 5);
    assert!(
        h.app.shown.visible_lines.start >= 14,
        "{:?}",
        h.app.shown.visible_lines
    );
    // The matches of the find bar.
    h.ctrl('f');
    h.type_text("line 1");
    // `line 1`, `line 10`..`line 19`.
    assert_eq!(h.app.shown.matches, 11);
}

#[test]
fn the_caret_cell_is_none_when_the_caret_is_scrolled_away() {
    let mut h = harness(&log(60), Cfg::default());
    h.scroll(TEXT_X + 2, 3, 10);
    assert_eq!(h.app.shown.caret_cell, None);
}

#[test]
fn ctrl_b_and_ctrl_alt_b_ask_for_navigation_at_the_caret() {
    let mut h = harness("let total = 1;", Cfg::default());
    h.click(TEXT_X + 5, 0);
    let at = caret(&h);
    key(&mut h, KeyCode::Char('b'), KeyModifiers::CONTROL);
    let nav = h.app.last.navigation.expect("declaration");
    assert_eq!((nav.offset, nav.kind), (at, NavigationKind::Declaration));
    key(
        &mut h,
        KeyCode::Char('b'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    let nav = h.app.last.navigation.expect("implementation");
    assert_eq!(nav.kind, NavigationKind::Implementation);
    assert_eq!(text(&h), "let total = 1;", "nothing was typed");
}

#[test]
fn cmd_b_is_navigation_on_macos_and_ctrl_b_moves_left_there() {
    let mut h = harness(
        "abc",
        Cfg {
            os: ModelOs::MacOs,
            ..Cfg::default()
        },
    );
    h.press(KeyCode::End);
    key(&mut h, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(
        caret(&h),
        2,
        "Ctrl+B is the Emacs-style Left of the macOS keymap"
    );
    assert!(h.app.last.navigation.is_none());
    key(&mut h, KeyCode::Char('b'), KeyModifiers::SUPER);
    assert_eq!(
        h.app.last.navigation.map(|n| n.kind),
        Some(NavigationKind::Declaration)
    );
}

// ==== typing assists (the model's rules through the terminal's keys) =========================

#[test]
fn brackets_and_quotes_close_themselves_and_are_overtyped() {
    let mut h = harness("", Cfg::default());
    h.type_text("f(");
    assert_eq!(text(&h), "f()");
    assert_eq!(caret(&h), 2);
    h.type_text("\"a");
    assert_eq!(text(&h), "f(\"a\")");
    h.type_text("\"");
    assert_eq!(text(&h), "f(\"a\")", "the closing quote is typed over");
    h.type_text(")");
    assert_eq!(text(&h), "f(\"a\")");
    assert_eq!(caret(&h), 6);
}

#[test]
fn typing_a_bracket_over_a_selection_wraps_it() {
    let mut h = harness("word", Cfg::default());
    h.ctrl('a');
    h.type_text("[");
    assert_eq!(text(&h), "[word]");
    assert_eq!(selection(&h), Some((1, 5)), "the selection stays inside");
}

#[test]
fn enter_between_braces_indents_and_puts_the_closer_on_its_own_line() {
    let mut h = harness("fn f() {", Cfg::default());
    h.press(KeyCode::End);
    h.type_text("}");
    h.press(KeyCode::Left);
    h.press(KeyCode::Enter);
    assert_eq!(text(&h), "fn f() {\n    \n}");
    assert_eq!(caret(&h), 13);
}

#[test]
fn enter_keeps_the_indent_of_the_line() {
    let mut h = harness("    abc", Cfg::default());
    h.press(KeyCode::End);
    h.press(KeyCode::Enter);
    assert_eq!(text(&h), "    abc\n    ");
}

#[test]
fn smart_backspace_deletes_an_empty_pair_and_an_indent_level() {
    let mut h = harness("", Cfg::default());
    h.type_text("(");
    h.press(KeyCode::Backspace);
    assert_eq!(text(&h), "", "both halves of the pair go");
    // The IntelliJ default deletes one character; the Indent mode goes back an indent level.
    let mut h = harness("        x", Cfg::default());
    h.app.editor.settings.smart_backspace = SmartBackspace::Indent;
    h.click(TEXT_X + 8, 0);
    h.press(KeyCode::Backspace);
    assert_eq!(text(&h), "    x", "an indent level, not one space");
    let mut h = harness("        x", Cfg::default());
    h.click(TEXT_X + 8, 0);
    h.press(KeyCode::Backspace);
    assert_eq!(text(&h), "       x", "one space by default");
}

#[test]
fn ctrl_slash_toggles_line_comments_on_the_selected_lines() {
    // Ctrl+/ arrives as Ctrl+7 in a legacy terminal.
    let mut h = harness("a\nb\nc\n", Cfg::default());
    h.press(KeyCode::Down);
    key(&mut h, KeyCode::Down, KeyModifiers::SHIFT);
    key(&mut h, KeyCode::Down, KeyModifiers::SHIFT);
    key(&mut h, KeyCode::Char('7'), KeyModifiers::CONTROL);
    assert_eq!(text(&h), "a\n//b\n//c\n");
    key(&mut h, KeyCode::Char('7'), KeyModifiers::CONTROL);
    assert_eq!(text(&h), "a\nb\nc\n");
    // Without a selection the caret line is the one.
    h.click(TEXT_X, 1);
    key(&mut h, KeyCode::Char('7'), KeyModifiers::CONTROL);
    assert_eq!(text(&h), "a\n//b\nc\n");
}

#[test]
fn duplicate_delete_and_move_line() {
    let mut h = harness("one\ntwo\nthree\n", Cfg::default());
    h.press(KeyCode::Down);
    h.ctrl('d');
    assert_eq!(text(&h), "one\ntwo\ntwo\nthree\n");
    h.ctrl('y');
    assert_eq!(
        text(&h),
        "one\ntwo\nthree\n",
        "the duplicate was the caret line"
    );
    // The caret is on `three` now; Alt+Shift+Up / Down move the line.
    key(&mut h, KeyCode::Up, KeyModifiers::ALT | KeyModifiers::SHIFT);
    assert_eq!(text(&h), "one\nthree\ntwo\n");
    key(
        &mut h,
        KeyCode::Down,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    );
    assert_eq!(text(&h), "one\ntwo\nthree\n");
}

#[test]
fn extend_and_shrink_selection_grow_by_syntax_units() {
    let mut h = harness("foo(bar, baz)", Cfg::default());
    h.click(TEXT_X + 5, 0);
    // Alt+Up is the fallback for Ctrl+W (the terminal's window leader).
    key(&mut h, KeyCode::Up, KeyModifiers::ALT);
    assert_eq!(selection(&h), Some((4, 7)), "the word");
    key(&mut h, KeyCode::Up, KeyModifiers::ALT);
    let wider = selection(&h).expect("selection");
    assert!(wider.0 <= 4 && wider.1 >= 7 && wider != (4, 7), "{wider:?}");
    key(&mut h, KeyCode::Down, KeyModifiers::ALT);
    assert_eq!(selection(&h), Some((4, 7)), "shrinks back");
}

#[test]
fn undo_groups_a_typed_word_and_redo_brings_it_back() {
    let mut h = harness("", Cfg::default());
    h.type_text("hello");
    h.press(KeyCode::Enter);
    h.type_text("world");
    h.ctrl('z');
    assert_eq!(text(&h), "hello\n", "the typed word is one step");
    h.ctrl('z');
    assert_eq!(text(&h), "hello", "the line break is its own step");
    // Redo is Ctrl+K Z in a legacy terminal (Ctrl+Shift+Z looks like Ctrl+Z).
    h.ctrl('k');
    h.type_text("z");
    assert_eq!(text(&h), "hello\n");
}

#[test]
fn carets_can_be_cloned_above_and_below_and_type_together() {
    let mut h = harness("aa\nbb\ncc\n", Cfg::default());
    h.press(KeyCode::Down);
    h.ctrl('k');
    key(&mut h, KeyCode::Up, KeyModifiers::CONTROL);
    h.ctrl('k');
    key(&mut h, KeyCode::Down, KeyModifiers::CONTROL);
    assert_eq!(h.app.editor.carets.caret_count(), 3);
    h.type_text("X");
    assert_eq!(text(&h), "Xaa\nXbb\nXcc\n");
}

#[test]
fn column_mode_extends_a_block_with_shift_and_the_arrows() {
    let mut h = harness("abcd\nefgh\nijkl\n", Cfg::default());
    h.click(TEXT_X + 1, 0);
    // Alt+Shift+Insert toggles the column mode; Ctrl+K C is the fallback.
    h.ctrl('k');
    h.type_text("c");
    key(&mut h, KeyCode::Down, KeyModifiers::SHIFT);
    key(&mut h, KeyCode::Down, KeyModifiers::SHIFT);
    key(&mut h, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(h.app.editor.carets.caret_count(), 3);
    h.type_text("_");
    assert_eq!(text(&h), "a_cd\ne_gh\ni_kl\n");
}

#[test]
fn the_macos_keymap_moves_by_words_with_option_and_deletes_lines_with_cmd() {
    let mut h = harness(
        "one two three",
        Cfg {
            os: ModelOs::MacOs,
            ..Cfg::default()
        },
    );
    key(&mut h, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(caret(&h), 3);
    key(&mut h, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(caret(&h), 7);
    // Cmd+Backspace deletes the line there.
    key(&mut h, KeyCode::Backspace, KeyModifiers::SUPER);
    assert_eq!(text(&h), "");
    // On Windows/Linux Option is not a word key: Alt+Right does not move the caret by word.
    let mut h = harness("one two three", Cfg::default());
    key(&mut h, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(caret(&h), 0);
}

// ==== rendering that came with parity ========================================================

#[test]
fn doc_comments_are_tinted_as_comments() {
    let src = "//! The crate.\n/// A point.\npub struct P;\n/** block */\n/*! inner */\n// plain\n";
    let h = harness(src, Cfg::default());
    let comment = h.rt.look.color(h.rt.look.theme.editor.comment);
    let plain = h.rt.look.palette.white;
    for (row, probe) in [(0u16, 5u16), (1, 8), (3, 4), (4, 4), (5, 5)] {
        assert_eq!(
            cell(&h, TEXT_X + probe, row).fg,
            comment,
            "row {row} is a comment: {}",
            h.screen()[row as usize]
        );
    }
    assert_ne!(cell(&h, TEXT_X + 6, 2).fg, comment, "`struct` is not");
    assert_ne!(comment, plain);
}

#[test]
fn the_current_match_is_inverted_and_bold_the_others_only_tinted() {
    let mut h = harness("foo bar foo baz foo\n", Cfg::default());
    h.ctrl('f');
    h.type_text("foo");
    let ed = h.rt.look.theme.editor;
    let p = h.rt.look.palette;
    // The first match from the caret is current.
    let current = cell(&h, TEXT_X, 1);
    assert_eq!(current.bg, h.rt.look.color(ed.caret));
    assert_eq!(current.fg, p.black);
    assert!(current.modifier.contains(Modifier::BOLD));
    // The others carry the search tint, plain weight and the normal text color.
    let other = cell(&h, TEXT_X + 8, 1);
    assert_eq!(other.bg, h.rt.look.color(ed.search_result));
    assert!(!other.modifier.contains(Modifier::BOLD));
    assert_ne!(other.fg, p.black);
    // The two are told apart by far more than the tint: the contrast of the backgrounds.
    let luma = |c: archipel_kit_tui::ratatui::style::Color| match c {
        archipel_kit_tui::ratatui::style::Color::Rgb(r, g, b) => {
            0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)
        }
        _ => 0.0,
    };
    assert!((luma(current.bg) - luma(other.bg)).abs() > 60.0);
    // F3 moves the emphasis.
    h.press(KeyCode::F(3));
    assert_ne!(cell(&h, TEXT_X, 1).bg, h.rt.look.color(ed.caret));
    assert_eq!(cell(&h, TEXT_X + 8, 1).bg, h.rt.look.color(ed.caret));
}

#[test]
fn the_unmatched_brace_is_bold_in_the_invalid_color_and_a_matched_pair_is_tinted() {
    let mut h = harness("f(a(b) c\n", Cfg::default());
    // After the lone `(` at column 1.
    h.click(TEXT_X + 2, 0);
    let ed = h.rt.look.theme.editor;
    let c = cell(&h, TEXT_X + 1, 0);
    assert_eq!(c.fg, h.rt.look.color(ed.invalid));
    assert!(c.modifier.contains(Modifier::BOLD));
    // Inside the inner pair both halves are tinted.
    h.click(TEXT_X + 5, 0);
    assert_eq!(
        cell(&h, TEXT_X + 3, 0).bg,
        h.rt.look.color(ed.matched_brace)
    );
    assert_eq!(
        cell(&h, TEXT_X + 5, 0).bg,
        h.rt.look.color(ed.matched_brace)
    );
}

#[test]
fn the_guide_of_the_carets_block_is_drawn_in_the_selected_color() {
    let src = "fn f() {\n    a();\n    b();\n}\n";
    let mut h = harness(src, Cfg::default());
    h.click(TEXT_X + 6, 1);
    let ed = h.rt.look.theme.editor;
    let selected = h.rt.look.color(ed.indent_guide_selected);
    assert_eq!(cell(&h, TEXT_X, 1).symbol(), "│");
    assert_eq!(cell(&h, TEXT_X, 1).fg, selected);
    assert_eq!(cell(&h, TEXT_X, 2).fg, selected);
    // Outside the block the guide is the plain line color.
    h.click(TEXT_X + 2, 3);
    assert_ne!(cell(&h, TEXT_X, 1).fg, selected);
}

// ==== hover ===================================================================================

fn diagnostics() -> Vec<RangeHighlight> {
    // `total` on line 1 is an error and a warning at once; `main` a weak warning.
    vec![
        RangeHighlight::new(20..25, HighlightSeverity::Warning).message("unused variable `total`"),
        RangeHighlight::new(20..25, HighlightSeverity::Error).message("expected `u8`, found `i32`"),
        RangeHighlight::new(3..7, HighlightSeverity::WeakWarning).message("function never used"),
        RangeHighlight::new(0..2, HighlightSeverity::Info),
    ]
}

fn with_diagnostics() -> Harness<Ed> {
    let mut ed = app(CODE, Cfg::default());
    ed.highlights = diagnostics();
    harness_with(ed, 60, 12)
}

#[test]
fn resting_on_a_diagnostic_opens_a_float_with_its_messages_after_the_delay() {
    let mut h = with_diagnostics();
    h.hover(TEXT_X + 9, 1);
    assert!(h.app.state.hover_rect().is_none(), "not yet");
    h.advance(0.3);
    assert!(h.app.state.hover_rect().is_none(), "300 ms is too early");
    h.advance(0.3);
    let rect = h.app.state.hover_rect().expect("the float");
    assert_eq!(
        h.app.shown.hover.expect("hover target").highlight,
        Some(0),
        "the first highlight that contains the offset"
    );
    let screen = h.text();
    // The error comes first, then the warning, each with its severity.
    let error = screen.find("expected `u8`").expect("error message");
    let warning = screen.find("unused variable").expect("warning message");
    assert!(error < warning, "{screen}");
    assert!(
        screen.contains("● Error") && screen.contains("● Warning"),
        "{screen}"
    );
    // It hangs below the pointer's row and does not cover it.
    assert!(rect.y > 1, "{rect:?}");
    // The severity label is colored by the severity.
    let (x, y) = h.find("● Error").expect("label");
    let ed = h.rt.look.theme.editor;
    let error = ed.error_effect;
    let expected =
        h.rt.look
            .readable(u32::from_be_bytes([0, error.r(), error.g(), error.b()]));
    assert_eq!(cell(&h, x, y).fg, expected);
    assert!(cell(&h, x, y).modifier.contains(Modifier::BOLD));
}

#[test]
fn a_highlight_without_a_message_reports_the_hover_but_opens_no_float() {
    let mut h = with_diagnostics();
    h.hover(TEXT_X, 0);
    h.advance(0.6);
    assert!(h.app.shown.hover.is_some());
    assert!(h.app.state.hover_rect().is_none());
}

#[test]
fn moving_the_pointer_restarts_the_wait_and_closes_the_float() {
    let mut h = with_diagnostics();
    h.hover(TEXT_X + 9, 1);
    h.advance(0.6);
    assert!(h.app.state.hover_rect().is_some());
    h.hover(TEXT_X + 30, 1);
    assert!(
        h.app.state.hover_rect().is_none(),
        "nothing under the pointer now"
    );
    h.hover(TEXT_X + 8, 1);
    h.advance(0.3);
    assert!(h.app.state.hover_rect().is_none(), "the wait started over");
    h.advance(0.3);
    assert!(h.app.state.hover_rect().is_some());
}

#[test]
fn a_key_dismisses_the_float_until_the_pointer_moves_on() {
    let mut h = with_diagnostics();
    h.hover(TEXT_X + 9, 1);
    h.advance(0.6);
    assert!(h.app.state.hover_rect().is_some());
    h.press(KeyCode::Down);
    assert!(h.app.state.hover_rect().is_none());
    h.advance(2.0);
    assert!(
        h.app.state.hover_rect().is_none(),
        "stays away while the pointer rests"
    );
    h.hover(TEXT_X + 10, 1);
    h.advance(0.6);
    assert!(h.app.state.hover_rect().is_some());
}

#[test]
fn the_deadline_asks_for_a_frame_when_the_delay_is_up() {
    let mut h = with_diagnostics();
    h.hover(TEXT_X + 9, 1);
    let due = h.app.state.deadline().expect("a wake-up is due");
    assert!(
        due > h.rt.now && due <= h.rt.now + 0.51,
        "{due} vs {}",
        h.rt.now
    );
    h.advance(0.6);
    assert!(
        h.app.state.deadline().is_none(),
        "nothing pending once shown"
    );
}

#[test]
fn resting_on_a_fold_placeholder_previews_the_folded_lines() {
    let mut h = harness(
        "fn main() {\n    let a = 1;\n    let b = 2;\n}\n",
        Cfg {
            folding: true,
            bracket_folding: true,
            ..Cfg::default()
        },
    );
    h.click(FOLD_X, 0);
    assert!(h.screen()[0].contains("{…}"), "{}", h.text());
    let (x, y) = h.find("{…}").expect("placeholder");
    h.hover(x + 1, y);
    h.advance(0.2);
    assert!(h.app.state.hover_rect().is_none());
    h.advance(0.2);
    assert!(h.app.shown.fold_preview.is_some());
    let screen = h.text();
    assert!(
        screen.contains("let a = 1;") && screen.contains("let b = 2;"),
        "{screen}"
    );
}

#[test]
fn a_stripe_mark_reports_the_hover_and_names_itself() {
    let mut h = with_diagnostics();
    let width = 60;
    let stripe = width - 1;
    // The error mark is on the row of line 1.
    h.hover(stripe, 1);
    assert_eq!(
        h.app.shown.stripe_hover,
        Some(1),
        "the error wins on the row"
    );
    h.advance(0.6);
    let screen = h.text();
    assert!(screen.contains("expected `u8`"), "{screen}");
    // A click moves the caret to the mark.
    h.click(stripe, 1);
    assert_eq!(caret(&h), 20);
    // Where no mark is, nothing is reported.
    h.hover(stripe, 8);
    assert_eq!(h.app.shown.stripe_hover, None);
}

#[test]
fn a_gutter_icon_names_itself_after_the_delay() {
    let mut ed = app(CODE, Cfg::default());
    ed.icons = vec![GutterIcon::new(1, Glyph::Run, "Run line 2").tone(Tone::Run)];
    let mut h = harness_with(ed, 60, 12);
    h.hover(1, 1);
    h.advance(0.6);
    assert!(h.text().contains("Run line 2"), "{}", h.text());
    h.hover(1, 2);
    assert!(!h.text().contains("Run line 2"));
}

// ==== find ====================================================================================

#[test]
fn a_multi_line_selection_becomes_the_scope_of_the_search() {
    let mut h = harness("a x\na x\na x\n", Cfg::default());
    h.press(KeyCode::Down);
    key(&mut h, KeyCode::Down, KeyModifiers::SHIFT);
    h.ctrl('f');
    h.type_text("x");
    assert_eq!(
        h.app.shown.matches,
        1,
        "only inside lines 2 to 3: {}",
        h.text()
    );
    h.press(KeyCode::Esc);
}

#[test]
fn ctrl_f3_searches_the_word_at_the_caret_as_a_whole_word() {
    let mut h = harness("foo foobar foo\n", Cfg::default());
    h.click(TEXT_X + 1, 0);
    key(&mut h, KeyCode::F(3), KeyModifiers::CONTROL);
    assert!(h.app.state.find_open());
    assert_eq!(selection(&h), Some((11, 14)), "foobar is not a whole word");
}

#[test]
fn clearing_the_query_puts_the_caret_and_the_view_back() {
    let mut h = harness(&log(40), Cfg::default());
    h.click(TEXT_X + 2, 2);
    let before = (caret(&h), h.app.state.first_row());
    h.ctrl('f');
    h.type_text("line 3");
    assert_ne!(caret(&h), before.0);
    for _ in 0..6 {
        h.press(KeyCode::Backspace);
    }
    assert_eq!((caret(&h), h.app.state.first_row()), before);
}

#[test]
fn replace_all_works_inside_the_scope_and_is_one_undo_step() {
    let mut h = harness("x x\nx x\nx x\n", Cfg::default());
    h.press(KeyCode::Down);
    key(&mut h, KeyCode::Down, KeyModifiers::SHIFT);
    h.ctrl('r');
    h.type_text("x");
    h.press(KeyCode::Tab);
    h.type_text("yy");
    h.alt('a');
    assert_eq!(text(&h), "x x\nyy yy\nx x\n");
    h.press(KeyCode::Esc);
    h.ctrl('z');
    assert_eq!(text(&h), "x x\nx x\nx x\n");
}

#[test]
fn preserve_case_replaces_with_the_case_of_the_match() {
    let mut h = harness("Foo foo FOO\n", Cfg::default());
    h.ctrl('r');
    h.type_text("foo");
    h.press(KeyCode::Tab);
    h.type_text("bar");
    h.alt('p');
    h.alt('a');
    assert_eq!(text(&h), "Bar bar BAR\n");
}

#[test]
fn the_wrap_hint_shows_and_the_second_press_wraps() {
    let mut h = harness("a b a\n", Cfg::default());
    h.ctrl('f');
    h.type_text("a");
    h.press(KeyCode::Enter);
    assert_eq!(selection(&h), Some((4, 5)));
    h.press(KeyCode::Enter);
    assert!(
        h.app
            .last
            .hint
            .as_deref()
            .is_some_and(|t| t.contains("press F3"))
    );
    assert_eq!(selection(&h), Some((4, 5)));
    h.press(KeyCode::Enter);
    assert_eq!(selection(&h), Some((0, 1)));
}

#[test]
fn the_query_history_is_shared_through_the_state() {
    let mut h = harness("abc abd\n", Cfg::default());
    h.ctrl('f');
    h.type_text("abc");
    h.press_with(KeyCode::Enter, KeyModifiers::CONTROL);
    assert_eq!(
        h.app.state.history.queries().first().map(String::as_str),
        Some("abc")
    );
}

// ==== completion ===============================================================================

fn completing(text: &str, words: Words) -> Harness<Ed> {
    let mut ed = app(
        text,
        Cfg {
            completer: true,
            ..Cfg::default()
        },
    );
    ed.words = words;
    harness_with(ed, 60, 14)
}

fn rows(h: &Harness<Ed>) -> Vec<String> {
    h.screen()
}

#[test]
fn camel_humps_narrow_the_list_and_prefix_matches_come_first() {
    let mut h = completing("", Words::default());
    h.type_text("gS");
    let screen = h.text();
    assert!(screen.contains("getString"), "{screen}");
    assert!(!screen.contains("println"), "{screen}");
    assert!(!screen.contains("Gap"), "`Gap` has no hump S: {screen}");
    // `ge` matches the prefix of `getString` and `get`, in the order the completer gave.
    let mut h = completing("", Words::default());
    h.type_text("ge");
    let screen = rows(&h);
    let a = screen
        .iter()
        .position(|l| l.contains("getString"))
        .expect("row");
    let b = screen
        .iter()
        .position(|l| l.contains(" get") && !l.contains("getString"))
        .expect("row");
    assert!(a < b, "{screen:?}");
}

#[test]
fn a_recently_chosen_item_ranks_first_among_equal_matches() {
    let mut h = completing(
        "",
        Words {
            recent: vec!["push"],
            ..Words::default()
        },
    );
    h.type_text("p");
    let screen = rows(&h);
    let push = screen
        .iter()
        .position(|l| l.contains("push"))
        .expect("push");
    let print = screen
        .iter()
        .position(|l| l.contains("print "))
        .or_else(|| screen.iter().position(|l| l.contains("print")))
        .expect("print");
    assert!(push < print, "{screen:?}");
}

#[test]
fn a_commit_character_inserts_the_item_and_is_typed_after_it() {
    let mut h = completing("", Words::default());
    h.type_text("si");
    h.type_text(".");
    assert_eq!(text(&h), "size.");
    assert!(!h.app.state.completion_open());
    assert_eq!(
        h.app.last.completed.as_ref().map(|c| c.label.as_str()),
        Some("size"),
        "the event reports the inserted item"
    );
}

#[test]
fn enter_reports_the_completed_item_and_is_one_undo_step() {
    let mut h = completing("", Words::default());
    h.type_text("pri");
    h.press(KeyCode::Enter);
    assert_eq!(text(&h), "println");
    assert_eq!(
        h.app.last.completed.as_ref().map(|c| c.label.as_str()),
        Some("println")
    );
    h.ctrl('z');
    assert_eq!(text(&h), "pri");
}

#[test]
fn a_lone_answer_to_ctrl_space_goes_in_at_once() {
    let mut h = completing("si", Words::default());
    h.press(KeyCode::End);
    h.ctrl(' ');
    assert_eq!(text(&h), "size");
    assert!(!h.app.state.completion_open());
}

#[test]
fn the_second_ctrl_space_asks_for_more() {
    let mut h = completing(
        "g",
        Words {
            more: vec!["garbage"],
            ..Words::default()
        },
    );
    h.press(KeyCode::End);
    h.ctrl(' ');
    assert!(h.app.state.completion_open());
    assert!(!h.text().contains("garbage"));
    h.ctrl(' ');
    assert!(h.text().contains("garbage"), "{}", h.text());
    assert!(*h.app.words.extended_calls.borrow() >= 1);
}

#[test]
fn the_auto_popup_delay_holds_the_list_back_until_the_tick() {
    let mut h = completing(
        "",
        Words {
            delay: 0.3,
            ..Words::default()
        },
    );
    h.type_text("p");
    assert!(!h.app.state.completion_open(), "not yet");
    assert!(h.app.state.deadline().is_some(), "a wake-up is due");
    h.advance(0.2);
    assert!(!h.app.state.completion_open());
    h.advance(0.2);
    assert!(h.app.state.completion_open(), "{}", h.text());
}

#[test]
fn backspace_on_an_empty_prefix_closes_the_list() {
    let mut h = completing("", Words::default());
    h.type_text("g");
    assert!(h.app.state.completion_open());
    h.press(KeyCode::Backspace);
    assert!(!h.app.state.completion_open());
}

#[test]
fn moving_the_caret_out_of_the_prefix_closes_the_list() {
    let mut h = completing("x", Words::default());
    h.press(KeyCode::End);
    h.type_text(" ge");
    assert!(h.app.state.completion_open(), "{}", h.text());
    h.press(KeyCode::Home);
    assert!(!h.app.state.completion_open());
}

#[test]
fn ctrl_q_shows_the_documentation_beside_the_list() {
    let mut h = completing(
        "",
        Words {
            docs: true,
            ..Words::default()
        },
    );
    h.type_text("get");
    assert!(!h.text().contains("Reads a string"));
    h.ctrl('q');
    assert!(h.text().contains("Reads a string"), "{}", h.text());
    h.ctrl('q');
    assert!(!h.text().contains("Reads a string"));
}

#[test]
fn bold_struck_out_items_and_the_tail_and_type_text_draw_as_the_presentation_says() {
    let mut h = completing("", Words::default());
    h.type_text("p");
    let (x, y) = h.find("println").expect("row");
    assert!(cell(&h, x, y).modifier.contains(Modifier::BOLD));
    let screen = h.screen();
    assert!(screen[y as usize].contains("println(x)"), "{screen:?}");
    assert!(screen[y as usize].contains("fn"), "{screen:?}");
    let (px, py) = h.find("push").expect("push row");
    assert!(cell(&h, px, py).modifier.contains(Modifier::CROSSED_OUT));
}

#[test]
fn an_advertisement_shows_under_the_rows() {
    let mut h = completing(
        "",
        Words {
            advertise: true,
            ..Words::default()
        },
    );
    h.type_text("p");
    assert!(
        h.text().contains("Press Ctrl+Space again for more"),
        "{}",
        h.text()
    );
}

#[test]
fn an_item_goes_in_at_every_caret_with_the_same_prefix() {
    let mut h = completing("pr\npr\n", Words::default());
    h.press(KeyCode::Down);
    h.press(KeyCode::End);
    h.ctrl('k');
    key(&mut h, KeyCode::Up, KeyModifiers::CONTROL);
    h.type_text("i");
    assert_eq!(text(&h), "pri\npri\n");
    h.press(KeyCode::Enter);
    assert_eq!(text(&h), "println\nprintln\n");
}

#[test]
fn a_click_picks_a_row() {
    let mut h = completing("", Words::default());
    h.type_text("p");
    let (x, y) = h.find("push").expect("push row");
    h.click(x, y);
    assert_eq!(text(&h), "push");
    assert!(!h.app.state.completion_open());
}

const MANY: &[&str] = &[
    "w00", "w01", "w02", "w03", "w04", "w05", "w06", "w07", "w08", "w09", "w10", "w11",
];

#[test]
fn the_wheel_scrolls_a_long_list_and_a_scrollbar_thumb_shows() {
    let mut h = completing(
        "",
        Words {
            more: MANY.to_vec(),
            ..Words::default()
        },
    );
    h.ctrl(' ');
    h.ctrl(' ');
    assert!(h.text().contains("getString"), "{}", h.text());
    assert!(
        !h.text().contains("w11"),
        "past the eight rows: {}",
        h.text()
    );
    let (x, y) = h.find("getString").expect("first row");
    h.scroll(x + 1, y, 12);
    assert!(h.text().contains("w11"), "{}", h.text());
    assert!(!h.text().contains("getString"), "{}", h.text());
}

#[test]
fn completion_is_off_while_the_find_prompt_has_the_focus() {
    let mut h = completing("", Words::default());
    h.ctrl('f');
    h.type_text("g");
    assert!(!h.app.state.completion_open());
}

#[test]
fn template_variables_complete_inside_braces() {
    let mut h = harness(
        "const a = `{{to}}`;\n",
        Cfg {
            variables: true,
            plain: true,
            ..Cfg::default()
        },
    );
    h.click(TEXT_X + 15, 0);
    assert_eq!(caret(&h), 15);
    h.type_text("k");
    assert!(h.app.state.completion_open(), "{}", h.text());
    assert!(h.text().contains("{{token}}"), "{}", h.text());
    assert!(
        !h.text().contains("{{host}}"),
        "narrowed by `tok`: {}",
        h.text()
    );
    h.press(KeyCode::Enter);
    assert_eq!(
        text(&h),
        "const a = `{{token}}`;\n",
        "the whole token is replaced"
    );
}

#[test]
fn template_variables_are_colored_by_what_they_resolve_to() {
    let h = harness_with(
        app(
            "a = `{{known}}` + `{{nope}}`;\n",
            Cfg {
                template: true,
                ..Cfg::default()
            },
        ),
        60,
        6,
    );
    let ed = h.rt.look.theme.editor;
    let look = &h.rt.look;
    let resolved = look.color(TextAttributesKey::Resolved.color(&ed));
    let unresolved = look.color(TextAttributesKey::Unresolved.color(&ed));
    assert_ne!(resolved, unresolved);
    // `known` starts at column 8, `nope` at column 23.
    assert_eq!(cell(&h, TEXT_X + 8, 0).fg, resolved);
    assert_eq!(cell(&h, TEXT_X + 23, 0).fg, unresolved);
}

// ==== robustness ================================================================================

#[test]
fn no_panic_with_every_option_on_tiny_sizes_and_random_events() {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for (w, hgt) in [(1u16, 1u16), (3, 2), (8, 4), (20, 6), (60, 12)] {
        let mut ed = app(
            CODE,
            Cfg {
                folding: true,
                bracket_folding: true,
                completer: true,
                wrap: true,
                whitespace: true,
                virtual_text: true,
                stick: true,
                scroll_to_line: Some(3),
                ..Cfg::default()
            },
        );
        ed.highlights = diagnostics();
        ed.icons = vec![GutterIcon::new(1, Glyph::Run, "Run")];
        ed.markers = vec![ChangeMarker::new(0..2, ChangeKind::Added)];
        let mut h = harness_with(ed, w, hgt);
        for _ in 0..300 {
            let r = next();
            let x = (r % u64::from(w.max(1))) as u16;
            let y = ((r >> 8) % u64::from(hgt.max(1))) as u16;
            match (r >> 16) % 9 {
                0 => {
                    h.hover(x, y);
                    h.advance(0.6);
                }
                1 => {
                    h.click(x, y);
                }
                2 => {
                    h.type_text(["p", "(", "\"", "g", ".", " "][(r >> 24) as usize % 6]);
                }
                3 => {
                    h.send(Event::Key(KeyEvent::new(
                        [
                            KeyCode::Enter,
                            KeyCode::Backspace,
                            KeyCode::Tab,
                            KeyCode::Down,
                            KeyCode::Esc,
                            KeyCode::F(3),
                        ][(r >> 24) as usize % 6],
                        KeyModifiers::NONE,
                    )));
                }
                4 => {
                    h.ctrl(['f', 'z', ' ', 'q', 'b', 'd'][(r >> 24) as usize % 6]);
                }
                5 => {
                    h.scroll(x, y, ((r >> 24) % 5) as i32 - 2);
                }
                6 => {
                    h.drag((x, y), (y.min(w - 1), x.min(hgt - 1)));
                }
                7 => {
                    h.resize(w.max(1), hgt.max(1));
                }
                _ => {
                    mouse(
                        &mut h,
                        MouseEventKind::Down(MouseButton::Left),
                        x,
                        y,
                        KeyModifiers::ALT,
                    );
                }
            }
        }
    }
}

// ==== snapshots (the images of docs/tui) ====================================================

fn both(name: &str, make: impl Fn(Look) -> Harness<Ed>) {
    for (look, mode) in [(Look::dark(), "dark"), (Look::light(), "light")] {
        let h = make(look);
        h.snapshot(&format!("{name}.{mode}"));
    }
}

const SAMPLE: &str = "use std::fmt;\n\n/// A point on the grid.\npub struct Point {\n    x: i32,\n    y: i32,\n}\n\nimpl Point {\n    pub fn norm(&self) -> f64 {\n        let sum = self.x * self.x + self.y * self.y;\n        (sum as f64).sqrt() // the length\n    }\n}\n";

#[test]
fn snapshot_hover_float_on_a_diagnostic() {
    both("editor-hover", |look| {
        let mut ed = app(SAMPLE, Cfg::default());
        let sum = SAMPLE.find("sum as").expect("in the sample");
        let unused = SAMPLE.find("fmt").expect("in the sample");
        ed.highlights = vec![
            RangeHighlight::new(unused..unused + 3, HighlightSeverity::Warning)
                .message("unused import: `std::fmt`"),
            RangeHighlight::new(sum..sum + 3, HighlightSeverity::Error)
                .message("mismatched types: expected `f64`, found `i32`"),
            RangeHighlight::new(sum..sum + 3, HighlightSeverity::Warning)
                .message("value assigned to `sum` is never read"),
        ];
        let mut h = harness_look(ed, look, 64, 20);
        h.click(TEXT_X, 14);
        h.hover(TEXT_X + 10, 11);
        h.advance(0.6);
        h
    });
}

#[test]
fn snapshot_fold_preview() {
    both("editor-fold-preview", |look| {
        let mut ed = app(
            SAMPLE,
            Cfg {
                folding: true,
                bracket_folding: true,
                ..Cfg::default()
            },
        );
        let mut h = harness_look(
            std::mem::replace(&mut ed, app("", Cfg::default())),
            look,
            64,
            14,
        );
        // Collapse `impl Point { … }`: the second region, on line 8.
        h.click(FOLD_X, 8);
        let (x, y) = h.find("{…}").expect("placeholder");
        h.hover(x + 1, y);
        h.advance(0.4);
        h
    });
}

#[test]
fn snapshot_completion_with_documentation() {
    both("editor-completion-docs", |look| {
        let mut ed = app(
            SAMPLE,
            Cfg {
                completer: true,
                ..Cfg::default()
            },
        );
        ed.words = Words {
            docs: true,
            advertise: true,
            ..Words::default()
        };
        let mut h = harness_look(ed, look, 72, 16);
        h.click(TEXT_X + 44, 10);
        key(&mut h, KeyCode::End, KeyModifiers::NONE);
        key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
        h.type_text("g");
        h.ctrl('q');
        h
    });
}

#[test]
fn snapshot_find_with_a_distinct_current_match() {
    both("editor-find-current", |look| {
        let mut h = harness_look(app(SAMPLE, Cfg::default()), look, 64, 14);
        h.ctrl('f');
        h.type_text("self");
        key(&mut h, KeyCode::Enter, KeyModifiers::NONE);
        h
    });
}

#[test]
fn snapshot_brace_match_and_indent_guides() {
    both("editor-braces", |look| {
        let mut h = harness_look(app(SAMPLE, Cfg::default()), look, 64, 15);
        // Right after the `(` of `.sqrt()`'s receiver: `(sum as f64)` is a pair.
        h.click(TEXT_X + 20, 11);
        h
    });
}
