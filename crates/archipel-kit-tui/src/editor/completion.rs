//! The completion menu: an nvim-cmp style popup under the caret (a kind icon, the matched
//! characters highlighted, the tail text and the type in grey, the selected item filled with
//! the accent) that behaves like the IntelliJ lookup, because it runs the same state machine
//! as the GUI list ([`LookupState`]): camel-hump matching and ranking, recently chosen items
//! first, a lone answer to Ctrl+Space inserted at once, a second Ctrl+Space asking for more,
//! Backspace closing on an empty prefix, the caret leaving the prefix closing it.
//!
//! Keys while it is open: Up and Down move and wrap, PageUp and PageDown jump, Enter inserts
//! the item, Tab replaces (the identifier after the caret too), `.` `,` `;` `=` space `:` `(`
//! insert the item and are typed after it, Ctrl+Q (Ctrl+J, F1) shows the documentation of the
//! selected item, Escape closes. The item goes in at every caret as one undo step
//! ([`insert_chosen`]).

use super::{EditorOutput, EditorState, Opts};
use crate::components::{clip, fill, put, text_width, wrap_text};
use crate::glyphs;
use crate::input::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crate::runtime::Runtime;
use crate::{Glyph, SymbolKind};
use archipel_kit::editor::logic::lookup::{
    COMMIT_CHARS, LookupFrame, LookupInput, LookupKeys, insert_chosen,
};
use archipel_kit::icons::Icon;
use archipel_kit::ui::Completion;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// The most rows the menu shows at once.
pub(crate) const MAX_ROWS: usize = 8;
/// The widest the documentation box beside the menu gets, in cells.
const DOC_WIDTH: u16 = 44;

/// An open menu: what the shared list answered, and where it was drawn.
#[derive(Clone, Debug)]
pub(crate) struct Menu {
    pub frame: LookupFrame,
    /// The byte offset where the typed prefix starts (the menu hangs from there).
    pub anchor: usize,
    /// The rectangle of the last frame.
    pub rect: Rect,
}

/// The icon kind of a suggestion: calls are functions, `Capitalized` names are types,
/// `UPPER_CASE` names constants, the rest fields.
pub fn default_kind(item: &Completion) -> SymbolKind {
    let label = item.label.as_str();
    if label.starts_with("{{") {
        SymbolKind::Variable
    } else if item.insert.contains('(') || label.ends_with(')') {
        SymbolKind::Function
    } else if label.chars().next().is_some_and(char::is_uppercase)
        && label.chars().any(char::is_lowercase)
    {
        SymbolKind::Struct
    } else if label.len() > 1
        && label
            .chars()
            .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
    {
        SymbolKind::Constant
    } else {
        SymbolKind::Field
    }
}

/// What a row's icon cell shows.
enum Mark {
    Symbol(SymbolKind),
    Glyph(Glyph),
}

fn mark_of(item: &Completion) -> Mark {
    match item.icon {
        Some(Icon::Symbol(symbol)) => Mark::Symbol(symbol.kind),
        Some(Icon::Glyph(glyph)) => Mark::Glyph(glyph),
        _ => Mark::Symbol(default_kind(item)),
    }
}

fn byte_of(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(b, _)| b)
}

fn caret_chars(editor: &archipel_kit::editor::model::Editor) -> usize {
    let offset = editor.carets.primary().offset;
    editor.document.text()[..offset].chars().count()
}

impl EditorState {
    /// Closes the menu and the list behind it.
    pub(crate) fn close_menu(&mut self) {
        self.lookup.close();
        self.menu = None;
        self.lookup_wake = None;
    }
}

/// Runs one step of the shared list with `keys`; `changed` is whether the text changed since
/// the last step. Inserts the chosen item.
pub(crate) fn feed(
    opts: &Opts<'_>,
    editor: &mut archipel_kit::editor::model::Editor,
    rt: &Runtime,
    st: &mut EditorState,
    keys: LookupKeys,
    changed: bool,
    out: &mut EditorOutput,
) {
    let Some(completer) = opts.completer else {
        return;
    };
    if opts.viewer || !opts.enabled {
        st.close_menu();
        return;
    }
    let focused = rt.focused(opts.id) && st.find.focus == super::find::FindFocus::Editor;
    let frame = {
        let text = editor.document.text();
        let caret = caret_chars(editor);
        st.lookup.update(
            completer,
            &LookupInput {
                text,
                caret: Some(caret),
                keys,
                lookup: completer.filters_by_prefix(),
                changed,
                focused,
                clicking: false,
                now: rt.now,
                max_rows: MAX_ROWS,
            },
        )
    };
    st.lookup_wake = frame.repaint_after.map(|after| rt.now + after);
    if let Some(chosen) = &frame.chosen {
        insert_chosen(editor, opts.language, chosen);
        out.completed = Some(chosen.item.clone());
        out.text_changed = true;
        out.consumed = true;
        st.menu = None;
        super::find::after_user_move(editor, st);
        super::run_scroll_to_caret(editor, st);
        return;
    }
    if frame.open && frame.found.is_some() {
        let rect = st.menu.as_ref().map(|m| m.rect).unwrap_or_default();
        let anchor = byte_of(editor.document.text(), st.lookup.anchor);
        st.menu = Some(Menu {
            frame,
            anchor,
            rect,
        });
    } else {
        st.menu = None;
    }
}

