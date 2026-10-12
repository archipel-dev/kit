//! [`CodeEditor`]: multi-line code editing on the caller's `String`.

use super::find::{self, FindState};
use super::highlight::{Highlighter, PlainText, Span, styled_job, token_format};
use super::text::{self, MAX_MATCHES};
use super::{code_font, gutter_width, paint_background, paint_border, paint_number};
use crate::parts::desktop;
use crate::theme::{EditorColors, Theme};
use crate::tokens::size;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{
    Event, EventFilter, Id, Key, Margin, Modifiers, Rect, Response, RichText, Sense, Shape,
    TextBuffer, TextEdit, Ui, UiBuilder, Widget, WidgetInfo, pos2, vec2,
};
use std::ops::Range;
use std::sync::Arc;

/// A multi-line code editor on the caller's `String`.
///
/// It looks like a [`CodeView`](super::CodeView) (same font, gutter and colors) and edits
/// the text in place:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::CodeEditor;
/// use archipel_kit::components::code::Json;
///
/// let mut body = String::from("{\n  \"name\": \"archipel\"\n}");
/// let response = ui.add(
///     CodeEditor::new(&mut body)
///         .label("Request body")
///         .highlighter(&Json)
///         .line_numbers(true)
///         .height(160.0),
/// );
/// if response.changed() { /* the body was edited */ }
/// # });
/// ```
///
/// - <kbd>Tab</kbd> inserts spaces up to the next tab stop ([`tab_width`](CodeEditor::tab_width),
///   or a tab character with [`use_tabs`](CodeEditor::use_tabs)); with a multi-line
///   selection it indents the lines, <kbd>Shift</kbd>+<kbd>Tab</kbd> unindents. The editor
///   keeps focus on <kbd>Tab</kbd>: <kbd>Esc</kbd> leaves it.
/// - <kbd>Enter</kbd> keeps the indentation of the line, and indents one level more after
///   `{`, `[` or `(` (splitting an empty pair onto three lines).
/// - Typing `(`, `[`, `{`, `"`, `'` or `` ` `` inserts the pair, typing the closer skips
///   over it, backspace in an empty pair removes both, and typing an opener over a
///   selection wraps it. Turn it off with [`bracket_pairing`](CodeEditor::bracket_pairing).
/// - <kbd>Ctrl</kbd>+<kbd>F</kbd> opens the find bar, like the view's.
/// - <kbd>Ctrl</kbd>+<kbd>]</kbd> and <kbd>Ctrl</kbd>+<kbd>[</kbd> indent and unindent lines.
/// - The kit's [`CodeView`](super::CodeView) is for large texts. The editor is built on
///   egui's `TextEdit`, which lays out the **whole** text whenever it changes: it stays
///   responsive up to about **1 MB** (roughly 20 000 lines), after which every
///   keystroke costs noticeable time. Larger inputs belong in a view, or behind a
///   "too large to edit" notice.
#[must_use = "add it with `ui.add(editor)`"]
pub struct CodeEditor<'a> {
    text: &'a mut String,
    label: Option<String>,
    placeholder: Option<String>,
    highlighter: &'a dyn Highlighter,
    line_numbers: bool,
    wrap: bool,
    enabled: bool,
    height: Option<f32>,
    id: Option<Id>,
    config: Config,
    completer: Option<&'a dyn super::super::Completer>,
}

/// How typing behaves.
#[derive(Clone, Copy)]
struct Config {
    tab_width: usize,
    use_tabs: bool,
    auto_indent: bool,
    pairing: bool,
    tab_indents: bool,
}

