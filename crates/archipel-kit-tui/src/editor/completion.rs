//! The completion menu: the kit's [`Completer`] results in an nvim-cmp style popup under the
//! caret, with a kind icon, the matched characters highlighted, the detail in grey and the
//! selected item filled with the accent.
//!
//! Keys while it is open (the GUI's lookup keys): Up and Down move and wrap, PageUp and
//! PageDown jump, Enter inserts the item, Tab replaces (the identifier after the caret too),
//! Escape closes. Typing keeps it open and narrows it as long as the completer returns items.

use super::layout::Ctx;
use super::{EditorOutput, EditorState, Opts};
use crate::SymbolKind;
use crate::components::{clip, fill, put, text_width};
use crate::glyphs;
use crate::input::{KeyCode, KeyEvent, KeyModifiers};
use crate::runtime::Runtime;
use archipel_kit::editor::model::{Editor, EditorAction};
use archipel_kit::ui::{Completion, Completions};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// The most rows the menu shows at once.
const MAX_ROWS: usize = 8;

/// An open menu.
#[derive(Clone, Debug)]
pub(crate) struct Menu {
    pub items: Vec<Completion>,
    pub selected: usize,
    pub top: usize,
    /// The replaced range in characters.
    pub start: usize,
    pub end: usize,
    /// The byte offset where the replaced range starts (the menu is anchored there).
    pub anchor: usize,
    /// What the user typed of the item, for highlighting the matched characters.
    pub typed: String,
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

fn byte_of(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(b, _)| b)
}

fn caret_chars(editor: &Editor) -> usize {
    let offset = editor.carets.primary().offset;
    editor.document.text()[..offset].chars().count()
}

fn open_with(editor: &Editor, st: &mut EditorState, completions: Completions) {
    if completions.items.is_empty() {
        st.menu = None;
        return;
    }
    let text = editor.document.text();
    let caret = caret_chars(editor);
    let typed: String = text
        .chars()
        .skip(completions.start)
        .take(caret.saturating_sub(completions.start))
        .collect();
    let keep = st
        .menu
        .as_ref()
        .and_then(|m| m.items.get(m.selected))
        .map(|i| i.label.clone());
    let selected = keep
        .and_then(|label| completions.items.iter().position(|i| i.label == label))
        .unwrap_or(0);
    st.menu = Some(Menu {
        selected,
        top: 0,
        start: completions.start,
        end: completions.end,
        anchor: byte_of(text, completions.start),
        typed,
        items: completions.items,
        rect: Rect::default(),
    });
}

/// Ctrl+Space: asks the completer for everything that fits at the caret.
pub(crate) fn request(opts: &Opts<'_>, editor: &Editor, st: &mut EditorState) {
    let Some(completer) = opts.completer else {
        return;
    };
    match completer.complete_on_request(editor.document.text(), caret_chars(editor)) {
        Some(c) => open_with(editor, st, c),
        None => st.menu = None,
    }
}

/// After an action ran: typing and deleting offer or narrow the list, anything else that
/// moved the caret closes it.
pub(crate) fn after(opts: &Opts<'_>, editor: &Editor, st: &mut EditorState, action: &EditorAction) {
    let Some(completer) = opts.completer else {
        return;
    };
    let single = editor.carets.caret_count() == 1 && !editor.carets.has_selection();
    match action {
        EditorAction::Type(_) | EditorAction::Backspace if single => {
            match completer.complete(editor.document.text(), caret_chars(editor)) {
                Some(c) => open_with(editor, st, c),
                None => st.menu = None,
            }
        }
        EditorAction::Delete if single && st.menu.is_some() => {
            match completer.complete_on_request(editor.document.text(), caret_chars(editor)) {
                Some(c) => open_with(editor, st, c),
                None => st.menu = None,
            }
        }
        _ => st.menu = None,
    }
}