/// Ctrl+Space: asks the completer for everything that fits at the caret (twice: for more).
pub(crate) fn request(
    opts: &Opts<'_>,
    editor: &mut archipel_kit::editor::model::Editor,
    rt: &Runtime,
    st: &mut EditorState,
    out: &mut EditorOutput,
) {
    let keys = LookupKeys {
        explicit: true,
        ..LookupKeys::default()
    };
    feed(opts, editor, rt, st, keys, false, out);
}

/// After an event ran: typing an identifier character opens or narrows the list, anything
/// else that moved the caret may close it (the shared list decides).
pub(crate) fn after_event(
    opts: &Opts<'_>,
    editor: &mut archipel_kit::editor::model::Editor,
    rt: &Runtime,
    st: &mut EditorState,
    event: &Event,
    text_changed: bool,
    out: &mut EditorOutput,
) {
    if opts.completer.is_none() {
        return;
    }
    let (typed_ident, backspace) = match event {
        Event::Key(key) => (
            input::typed_char(key)
                .is_some_and(archipel_kit::editor::logic::lookup::is_identifier_char),
            key.code == KeyCode::Backspace,
        ),
        _ => (false, false),
    };
    if !st.lookup.open && !(text_changed && typed_ident) {
        return;
    }
    let keys = LookupKeys {
        typed_ident,
        backspace,
        ..LookupKeys::default()
    };
    feed(opts, editor, rt, st, keys, text_changed, out);
}

/// Keys while the menu is open. Returns whether the key was used.
pub(crate) fn key(
    opts: &Opts<'_>,
    editor: &mut archipel_kit::editor::model::Editor,
    rt: &Runtime,
    st: &mut EditorState,
    ev: &KeyEvent,
    out: &mut EditorOutput,
) -> bool {
    if st.menu.is_none() {
        return false;
    }
    let mods = ev.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    let sup = mods.contains(KeyModifiers::SUPER);
    let plain = !ctrl && !alt && !sup;
    let lookup = opts.completer.is_none_or(|c| c.filters_by_prefix());
    let mut keys = LookupKeys::default();
    match ev.code {
        KeyCode::Down if plain => keys.down = true,
        KeyCode::Up if plain => keys.up = true,
        KeyCode::PageDown if plain => keys.page_down = true,
        KeyCode::PageUp if plain => keys.page_up = true,
        KeyCode::Esc if plain => {
            st.close_menu();
            return true;
        }
        KeyCode::Enter if plain => keys.accept = true,
        KeyCode::Tab if plain => {
            keys.accept = true;
            keys.replace = true;
        }
        KeyCode::Char('.') if ctrl && !alt && lookup => {
            keys.accept = true;
            keys.commit = Some('.');
        }
        KeyCode::Char('q' | 'j') if ctrl && !alt && lookup => keys.toggle_doc = true,
        KeyCode::F(1) if plain && lookup => keys.toggle_doc = true,
        KeyCode::Char(' ') if ctrl && !alt => keys.explicit = true,
        _ => {
            // A context character chooses the item and is typed after it.
            match input::typed_char(ev) {
                Some(c) if lookup && COMMIT_CHARS.contains(&c) => {
                    keys.accept = true;
                    keys.commit = Some(c);
                }
                _ => return false,
            }
        }
    }
    feed(opts, editor, rt, st, keys, false, out);
    out.consumed = true;
    true
}

/// A left press at row `y` inside the menu: picks the row.
pub(crate) fn click(
    opts: &Opts<'_>,
    editor: &mut archipel_kit::editor::model::Editor,
    st: &mut EditorState,
    y: u16,
    out: &mut EditorOutput,
) {
    out.consumed = true;
    let Some(menu) = st.menu.as_ref() else {
        return;
    };
    let row = usize::from(y.saturating_sub(menu.rect.y)) + menu.frame.top;
    let Some(chosen) = menu.frame.chosen_row(row) else {
        return;
    };
    insert_chosen(editor, opts.language, &chosen);
    out.completed = Some(chosen.item);
    out.text_changed = true;
    st.close_menu();
    super::find::after_user_move(editor, st);
    super::run_scroll_to_caret(editor, st);
}