impl<'a> CodeEditor<'a> {
    /// An editor on `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            label: None,
            placeholder: None,
            highlighter: &PlainText,
            line_numbers: false,
            wrap: false,
            enabled: true,
            height: None,
            id: None,
            completer: None,
            config: Config {
                tab_width: 4,
                use_tabs: false,
                auto_indent: true,
                pairing: true,
                tab_indents: true,
            },
        }
    }

    /// Offers completions while typing, in a list under the caret (fields of a GraphQL
    /// schema, keywords). <kbd>Ctrl</kbd>+<kbd>Space</kbd> opens it explicitly; while it is
    /// open <kbd>Up</kbd>/<kbd>Down</kbd> move, <kbd>Enter</kbd> or <kbd>Tab</kbd> insert
    /// and <kbd>Esc</kbd> closes. See [`TextField::completer`](super::super::TextField::completer).
    pub fn completer(mut self, completer: &'a dyn super::super::Completer) -> Self {
        self.completer = Some(completer);
        self
    }

    /// The accessible name of the editor ("Request body").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Hint shown while the text is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// The [`Highlighter`] that colors the text (default: none).
    pub fn highlighter(mut self, highlighter: &'a dyn Highlighter) -> Self {
        self.highlighter = highlighter;
        self
    }

    /// Shows line numbers in a gutter.
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// Wraps long lines at the editor's width instead of scrolling sideways.
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Disabled editors are dimmed, read-only and skipped by <kbd>Tab</kbd>.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Columns per indentation level (default 4).
    pub fn tab_width(mut self, width: usize) -> Self {
        self.config.tab_width = width.clamp(1, 16);
        self
    }

    /// Indents with tab characters instead of spaces.
    pub fn use_tabs(mut self, on: bool) -> Self {
        self.config.use_tabs = on;
        self
    }

    /// <kbd>Enter</kbd> keeps the indentation of the current line (default on).
    pub fn auto_indent(mut self, on: bool) -> Self {
        self.config.auto_indent = on;
        self
    }

    /// Inserts closing brackets and quotes (default on).
    pub fn bracket_pairing(mut self, on: bool) -> Self {
        self.config.pairing = on;
        self
    }

    /// <kbd>Tab</kbd> indents (default on). Turn it off in forms, where <kbd>Tab</kbd> must
    /// move to the next control; <kbd>Ctrl</kbd>+<kbd>]</kbd> and <kbd>Ctrl</kbd>+<kbd>[</kbd>
    /// indent and unindent the selected lines either way.
    pub fn tab_indents(mut self, on: bool) -> Self {
        self.config.tab_indents = on;
        self
    }

    /// Height in points (default: the available height, or 300 when it is unbounded).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Gives the editor an explicit id, for several editors sharing one place in the ui.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
}

/// The galley of the last layout, reused while text, wrap width and colors stay the same.
#[derive(Clone)]
struct GalleyCache {
    key: u64,
    galley: Arc<egui::Galley>,
}

/// Matches of the find bar with their char offsets (what the galley is addressed by).
#[derive(Clone)]
struct Found {
    key: u64,
    matches: Arc<Vec<(Range<usize>, Range<usize>)>>,
}

/// What [`CodeEditor::show`] reports besides the usual [`Response`].
#[non_exhaustive]
pub struct CodeEditorOutput {
    /// The editor's response: `changed()` after every edit.
    pub response: Response,
    /// The selected text as a byte range of the text, `None` when nothing is selected (only a
    /// caret, or the editor was never focused). A query editor runs just the selection.
    pub selection: Option<Range<usize>>,
}

impl CodeEditor<'_> {
    /// Adds the editor and reports the selection besides the response.
    pub fn show(self, ui: &mut Ui) -> CodeEditorOutput {
        let (response, text) = self.run(ui);
        let selection = egui::text_edit::TextEditState::load(ui.ctx(), response.id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| {
                let [a, b] = range.sorted_cursors();
                let byte = |chars: usize| {
                    text.char_indices()
                        .nth(chars)
                        .map_or(text.len(), |(byte, _)| byte)
                };
                byte(a.index)..byte(b.index)
            })
            .filter(|range| !range.is_empty());
        CodeEditorOutput {
            response,
            selection,
        }
    }
}