/// Replaces `start..end` (characters) with the selected item as one undo step and puts the
/// caret where the item asks; with `replace` the identifier after the caret goes too.
pub(crate) fn accept(editor: &mut Editor, st: &mut EditorState, replace: bool) -> bool {
    let Some(menu) = st.menu.take() else {
        return false;
    };
    let Some(item) = menu.items.get(menu.selected) else {
        return false;
    };
    let text = editor.document.text();
    let mut end = menu.end.max(menu.start);
    if replace {
        let chars: Vec<char> = text.chars().collect();
        while chars
            .get(end)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            end += 1;
        }
    }
    let (from, to) = (byte_of(text, menu.start), byte_of(text, end));
    let inserted = item.insert.chars().count();
    let after_chars = item.caret.unwrap_or(inserted).min(inserted);
    let after_byte: usize = item
        .insert
        .chars()
        .take(after_chars)
        .map(char::len_utf8)
        .sum();
    let insert = item.insert.clone();
    editor.replace_range(from, to, &insert);
    editor.carets.set_caret(&editor.document, from + after_byte);
    true
}

/// Keys while the menu is open. Returns whether the key was used.
pub(crate) fn key(
    _opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    ev: &KeyEvent,
    out: &mut EditorOutput,
) -> bool {
    let Some(menu) = st.menu.as_mut() else {
        return false;
    };
    let plain = !ev
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);
    if !plain {
        return false;
    }
    let count = menu.items.len();
    let page = MAX_ROWS.min(count).saturating_sub(1).max(1);
    match ev.code {
        KeyCode::Down => menu.selected = (menu.selected + 1) % count,
        KeyCode::Up => menu.selected = (menu.selected + count - 1) % count,
        KeyCode::PageDown => menu.selected = (menu.selected + page).min(count - 1),
        KeyCode::PageUp => menu.selected = menu.selected.saturating_sub(page),
        KeyCode::Esc => st.menu = None,
        KeyCode::Enter | KeyCode::Tab | KeyCode::BackTab if ev.code != KeyCode::BackTab => {
            let replace = ev.code == KeyCode::Tab;
            if accept(editor, st, replace) {
                out.text_changed = true;
                super::find::after_user_move(editor, st);
                let at = editor.carets.primary().offset;
                let _ = at;
                super::run_scroll_to_caret(editor, st);
            }
        }
        _ => return false,
    }
    let _ = rt;
    true
}

/// A left press at (`x`, `y`) inside the menu: picks the row.
pub(crate) fn click(editor: &mut Editor, st: &mut EditorState, y: u16, out: &mut EditorOutput) {
    let Some(menu) = st.menu.as_mut() else {
        return;
    };
    let row = y.saturating_sub(menu.rect.y) as usize + menu.top;
    if row < menu.items.len() {
        menu.selected = row;
        if accept(editor, st, false) {
            out.text_changed = true;
            super::find::after_user_move(editor, st);
            super::run_scroll_to_caret(editor, st);
        }
    }
    out.consumed = true;
}

/// Turns the wheel over the menu: moves the selection.
pub(crate) fn wheel(st: &mut EditorState, delta: i32) {
    if let Some(menu) = st.menu.as_mut() {
        let last = menu.items.len() as i32 - 1;
        menu.selected = (menu.selected as i32 + delta).clamp(0, last) as usize;
    }
}

/// The matched characters of `label` for what was `typed`: the typed characters in order,
/// ignoring case (the same positions an nvim-cmp fuzzy match marks).
fn matched(label: &str, typed: &str) -> Vec<usize> {
    let mut want = typed.chars().flat_map(char::to_lowercase).peekable();
    let mut out = Vec::new();
    for (i, c) in label.chars().enumerate() {
        let Some(w) = want.peek() else {
            break;
        };
        if c.to_lowercase().next() == Some(*w) {
            out.push(i);
            want.next();
        }
    }
    if want.peek().is_some() {
        out.clear();
    }
    out
}