/// Turns the wheel over the menu: scrolls the rows.
pub(crate) fn wheel(
    opts: &Opts<'_>,
    editor: &mut archipel_kit::editor::model::Editor,
    rt: &Runtime,
    st: &mut EditorState,
    delta: i32,
    out: &mut EditorOutput,
) {
    let keys = LookupKeys {
        wheel: delta as f32,
        ..LookupKeys::default()
    };
    feed(opts, editor, rt, st, keys, false, out);
}

/// Draws the menu under the caret cell (`anchor_x`, `caret_y`), within `bounds`; above the
/// caret when there is no room below. The documentation of the selected item opens beside it
/// when it was asked for (Ctrl+Q).
pub(crate) fn draw(
    opts: &Opts<'_>,
    rt: &mut Runtime,
    st: &mut EditorState,
    anchor_x: u16,
    caret_y: u16,
    bounds: Rect,
    buf: &mut Buffer,
) {
    let Some(menu) = st.menu.as_mut() else {
        return;
    };
    let Some(found) = menu.frame.found.as_ref() else {
        return;
    };
    let look = rt.look.clone();
    let p = look.palette;
    let rows = menu.frame.rows();
    let count = rows.len();
    let visible = count.min(MAX_ROWS);
    if visible == 0 || bounds.width < 8 || bounds.height < 3 {
        return;
    }
    let top = menu.frame.top.min(count - visible);
    let label_w = rows
        .iter()
        .map(|r| text_width(&r.item_text) + text_width(&r.tail_text))
        .max()
        .unwrap_or(0);
    let type_w = rows
        .iter()
        .map(|r| text_width(&r.type_text))
        .max()
        .unwrap_or(0);
    let advert = menu.frame.advertisement.clone();
    let width = (3 + label_w + if type_w > 0 { 2 + type_w } else { 0 } + 1)
        .max(advert.as_deref().map_or(0, |a| text_width(a) + 2))
        .clamp(12, bounds.width.max(12))
        .min(bounds.width);
    let height = visible as u16 + u16::from(advert.is_some());
    let x = anchor_x
        .saturating_sub(2)
        .min(bounds.right().saturating_sub(width))
        .max(bounds.x);
    let below = bounds.bottom().saturating_sub(caret_y + 1);
    let y = if below >= height {
        caret_y + 1
    } else {
        caret_y.saturating_sub(height).max(bounds.y)
    };
    let rect = Rect::new(x, y, width, height.min(bounds.bottom().saturating_sub(y)));
    menu.rect = Rect::new(rect.x, rect.y, rect.width, visible as u16);
    for (n, index) in (top..count).take(visible).enumerate() {
        let row = &rows[index];
        let item = &found.items[menu.frame.ranked[index].0];
        let selected = index == menu.frame.selected;
        let row_y = rect.y + n as u16;
        if row_y >= rect.bottom() {
            break;
        }
        let bg = if selected { p.pmenu_bg } else { p.one_bg };
        let fg = if selected { p.on(p.pmenu_bg) } else { p.white };
        let base = Style::default().fg(fg).bg(bg);
        fill(buf, rect, rect.x, row_y, rect.width, base);
        let (symbol, icon_color) = match mark_of(item) {
            Mark::Symbol(kind) => (glyphs::symbol(kind), look.readable(kind.color())),
            Mark::Glyph(glyph) => (glyphs::nerd_font(glyph), p.light_grey),
        };
        put(
            buf,
            rect,
            rect.x + 1,
            row_y,
            &symbol.to_string(),
            Style::default()
                .fg(if selected { fg } else { icon_color })
                .bg(bg),
        );
        let mut label_style = base;
        if row.bold {
            label_style = label_style.add_modifier(Modifier::BOLD);
        }
        if row.strikeout {
            label_style = label_style.add_modifier(Modifier::CROSSED_OUT);
        }
        let mut cx = rect.x + 3;
        let room = rect.right().saturating_sub(1);
        for (i, c) in row.item_text.chars().enumerate() {
            if cx >= room {
                break;
            }
            let matched = row.fragments.iter().any(|f| f.contains(&i));
            let style = if matched {
                label_style
                    .fg(if selected { fg } else { p.blue })
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(if selected {
                        Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    })
            } else {
                label_style
            };
            cx = put(buf, rect, cx, row_y, &c.to_string(), style);
        }
        if !row.tail_text.is_empty() && cx < room {
            let tail = clip(&row.tail_text, room.saturating_sub(cx));
            cx = put(
                buf,
                rect,
                cx,
                row_y,
                &tail,
                Style::default()
                    .fg(if selected { fg } else { p.grey_fg })
                    .bg(bg),
            );
        }
        if !row.type_text.is_empty() {
            let detail = clip(
                &row.type_text,
                type_w.min(rect.width.saturating_sub(label_w + 6)),
            );
            let dx = rect.right().saturating_sub(text_width(&detail) + 1);
            if dx > cx {
                put(
                    buf,
                    rect,
                    dx,
                    row_y,
                    &detail,
                    Style::default()
                        .fg(if selected { fg } else { p.light_grey })
                        .bg(bg),
                );
            }
        }
    }
    // A scrollbar thumb when there are more items than rows.
    if count > visible && visible > 0 {
        let track = visible;
        let thumb = (track * visible / count).max(1);
        let at = top * (track - thumb) / (count - visible).max(1);
        for n in 0..track {
            let on = n >= at && n < at + thumb;
            let cell = (rect.right() - 1, rect.y + n as u16);
            if let Some(c) = buf.cell_mut(cell) {
                c.set_symbol(" ")
                    .set_bg(if on { p.one_bg3 } else { p.one_bg2 });
            }
        }
    }
    if let Some(advert) = &advert {
        let row_y = rect.y + visible as u16;
        if row_y < rect.bottom() {
            let style = Style::default()
                .fg(p.grey_fg)
                .bg(p.one_bg2)
                .add_modifier(Modifier::ITALIC);
            fill(buf, rect, rect.x, row_y, rect.width, style);
            put(
                buf,
                rect,
                rect.x + 1,
                row_y,
                &clip(advert, rect.width - 2),
                style,
            );
        }
    }
    let docs = menu.frame.doc.clone();
    let menu_rect = menu.rect;
    rt.clickable(opts.menu_id(), menu_rect);
    // The documentation of the selected item, beside the menu where there is room (the side
    // with more), else under or over it.
    if let Some(doc) = docs {
        let right_room = bounds.right().saturating_sub(menu_rect.right());
        let left_room = menu_rect.x.saturating_sub(bounds.x);
        let side = right_room.max(left_room);
        let beside = side >= 18;
        let room = if beside { side } else { bounds.width };
        let width = (DOC_WIDTH + 4).min(room);
        let lines = wrap_text(&doc, width.saturating_sub(4).max(1));
        let inner = lines.iter().map(|l| text_width(l)).max().unwrap_or(0);
        let doc_w = (inner + 4).min(width.max(5));
        let doc_h = (lines.len() as u16 + 2).min(bounds.height);
        let (x, y) = if beside && right_room >= left_room {
            (menu_rect.right(), menu_rect.y)
        } else if beside {
            (menu_rect.x.saturating_sub(doc_w), menu_rect.y)
        } else if menu_rect.bottom() + doc_h <= bounds.bottom() {
            (
                menu_rect.x,
                menu_rect.bottom() + u16::from(advert.is_some()),
            )
        } else {
            (menu_rect.x, menu_rect.y.saturating_sub(doc_h))
        };
        let x = x.min(bounds.right().saturating_sub(doc_w)).max(bounds.x);
        let y = y.min(bounds.bottom().saturating_sub(doc_h)).max(bounds.y);
        let r = Rect::new(x, y, doc_w, doc_h);
        if r.width >= 5 && r.height >= 3 {
            let bg = p.one_bg;
            crate::components::fill_rect(buf, r, Style::default().bg(bg).fg(p.light_grey));
            crate::tooltip::frame(buf, r, Style::default().fg(p.grey).bg(bg));
            for (n, line) in lines.iter().enumerate() {
                let ly = r.y + 1 + n as u16;
                if ly + 1 >= r.bottom() {
                    break;
                }
                put(
                    buf,
                    r,
                    r.x + 2,
                    ly,
                    line,
                    Style::default().fg(p.light_grey).bg(bg),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_the_shape_of_the_label() {
        let kind = |label: &str, insert: &str| default_kind(&Completion::new(label, insert));
        assert_eq!(kind("println", "println!()"), SymbolKind::Function);
        assert_eq!(kind("Vec", "Vec"), SymbolKind::Struct);
        assert_eq!(kind("MAX_ROWS", "MAX_ROWS"), SymbolKind::Constant);
        assert_eq!(kind("len", "len"), SymbolKind::Field);
        assert_eq!(kind("{{token}}", "{{token}}"), SymbolKind::Variable);
    }

    #[test]
    fn an_icon_of_the_item_wins_over_the_guess() {
        let item = Completion::new("run", "run").icon(Glyph::Run);
        assert!(matches!(mark_of(&item), Mark::Glyph(Glyph::Run)));
        let plain = Completion::new("run", "run");
        assert!(matches!(mark_of(&plain), Mark::Symbol(_)));
    }
}