impl Widget for CodeEditor<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.run(ui).0
    }
}

impl<'a> CodeEditor<'a> {
    /// Draws the editor; returns its response and the text, as edited.
    fn run(self, ui: &mut Ui) -> (Response, &'a mut String) {
        let ctx = ui.ctx().clone();
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        ui.skip_ahead_auto_ids(1);
        let theme = Theme::current(&ctx);
        let editor = theme.editor;
        let (font, cell) = code_font(ui);
        let line_height = size::CODE_LINE_HEIGHT;
        let pad = size::CODE_PADDING;
        let enabled = self.enabled && ui.is_enabled();
        let wrap = self.wrap;
        let config = self.config;
        let highlighter = self.highlighter;

        let height = self.height.unwrap_or_else(|| {
            let h = ui.available_height();
            if h.is_finite() && (40.0..10_000.0).contains(&h) {
                h
            } else {
                300.0
            }
        });
        // A layout that has run out of room must not make egui allocate a negative size.
        let size = vec2(ui.available_width(), height).max(egui::Vec2::ZERO);
        let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
        paint_background(ui, outer);
        let inner = outer.shrink(1.0);
        let lines = self.text.bytes().filter(|&b| b == b'\n').count() + 1;
        let gutter_w = gutter_width(lines, cell, self.line_numbers, false);
        let gutter = Rect::from_min_max(inner.min, pos2(inner.min.x + gutter_w, inner.max.y));
        let area = Rect::from_min_max(pos2(inner.min.x + gutter_w, inner.min.y), inner.max);

        // Keys that edit: before the TextEdit sees them.
        let focused = enabled && ctx.memory(|m| m.has_focus(id));
        let mut find = FindState::load(&ctx, id);
        find.keys(ui, focused);
        // An open completion list takes its keys first: Enter inserts, it does not indent.
        let (list, _) =
            super::super::completion::List::begin(ui, id, focused && self.completer.is_some());
        let mut edited = false;
        if focused {
            edited = edit_keys(ui, id, self.text, config);
        }

        // The text, hashed once per frame: the key of every cache below.
        let text_key = egui::util::hash(self.text.as_str());
        let found = if find.open && !find.query.is_empty() {
            Some(matches(&ctx, id, self.text, text_key, &find.query))
        } else {
            None
        };
        let match_count = found.as_ref().map_or(0, |f| f.matches.len());
        if match_count > 0 {
            find.current = find.current.min(match_count - 1);
        }

        let style_key = egui::util::hash((
            enabled,
            highlighter.name(),
            editor.text,
            editor.keyword,
            editor.string,
            editor.comment,
            font.size.to_bits(),
        ));
        let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| -> Arc<egui::Galley> {
            let wrap_width = if wrap { wrap_width } else { f32::INFINITY };
            let key = egui::util::hash((text_key, wrap_width.to_bits(), style_key));
            let slot = id.with("archipel-code-galley");
            if let Some(cache) = ui.data(|d| d.get_temp::<GalleyCache>(slot))
                && cache.key == key
            {
                return cache.galley;
            }
            let galley = layout(
                ui,
                buf.as_str(),
                highlighter,
                &font,
                &editor,
                wrap_width,
                enabled,
            );
            ui.data_mut(|d| {
                d.insert_temp(
                    slot,
                    GalleyCache {
                        key,
                        galley: galley.clone(),
                    },
                )
            });
            galley
        };

        // Scroll area around the text edit.
        let mut child = ui.new_child(UiBuilder::new().max_rect(area));
        child.set_clip_rect(area.intersect(ui.clip_rect()));
        child.scope(|ui| {
            // Selection and caret colors of the editor.
            ui.visuals_mut().selection.bg_fill = editor.selection;
        });
        let mut gutter_rows: Vec<(f32, usize, bool)> = Vec::new();
        let rows_in_view = (area.height() / line_height).ceil() as usize;
        let jump = std::mem::take(&mut find.jump);
        let mut response = None;
        let mut caret = None;
        desktop(egui::ScrollArea::new([!wrap, true]))
            .id_salt(id.with("scroll"))
            .auto_shrink(false)
            .show(&mut child, |ui| {
                ui.visuals_mut().selection.bg_fill = editor.selection;
                let backdrop = ui.painter().add(Shape::Noop);
                let mut edit = TextEdit::multiline(self.text)
                    .id(id)
                    .frame(false)
                    .font(egui::TextStyle::Monospace)
                    .margin(Margin::symmetric(pad as i8, (pad / 2.0) as i8))
                    .desired_width(area.width())
                    .desired_rows(rows_in_view.max(1))
                    .lock_focus(config.tab_indents)
                    .interactive(enabled)
                    .layouter(&mut layouter);
                if let Some(placeholder) = &self.placeholder {
                    edit = edit.hint_text(
                        RichText::new(placeholder).color(Theme::colors(ui.ctx()).text_placeholder),
                    );
                }
                let output = edit.show(ui);
                let galley = &output.galley;
                let origin = output.galley_pos;
                let visible = ui.clip_rect();

                let mut shapes = Vec::new();
                // The caret row.
                if let Some(range) = output.cursor_range
                    && enabled
                    && range.is_empty()
                {
                    let row = galley.layout_from_cursor(range.primary).row;
                    if let Some(placed) = galley.rows.get(row) {
                        let y = origin.y + placed.pos.y;
                        shapes.push(Shape::rect_filled(
                            Rect::from_min_max(
                                pos2(visible.left(), y),
                                pos2(visible.right(), y + placed.row.height()),
                            ),
                            0.0,
                            editor.caret_row,
                        ));
                    }
                }
                // Find matches.
                if let Some(found) = &found {
                    let low = galley
                        .cursor_from_pos(vec2(0.0, visible.top() - origin.y))
                        .index;
                    let high = galley
                        .cursor_from_pos(vec2(0.0, visible.bottom() - origin.y))
                        .index;
                    let from = found.matches.partition_point(|(_, c)| c.end < low);
                    for (i, (_, chars)) in found.matches[from..].iter().enumerate() {
                        if chars.start > high {
                            break;
                        }
                        let index = from + i;
                        let a = galley.pos_from_cursor(CCursor::new(chars.start));
                        let b = galley.pos_from_cursor(CCursor::new(chars.end));
                        let color = if index == find.current {
                            editor.search_result_current
                        } else {
                            editor.search_result
                        };
                        let rect_of = |x0: f32, x1: f32, row: Rect| {
                            Rect::from_min_max(
                                pos2(origin.x + x0, origin.y + row.min.y),
                                pos2(origin.x + x1, origin.y + row.max.y),
                            )
                        };
                        if (a.min.y - b.min.y).abs() < 1.0 {
                            shapes.push(Shape::rect_filled(
                                rect_of(a.min.x, b.min.x, a),
                                2.0,
                                color,
                            ));
                        } else {
                            shapes.push(Shape::rect_filled(
                                rect_of(a.min.x, galley.size().x, a),
                                2.0,
                                color,
                            ));
                            shapes.push(Shape::rect_filled(rect_of(0.0, b.min.x, b), 2.0, color));
                        }
                        if index == find.current && jump {
                            ui.scroll_to_rect(
                                rect_of(a.min.x, b.min.x.max(a.min.x + cell), a)
                                    .expand(line_height),
                                None,
                            );
                        }
                    }
                }
                ui.painter().set(backdrop, Shape::Vec(shapes));

                // Line numbers: one per galley row that starts a line.
                let caret_row = output
                    .cursor_range
                    .map(|r| galley.layout_from_cursor(r.primary).row);
                let mut line = 0;
                let mut starts_line = true;
                for (row_index, placed) in galley.rows.iter().enumerate() {
                    let y = origin.y + placed.pos.y;
                    if starts_line
                        && y + placed.row.height() >= visible.top()
                        && y <= visible.bottom()
                    {
                        gutter_rows.push((
                            y + placed.row.height() / 2.0,
                            line,
                            caret_row == Some(row_index),
                        ));
                    }
                    starts_line = placed.row.ends_with_newline;
                    if starts_line {
                        line += 1;
                    }
                }
                caret = output.cursor_range.map(|range| {
                    let r = galley.pos_from_cursor(range.primary);
                    let rect = Rect::from_min_max(
                        pos2(r.min.x, r.min.y) + origin.to_vec2(),
                        pos2(r.min.x + 1.0, r.max.y) + origin.to_vec2(),
                    );
                    (range.primary.index, rect)
                });
                response = Some(output.response);
            });
        let mut response = response.expect("scroll area ran its content");
        if edited {
            response.mark_changed();
        }
        if let Some(completer) = self.completer {
            list.end(ui, completer, self.text, &mut response, caret);
        }

        // Gutter outside the scroll area, so sideways scrolling leaves it in place.
        if gutter_w > 0.0 {
            let painter = ui
                .painter()
                .with_clip_rect(gutter.intersect(ui.clip_rect()));
            for (y, line, caret) in gutter_rows {
                let color = if caret && enabled {
                    editor.line_number_caret
                } else {
                    editor.line_number
                };
                paint_number(&painter, gutter, y, line + 1, &font, color, false);
            }
            if ui.rect_contains_pointer(gutter) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
            }
        }