/// Draws the menu under the caret cell (`x`, `y`) of the text at `anchor_x`, within `bounds`;
/// above the caret when there is no room below.
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
    let look = rt.look.clone();
    let p = look.palette;
    let count = menu.items.len();
    let rows = count.min(MAX_ROWS);
    menu.top = menu
        .top
        .min(count - rows)
        .max((menu.selected + 1).saturating_sub(rows))
        .min(menu.selected);
    let kinds: Vec<SymbolKind> = menu.items.iter().map(default_kind).collect();
    let label_w = menu
        .items
        .iter()
        .map(|i| text_width(&i.label))
        .max()
        .unwrap_or(0);
    let detail_w = menu
        .items
        .iter()
        .map(|i| text_width(&i.detail))
        .max()
        .unwrap_or(0);
    let width = (3 + label_w + if detail_w > 0 { 2 + detail_w } else { 0 } + 1)
        .clamp(12, bounds.width.max(12))
        .min(bounds.width);
    if rows == 0 || bounds.width < 8 || bounds.height < 3 {
        return;
    }
    let height = rows as u16;
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
    menu.rect = rect;
    for (n, index) in (menu.top..count).take(rect.height as usize).enumerate() {
        let item = &menu.items[index];
        let selected = index == menu.selected;
        let row_y = rect.y + n as u16;
        let bg = if selected { p.pmenu_bg } else { p.one_bg };
        let fg = if selected { p.on(p.pmenu_bg) } else { p.white };
        let base = Style::default().fg(fg).bg(bg);
        fill(buf, rect, rect.x, row_y, rect.width, base);
        let kind = kinds[index];
        let icon_color = if selected {
            fg
        } else {
            look.readable(kind.color())
        };
        put(
            buf,
            rect,
            rect.x + 1,
            row_y,
            &glyphs::symbol(kind).to_string(),
            Style::default().fg(icon_color).bg(bg),
        );
        let label_x = rect.x + 3;
        let marks = matched(&item.label, &menu.typed);
        let mut cx = label_x;
        let room = rect.right().saturating_sub(1);
        for (i, c) in item.label.chars().enumerate() {
            if cx >= room {
                break;
            }
            let style = if marks.contains(&i) {
                Style::default()
                    .fg(if selected { fg } else { p.blue })
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(if selected {
                        Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    })
            } else {
                base
            };
            cx = put(buf, rect, cx, row_y, &c.to_string(), style);
        }
        if !item.detail.is_empty() {
            let detail = clip(
                &item.detail,
                detail_w.min(rect.width.saturating_sub(label_w + 6)),
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
    if count > rows && rect.height > 0 {
        let track = rect.height as usize;
        let thumb = (track * rows / count).max(1);
        let at = menu.top * (track - thumb) / (count - rows).max(1);
        for n in 0..track {
            let on = n >= at && n < at + thumb;
            let cell = (rect.right() - 1, rect.y + n as u16);
            if let Some(c) = buf.cell_mut(cell) {
                c.set_symbol(" ")
                    .set_bg(if on { p.one_bg3 } else { p.one_bg2 });
            }
        }
    }
    rt.clickable(opts.menu_id(), rect);
    let _ = Ctx::new;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_typed_characters_are_matched_in_order_ignoring_case() {
        assert_eq!(matched("println", "prl"), [0, 1, 5]);
        assert_eq!(matched("Vec", "ve"), [0, 1]);
        assert!(matched("abc", "xyz").is_empty());
    }

    #[test]
    fn kinds_follow_the_shape_of_the_label() {
        let kind = |l: &str, i: &str| default_kind(&Completion::new(l, i));
        assert_eq!(kind("push()", "push("), SymbolKind::Function);
        assert_eq!(kind("String", "String"), SymbolKind::Struct);
        assert_eq!(kind("MAX_LEN", "MAX_LEN"), SymbolKind::Constant);
        assert_eq!(kind("len", "len"), SymbolKind::Field);
        assert_eq!(kind("{{token}}", "{{token}}"), SymbolKind::Variable);
    }
}