        // Keep Tab and Escape for the editor (Escape closes the find bar while it is open).
        if focused {
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        tab: config.tab_indents,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: find.open,
                    },
                );
            });
        }

        if find.open {
            let events = find::bar(ui, area, &mut find, match_count);
            if events.query_changed || events.step != 0 {
                ctx.request_repaint();
            }
        }
        if !find.open && find.field_focused(&ctx) {
            response.request_focus();
        }
        find.store(&ctx, id);

        paint_border(ui, outer, response.has_focus());
        let label = self.label.clone().unwrap_or_else(|| "Code".to_owned());
        response.widget_info(|| {
            WidgetInfo::text_edit(enabled, String::new(), String::new(), label.clone())
        });
        ctx.accesskit_node_builder(response.id, |node| node.set_label(label.clone()));
        (response, self.text)
    }
}

/// Matches of `query` in `text` with their char offsets, cached per text and query.
fn matches(ctx: &egui::Context, id: Id, text: &str, text_key: u64, query: &str) -> Found {
    let key = egui::util::hash((text_key, query));
    let slot = id.with("archipel-code-found");
    if let Some(found) = ctx.data(|d| d.get_temp::<Found>(slot))
        && found.key == key
    {
        return found;
    }
    let mut out = Vec::new();
    let (mut byte, mut chars) = (0, 0);
    for range in text::search(text, query).into_iter().take(MAX_MATCHES) {
        chars += text[byte..range.start].chars().count();
        let len = text[range.clone()].chars().count();
        out.push((range.clone(), chars..chars + len));
        chars += len;
        byte = range.end;
    }
    let found = Found {
        key,
        matches: Arc::new(out),
    };
    ctx.data_mut(|d| d.insert_temp(slot, found.clone()));
    found
}

/// The galley of `text`, colored by `highlighter`.
fn layout(
    ui: &Ui,
    text: &str,
    highlighter: &dyn Highlighter,
    font: &egui::FontId,
    colors: &EditorColors,
    wrap_width: f32,
    enabled: bool,
) -> Arc<egui::Galley> {
    let mut spans: Vec<Span> = Vec::new();
    highlighter.highlight_text(text, &mut spans);
    let job = styled_job(text, 0, &spans, wrap_width, |kind| {
        token_format(kind, font, colors, Some(size::CODE_LINE_HEIGHT), enabled)
    });
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// Applies the editing keys of this frame to `text`; returns whether it changed.
fn edit_keys(ui: &Ui, id: Id, text: &mut String, config: Config) -> bool {
    let ctx = ui.ctx();
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    let chars = text.chars().count();
    let range = state
        .cursor
        .char_range()
        .unwrap_or(CCursorRange::one(CCursor::new(chars)));
    let to_byte = |text: &str, c: usize| text.char_indices().nth(c).map_or(text.len(), |(b, _)| b);
    let mut sel = {
        let (a, b) = (
            range.secondary.index.min(chars),
            range.primary.index.min(chars),
        );
        (to_byte(text, a), to_byte(text, b))
    };
    let mut changed = false;
    ui.input_mut(|input| {
        let mut events = std::mem::take(&mut input.events);
        events.retain(|event| {
            let (lo, hi) = (sel.0.min(sel.1), sel.0.max(sel.1));
            let forward = sel.0 <= sel.1;
            let new = match event {
                Event::Key {
                    key: Key::Tab,
                    pressed: true,
                    modifiers,
                    ..
                } if config.tab_indents && !modifiers.command && !modifiers.alt => {
                    Some(tab(text, (lo, hi), config, modifiers.shift, false))
                }
                Event::Key {
                    key: Key::CloseBracket,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => Some(tab(text, (lo, hi), config, false, true)),
                Event::Key {
                    key: Key::OpenBracket,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => Some(tab(text, (lo, hi), config, true, true)),
                Event::Key {
                    key: Key::Enter,
                    pressed: true,
                    modifiers,
                    ..
                } if config.auto_indent && *modifiers == Modifiers::NONE => {
                    Some(enter(text, (lo, hi), config))
                }
                Event::Key {
                    key: Key::Backspace,
                    pressed: true,
                    modifiers,
                    ..
                } if config.pairing && *modifiers == Modifiers::NONE => {
                    backspace_pair(text, (lo, hi))
                }
                Event::Text(typed) if config.pairing && typed.chars().count() == 1 => {
                    type_char(text, (lo, hi), typed.chars().next().unwrap_or(' '))
                }
                _ => None,
            };
            match new {
                Some((a, b)) => {
                    sel = if forward || a == b { (a, b) } else { (b, a) };
                    changed = true;
                    false
                }
                None => true,
            }
        });
        input.events = events;
    });
    if changed {
        let to_char = |text: &str, b: usize| text[..b.min(text.len())].chars().count();
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(to_char(text, sel.0)),
            CCursor::new(to_char(text, sel.1)),
        )));
        state.store(ctx, id);
        ctx.request_repaint();
    }
    changed
}

fn line_start(text: &str, byte: usize) -> usize {
    text[..byte].rfind('\n').map_or(0, |i| i + 1)
}

fn leading_whitespace(line: &str) -> &str {
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}

/// One indentation level.
fn unit(config: Config) -> String {
    if config.use_tabs {
        "\t".to_owned()
    } else {
        " ".repeat(config.tab_width)
    }
}

/// Columns before `byte` on its line, tabs counting up to the tab stop.
fn column(text: &str, byte: usize, tab_width: usize) -> usize {
    text[line_start(text, byte)..byte]
        .chars()
        .fold(0, |col, c| {
            if c == '\t' {
                col + tab_width - col % tab_width
            } else {
                col + 1
            }
        })
}

/// <kbd>Tab</kbd> (or <kbd>Shift</kbd>+<kbd>Tab</kbd> with `back`) over the selection
/// `sel` (byte offsets); returns the new selection. `lines` indents whole lines even for
/// a selection within one.
fn tab(
    text: &mut String,
    sel: (usize, usize),
    config: Config,
    back: bool,
    lines: bool,
) -> (usize, usize) {
    let (lo, hi) = sel;
    let multi = lines || text[lo..hi].contains('\n');
    if !back && !multi {
        let col = column(text, lo, config.tab_width);
        let indent = if config.use_tabs {
            "\t".to_owned()
        } else {
            " ".repeat(config.tab_width - col % config.tab_width)
        };
        text.replace_range(lo..hi, &indent);
        return (lo + indent.len(), lo + indent.len());
    }
    // Whole lines from the one holding `lo` to the one holding `hi` (not counting a `hi`
    // at the very start of a line).
    let first = line_start(text, lo);
    let end = if hi > lo && text[..hi].ends_with('\n') {
        hi - 1
    } else {
        hi
    };
    let last_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    let block = text[first..last_end].to_owned();
    let unit = unit(config);
    let mut out = String::new();
    // Where `lo` and `hi` land: inside an edited line they move with its start, past the
    // block by the total change.
    let (mut new_lo, mut new_hi) = (None, None);
    let mut total: isize = 0;
    let mut at = first;
    for (n, line) in block.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let (edited, delta): (String, isize) = if back {
            let ws = leading_whitespace(line);
            let remove = if ws.starts_with('\t') {
                1
            } else {
                ws.chars()
                    .take(config.tab_width)
                    .take_while(|&c| c == ' ')
                    .count()
            };
            (line[remove..].to_owned(), -(remove as isize))
        } else if line.is_empty() {
            (String::new(), 0)
        } else {
            (format!("{unit}{line}"), unit.len() as isize)
        };
        let new_start = (first + out.len()) as isize;
        let map = |pos: usize| -> usize {
            let col = (pos - at) as isize;
            let moved = if back {
                (col + delta).max(0)
            } else {
                col + delta
            };
            (new_start + moved) as usize
        };
        if (at..=at + line.len()).contains(&lo) {
            new_lo = Some(map(lo));
        }
        if (at..=at + line.len()).contains(&hi) {
            new_hi = Some(map(hi));
        }
        out.push_str(&edited);
        total += delta;
        at += line.len() + 1;
    }
    text.replace_range(first..last_end, &out);
    let past = |pos: usize| (pos as isize + total) as usize;
    let new_lo = new_lo.unwrap_or_else(|| past(lo));
    (new_lo, new_hi.unwrap_or_else(|| past(hi)).max(new_lo))
}

/// <kbd>Enter</kbd>: a line break that keeps the indentation.
fn enter(text: &mut String, sel: (usize, usize), config: Config) -> (usize, usize) {
    let (lo, hi) = sel;
    text.replace_range(lo..hi, "");
    let prefix = &text[line_start(text, lo)..lo];
    let indent = leading_whitespace(prefix).to_owned();
    let opener = prefix
        .trim_end()
        .chars()
        .last()
        .filter(|c| matches!(c, '{' | '[' | '('));
    let next = text[lo..].chars().next();
    let deeper = if opener.is_some() {
        unit(config)
    } else {
        String::new()
    };
    let mut insert = format!("\n{indent}{deeper}");
    let cursor = lo + insert.len();
    if let (Some(open), Some(next)) = (opener, next)
        && closer(open) == Some(next)
    {
        insert.push('\n');
        insert.push_str(&indent);
    }
    text.insert_str(lo, &insert);
    (cursor, cursor)
}

fn closer(open: char) -> Option<char> {
    Some(match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '"' => '"',
        '\'' => '\'',
        '`' => '`',
        _ => return None,
    })
}

fn is_quote(c: char) -> bool {
    matches!(c, '"' | '\'' | '`')
}

/// Typing `c`: pairs openers, wraps selections and skips over closers. `None` leaves it
/// to the text edit.
fn type_char(text: &mut String, sel: (usize, usize), c: char) -> Option<(usize, usize)> {
    let (lo, hi) = sel;
    let next = text[hi..].chars().next();
    let prev = text[..lo].chars().last();
    if let Some(close) = closer(c) {
        if lo != hi {
            let inner = text[lo..hi].to_owned();
            text.replace_range(lo..hi, &format!("{c}{inner}{close}"));
            return Some((lo + c.len_utf8(), hi + c.len_utf8()));
        }
        if is_quote(c) {
            if next == Some(c) {
                return Some((lo + 1, lo + 1));
            }
            if prev.is_some_and(|p| p.is_alphanumeric() || p == c)
                || next.is_some_and(char::is_alphanumeric)
            {
                return None;
            }
        } else if next.is_some_and(char::is_alphanumeric) {
            return None;
        }
        text.insert_str(lo, &format!("{c}{close}"));
        return Some((lo + c.len_utf8(), lo + c.len_utf8()));
    }
    if lo == hi && matches!(c, ')' | ']' | '}') && next == Some(c) {
        return Some((lo + 1, lo + 1));
    }
    None
}

/// Backspace between an empty pair removes both.
fn backspace_pair(text: &mut String, sel: (usize, usize)) -> Option<(usize, usize)> {
    let (lo, hi) = sel;
    if lo != hi {
        return None;
    }
    let prev = text[..lo].chars().last()?;
    let next = text[lo..].chars().next()?;
    if closer(prev) == Some(next) {
        let start = lo - prev.len_utf8();
        text.replace_range(start..lo + next.len_utf8(), "");
        return Some((start, start));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: Config = Config {
        tab_width: 4,
        use_tabs: false,
        auto_indent: true,
        pairing: true,
        tab_indents: true,
    };

    #[test]
    fn tab_inserts_spaces_to_the_next_stop() {
        let mut text = "ab".to_owned();
        assert_eq!(tab(&mut text, (2, 2), CONFIG, false, false), (4, 4));
        assert_eq!(text, "ab  ");
    }

    #[test]
    fn tab_indents_and_untabs_selected_lines() {
        let mut text = "a\nb\nc".to_owned();
        let sel = tab(&mut text, (0, 3), CONFIG, false, false);
        assert_eq!(text, "    a\n    b\nc");
        assert_eq!(sel, (4, 11));
        let sel = tab(&mut text, sel, CONFIG, true, false);
        assert_eq!(text, "a\nb\nc");
        assert_eq!(sel, (0, 3));
    }

    #[test]
    fn enter_keeps_and_deepens_the_indent() {
        let mut text = "  if x {".to_owned();
        let (at, _) = enter(&mut text, (8, 8), CONFIG);
        assert_eq!(text, "  if x {\n      ");
        assert_eq!(at, text.len());
        let mut text = "{}".to_owned();
        let (at, _) = enter(&mut text, (1, 1), CONFIG);
        assert_eq!(text, "{\n    \n}");
        assert_eq!(at, 6);
    }

    #[test]
    fn pairs_wrap_skip_and_delete() {
        let mut text = String::new();
        assert_eq!(type_char(&mut text, (0, 0), '('), Some((1, 1)));
        assert_eq!(text, "()");
        assert_eq!(type_char(&mut text, (1, 1), ')'), Some((2, 2)));
        assert_eq!(text, "()");
        assert_eq!(backspace_pair(&mut text, (1, 1)), Some((0, 0)));
        assert_eq!(text, "");
        let mut text = "abc".to_owned();
        assert_eq!(type_char(&mut text, (0, 3), '"'), Some((1, 4)));
        assert_eq!(text, "\"abc\"");
        // An apostrophe inside a word is just an apostrophe.
        let mut text = "don".to_owned();
        assert_eq!(type_char(&mut text, (3, 3), '\''), None);
    }

    #[test]
    fn find_is_case_insensitive_for_ascii() {
        assert_eq!(text::search("Foo foo FOO", "foo"), vec![0..3, 4..7, 8..11]);
        assert!(text::search("abc", "").is_empty());
    }
}
