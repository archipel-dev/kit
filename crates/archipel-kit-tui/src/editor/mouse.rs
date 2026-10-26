//! The mouse, a port of the GUI editor's rules to cells: a click places the caret, a double
//! click selects the word, a triple click the line, Shift+click extends (by the unit of the
//! last multi-click), a drag selects and autoscrolls beyond the edges, Alt+drag and the
//! middle button select a column, Alt+Shift+click adds or removes a caret, Ctrl+click asks for
//! navigation, a click on a collapsed fold expands it, the gutter selects lines and folds,
//! the wheel scrolls.

use super::layout::{Ctx, RowMap};
use super::{
    EditorOutput, EditorState, Geo, GutterClick, Opts, apply_scroll, find, geometry, row_col,
};
use crate::hit::Id;
use crate::input::{KeyModifiers, MouseButton};
use crate::pointer::{Pointer, PointerKind};
use crate::runtime::Runtime;
use archipel_kit::editor::model::{
    ActionContext, ClipboardContent, Editor, EditorAction, LogicalPosition, Os, ScrollRequest,
    ScrollType, column_of,
};
use archipel_kit::editor::{GutterArea, NavigationKind, NavigationRequest};
use archipel_kit::tokens::{editor as metrics, motion};
use std::rc::Rc;

type Cell = (u16, u16);

/// The last unit of a multi-click, for Shift+click and drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unit {
    Word(usize, usize),
    Line(usize, usize),
}

/// A mouse gesture in progress.
#[derive(Clone, Debug)]
pub(crate) enum Drag {
    /// Extending the primary selection from `anchor` by characters; `multi` keeps the other
    /// carets (the gesture began with Alt+Shift+click).
    Char { anchor: usize, multi: bool },
    /// Extending by whole words around the original word `start..end`.
    Word { start: usize, end: usize },
    /// Extending by whole lines around the original line `start..end`.
    Line { start: usize, end: usize },
    /// A rectangular selection (Alt+drag, middle-drag); a middle press that does not move is
    /// a middle click.
    Column { middle: bool, press: Cell },
    /// Dragging in the line number column: selects lines once the pointer moves.
    GutterLines {
        anchor_line: usize,
        started: bool,
        press: Cell,
    },
    /// Dragging from another part of the gutter: selects from the start of the line.
    GutterChar {
        anchor: usize,
        started: bool,
        press: Cell,
    },
    /// A press on a gutter icon: a click when released on the same cell.
    GutterIcon {
        line: usize,
        lane: usize,
        press: Cell,
    },
    /// A press inside the selection: it collapses on release without a drag; a drag starts a
    /// new selection at the press.
    InSelection { offset: usize, press: Cell },
    /// A press on a collapsed fold placeholder: it expands on release without a drag.
    OnFold { region_start: usize, press: Cell },
    /// Dragging in the scrollbar column.
    Stripe,
}

/// Where a cell is in the text.
pub(crate) struct Located {
    pub visual: usize,
    /// The cell column counted from the start of the visual line.
    pub col: usize,
    pub offset: usize,
    pub virtual_cols: usize,
    pub logical: usize,
}

pub(crate) struct View {
    pub geo: Geo,
    pub map: Rc<RowMap>,
}

fn view(opts: &Opts<'_>, editor: &Editor, st: &mut EditorState) -> View {
    let geo = geometry(opts, editor, st, st.area);
    let map = st.rows(editor, geo.wrap_cols);
    View { geo, map }
}

/// The text position under cell (`x`, `y`), clamped to the document.
pub(crate) fn locate(editor: &Editor, v: &View, st: &EditorState, x: u16, y: u16) -> Located {
    let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
    let total = v.map.total_rows();
    let row = (st.first_row as isize + (y as isize - v.geo.text.y as isize))
        .clamp(0, total as isize - 1) as usize;
    let (visual, k) = v.map.visual_at_row(row);
    let layout = ctx.layout(visual);
    let base = match v.map.wrap() {
        Some(wc) => k * wc,
        None => st.first_col,
    };
    let rel = (x as isize - v.geo.text.x as isize).max(0) as usize;
    let mut col = base + rel;
    if let Some(wc) = v.map.wrap() {
        col = col.min(base + wc);
    }
    let (offset, virtual_cols) = layout.offset_at(ctx.doc, ctx.tab, col);
    Located {
        visual,
        col,
        offset,
        virtual_cols,
        logical: layout.logical,
    }
}

use archipel_kit::editor::logic::gestures::{
    inside_selection, line_range, select_lines, set_selection, word_or_line,
};

/// The model's logical position of a located cell (columns past the line end are virtual).
fn logical_position(editor: &Editor, at: &Located) -> LogicalPosition {
    let col = column_of(&editor.document, at.offset, editor.settings.tab_size) + at.virtual_cols;
    LogicalPosition::new(at.logical, col)
}

pub(crate) use archipel_kit::editor::logic::gestures::{autoscroll_tick, region_at_line};

fn nav_modifier(os: Os, mods: KeyModifiers) -> bool {
    match os {
        Os::MacOs => mods.contains(KeyModifiers::SUPER),
        Os::WindowsLinux => mods.contains(KeyModifiers::CONTROL),
    }
}

/// A mouse event for the editor (the runtime resolved the pointer).
pub(crate) fn mouse(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    out: &mut EditorOutput,
) {
    let Some(p) = rt.pointer().copied() else {
        return;
    };
    match p.kind {
        PointerKind::Down(button) => press(opts, editor, rt, st, &p, button, out),
        PointerKind::Up(button) => release(opts, editor, st, &p, button, out),
        PointerKind::Drag(_) if st.drag.is_some() => {
            drag(opts, editor, rt, st, &p, out);
            out.consumed = true;
        }
        PointerKind::ScrollUp
        | PointerKind::ScrollDown
        | PointerKind::ScrollLeft
        | PointerKind::ScrollRight => wheel(opts, editor, rt, st, &p, out),
        _ => {}
    }
    find::after_user_move(editor, st);
}

fn ours(opts: &Opts<'_>, id: Option<Id>) -> bool {
    id.is_some_and(|id| {
        id == opts.id
            || id == opts.gutter_id()
            || id == opts.stripe_id()
            || id == opts.bar_id()
            || id == opts.menu_id()
    })
}

fn press(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    p: &Pointer,
    button: MouseButton,
    out: &mut EditorOutput,
) {
    if !ours(opts, p.target) {
        // A press elsewhere closes the completion menu.
        st.close_menu();
        return;
    }
    let target = p.target;
    if target == Some(opts.menu_id()) {
        super::completion::click(opts, editor, st, p.y, out);
        return;
    }
    st.close_menu();
    rt.focus.focus(opts.id);
    out.consumed = true;
    if target == Some(opts.bar_id()) {
        if button == MouseButton::Left {
            find::click(opts, editor, rt, st, p.x, p.y, out);
        }
        return;
    }
    st.find.focus = find::FindFocus::Editor;
    st.find.history_open = false;
    st.autoscroll_last = rt.now;
    st.autoscroll_counter = 0;
    let v = view(opts, editor, st);
    if target == Some(opts.stripe_id()) {
        if button == MouseButton::Left {
            stripe_press(opts, editor, st, &v.geo, p.y, out);
            st.drag = Some(Drag::Stripe);
        }
        return;
    }
    if target == Some(opts.gutter_id()) {
        gutter_press(opts, editor, st, &v, p, button, out);
        return;
    }
    let at = locate(editor, &v, st, p.x, p.y);
    match button {
        MouseButton::Right => {
            if !inside_selection(editor, at.offset) {
                editor.carets.set_caret(&editor.document, at.offset);
                out.carets_moved = true;
            }
            out.context_menu = Some((p.x, p.y));
        }
        MouseButton::Middle => {
            let position = logical_position(editor, &at);
            let tab = editor.settings.tab_size;
            if p.mods.contains(KeyModifiers::SHIFT) && p.mods.contains(KeyModifiers::ALT) {
                editor
                    .carets
                    .extend_block_selection(&editor.document, tab, position);
            } else {
                editor
                    .carets
                    .start_block_selection(&editor.document, tab, position);
            }
            st.drag = Some(Drag::Column {
                middle: true,
                press: (p.x, p.y),
            });
            out.carets_moved = true;
        }
        MouseButton::Left => primary_press(opts, editor, st, p, &at, out),
    }
}

#[allow(clippy::too_many_arguments)]
fn primary_press(
    opts: &Opts<'_>,
    editor: &mut Editor,
    st: &mut EditorState,
    p: &Pointer,
    at: &Located,
    out: &mut EditorOutput,
) {
    let (alt, shift) = (
        p.mods.contains(KeyModifiers::ALT),
        p.mods.contains(KeyModifiers::SHIFT),
    );
    let nav = nav_modifier(opts.os, p.mods);
    let press = (p.x, p.y);
    let tab = editor.settings.tab_size;
    if p.clicks <= 1 {
        st.unit = None;
    }
    let layout = Ctx::new(&editor.document, &editor.folding, tab).layout(at.visual);
    // A click on a collapsed fold placeholder expands it on release.
    if p.clicks == 1
        && !shift
        && !alt
        && let Some(seg) = layout.fold_at(at.col)
    {
        st.drag = Some(Drag::OnFold {
            region_start: seg.off0,
            press,
        });
        return;
    }
    if alt && shift {
        // Alt+Shift+click: add a caret, or remove the one that is there.
        st.unit = None;
        if let Some(index) = editor
            .carets
            .carets()
            .iter()
            .position(|c| c.offset == at.offset && !c.has_selection())
        {
            editor.carets.remove_caret(index);
            out.carets_moved = true;
            return;
        }
        if editor.carets.add_caret(&editor.document, at.offset) {
            st.drag = Some(Drag::Char {
                anchor: at.offset,
                multi: true,
            });
        }
        out.carets_moved = true;
        return;
    }
    if alt && !shift && !nav {
        // Alt+click and drag: column selection.
        let position = logical_position(editor, at);
        editor
            .carets
            .start_block_selection(&editor.document, tab, position);
        st.drag = Some(Drag::Column {
            middle: false,
            press,
        });
        st.unit = None;
        out.carets_moved = true;
        return;
    }
    if nav && !shift {
        out.navigation = Some(NavigationRequest {
            offset: at.offset,
            kind: if alt {
                NavigationKind::Implementation
            } else {
                NavigationKind::Declaration
            },
        });
        editor.carets.set_caret(&editor.document, at.offset);
        st.drag = None;
        out.carets_moved = true;
        return;
    }
    if shift {
        // Extend from the anchor, by the unit of the last multi-click.
        let anchor = editor.carets.primary().anchor;
        let lead = at.offset;
        match st.unit {
            Some(Unit::Word(s, e)) => {
                let (ws, we) = word_or_line(editor, lead);
                if lead < s {
                    set_selection(editor, e, ws.min(lead), false);
                } else {
                    set_selection(editor, s, we.max(lead), false);
                }
            }
            Some(Unit::Line(s, e)) => {
                let (ls, le) = line_range(editor, lead);
                if ls < s {
                    set_selection(editor, e, ls, false);
                } else {
                    set_selection(editor, s, le, false);
                }
            }
            None => set_selection(editor, anchor, lead, false),
        }
        st.drag = Some(match st.unit {
            Some(Unit::Word(s, e)) => Drag::Word { start: s, end: e },
            Some(Unit::Line(s, e)) => Drag::Line { start: s, end: e },
            None => Drag::Char {
                anchor,
                multi: false,
            },
        });
        out.carets_moved = true;
        return;
    }
    match p.clicks {
        0 | 1 => {
            if inside_selection(editor, at.offset) {
                st.drag = Some(Drag::InSelection {
                    offset: at.offset,
                    press,
                });
            } else {
                editor.carets.set_caret(&editor.document, at.offset);
                st.drag = Some(Drag::Char {
                    anchor: at.offset,
                    multi: false,
                });
                st.unit = None;
                out.carets_moved = true;
            }
        }
        2 => {
            let (s, e) = word_or_line(editor, at.offset);
            set_selection(editor, s, e, false);
            st.unit = Some(Unit::Word(s, e));
            st.drag = Some(Drag::Word { start: s, end: e });
            out.carets_moved = true;
        }
        _ => {
            let (s, e) = line_range(editor, at.offset);
            set_selection(editor, s, e, false);
            st.unit = Some(Unit::Line(s, e));
            st.drag = Some(Drag::Line { start: s, end: e });
            out.carets_moved = true;
        }
    }
}

fn gutter_press(
    opts: &Opts<'_>,
    editor: &mut Editor,
    st: &mut EditorState,
    v: &View,
    p: &Pointer,
    button: MouseButton,
    out: &mut EditorOutput,
) {
    let at = locate(editor, v, st, p.x, p.y);
    let line = at.logical;
    let area = v.geo.gutter_area(p.x);
    let press = (p.x, p.y);
    // Icons are buttons of their own; so are the fold markers.
    if let GutterArea::Icon(lane) = area
        && opts.icons.iter().any(|i| i.line == line && i.lane == lane)
    {
        if button == MouseButton::Left {
            st.drag = Some(Drag::GutterIcon { line, lane, press });
        } else {
            out.gutter_clicks.push(GutterClick {
                line,
                area,
                button,
                modifiers: p.mods,
            });
        }
        return;
    }
    if v.geo.in_fold_column(p.x)
        && button == MouseButton::Left
        && let Some(index) = region_at_line(editor, line)
    {
        editor.toggle_fold_region(index, p.mods.contains(KeyModifiers::ALT));
        out.carets_moved = true;
        return;
    }
    if button != MouseButton::Left {
        out.gutter_clicks.push(GutterClick {
            line,
            area,
            button,
            modifiers: p.mods,
        });
        return;
    }
    let doc = &editor.document;
    let line_start = doc.line_start_offset(line);
    let line_break_end = doc.line_end_with_break(line);
    if p.mods.contains(KeyModifiers::SHIFT) && area == GutterArea::LineNumber {
        // Extend the selection by whole lines.
        let anchor_line = doc.line_number(editor.carets.primary().anchor);
        let (anchor, lead) = if line >= anchor_line {
            (doc.line_start_offset(anchor_line), line_break_end)
        } else {
            (doc.line_end_with_break(anchor_line), line_start)
        };
        set_selection(editor, anchor, lead, false);
        st.drag = Some(Drag::GutterLines {
            anchor_line,
            started: true,
            press,
        });
        out.carets_moved = true;
        return;
    }
    if area == GutterArea::LineNumber {
        if opts.gutter_click_selects_line {
            set_selection(editor, line_start, line_break_end, false);
            st.drag = Some(Drag::GutterLines {
                anchor_line: line,
                started: true,
                press,
            });
            out.carets_moved = true;
        } else {
            st.drag = Some(Drag::GutterLines {
                anchor_line: line,
                started: false,
                press,
            });
        }
    } else {
        st.drag = Some(Drag::GutterChar {
            anchor: line_start,
            started: false,
            press,
        });
    }
}

fn release(
    opts: &Opts<'_>,
    editor: &mut Editor,
    st: &mut EditorState,
    p: &Pointer,
    button: MouseButton,
    out: &mut EditorOutput,
) {
    st.autoscroll_at = None;
    let Some(state) = st.drag.take() else {
        return;
    };
    out.consumed = true;
    let pos = (p.x, p.y);
    let v = view(opts, editor, st);
    match state {
        Drag::InSelection { offset, press } => {
            if pos == press {
                editor.carets.set_caret(&editor.document, offset);
                out.carets_moved = true;
            }
        }
        Drag::OnFold {
            region_start,
            press,
        } => {
            if pos == press
                && let Some(index) = editor
                    .folding
                    .regions()
                    .iter()
                    .position(|r| r.start == region_start && !r.expanded)
            {
                editor.toggle_fold_region(index, false);
                out.carets_moved = true;
            }
        }
        Drag::GutterLines {
            anchor_line,
            started: false,
            press,
        } => {
            if pos == press {
                out.gutter_clicks.push(GutterClick {
                    line: anchor_line.min(editor.document.line_count() - 1),
                    area: GutterArea::LineNumber,
                    button,
                    modifiers: p.mods,
                });
            }
        }
        Drag::GutterChar {
            started: false,
            press,
            ..
        } => {
            if pos == press {
                let at = locate(editor, &v, st, press.0, press.1);
                out.gutter_clicks.push(GutterClick {
                    line: at.logical,
                    area: v.geo.gutter_area(press.0),
                    button,
                    modifiers: p.mods,
                });
            }
        }
        Drag::GutterIcon { line, lane, press } => {
            if pos == press {
                out.gutter_clicks.push(GutterClick {
                    line,
                    area: GutterArea::Icon(lane),
                    button,
                    modifiers: p.mods,
                });
            }
        }
        Drag::Column {
            middle: true,
            press,
        } if pos == press => {
            // A middle click: Go to Declaration on macOS, paste the primary selection on Linux.
            let at = locate(editor, &v, st, press.0, press.1);
            match opts.os {
                Os::MacOs => {
                    out.navigation = Some(NavigationRequest {
                        offset: at.offset,
                        kind: NavigationKind::Declaration,
                    });
                    editor.carets.set_caret(&editor.document, at.offset);
                }
                Os::WindowsLinux => {
                    editor.carets.set_caret(&editor.document, at.offset);
                    if !opts.viewer && !st.primary.is_empty() {
                        let content = ClipboardContent::text(st.primary.clone());
                        let ctx = ActionContext::new(opts.language).with_clipboard(&content);
                        let outcome = editor.execute(&EditorAction::PasteSimple, &ctx);
                        out.text_changed |= outcome.text_changed;
                    }
                }
            }
            out.carets_moved = true;
        }
        Drag::Char { .. } | Drag::Word { .. } | Drag::Line { .. } | Drag::Column { .. } => {
            // The selection made by the mouse is the primary selection on Linux.
            let texts = editor.carets.selected_texts(&editor.document);
            if !texts.is_empty() {
                st.primary = texts.join("\n");
            }
        }
        _ => {}
    }
    st.autoscroll_counter = 0;
}

fn drag(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    p: &Pointer,
    out: &mut EditorOutput,
) {
    let Some(state) = st.drag.clone() else {
        return;
    };
    let v = view(opts, editor, st);
    let pos = (p.x, p.y);
    if let Drag::Stripe = state {
        if let Some(stripe) = v.geo.stripe {
            let _ = stripe;
            stripe_scroll(opts, editor, st, &v.geo, p.y);
        }
        return;
    }
    let tab = editor.settings.tab_size;
    let at = locate(editor, &v, st, p.x, p.y);
    let moved_from = |press: Cell| pos != press;
    match state {
        Drag::Char { anchor, multi } => {
            set_selection(editor, anchor, at.offset, multi);
            out.carets_moved = true;
        }
        Drag::Word { start, end } => {
            let (ws, we) = word_or_line(editor, at.offset);
            if at.offset < start {
                set_selection(editor, end, ws.min(at.offset), false);
            } else {
                set_selection(editor, start, we.max(at.offset).max(end), false);
            }
            out.carets_moved = true;
        }
        Drag::Line { start, end } => {
            let (ls, le) = line_range(editor, at.offset);
            if ls < start {
                set_selection(editor, end, ls, false);
            } else {
                set_selection(editor, start, le, false);
            }
            out.carets_moved = true;
        }
        Drag::Column { .. } => {
            let position = logical_position(editor, &at);
            editor
                .carets
                .extend_block_selection(&editor.document, tab, position);
            out.carets_moved = true;
        }
        Drag::GutterLines {
            anchor_line,
            started,
            press,
        } => {
            if !started && !moved_from(press) {
                return;
            }
            st.drag = Some(Drag::GutterLines {
                anchor_line,
                started: true,
                press,
            });
            select_lines(editor, anchor_line, at.logical);
            out.carets_moved = true;
        }
        Drag::GutterChar {
            anchor,
            started,
            press,
        } => {
            if !started && !moved_from(press) {
                return;
            }
            st.drag = Some(Drag::GutterChar {
                anchor,
                started: true,
                press,
            });
            set_selection(editor, anchor, at.offset, false);
            out.carets_moved = true;
        }
        Drag::InSelection { offset, press } => {
            if moved_from(press) {
                // No drag and drop: a drag from inside the selection selects anew.
                st.drag = Some(Drag::Char {
                    anchor: offset,
                    multi: false,
                });
                set_selection(editor, offset, at.offset, false);
                out.carets_moved = true;
            }
        }
        Drag::OnFold {
            region_start,
            press,
        } => {
            if moved_from(press) {
                st.drag = Some(Drag::Char {
                    anchor: region_start,
                    multi: false,
                });
                set_selection(editor, region_start, at.offset, false);
                out.carets_moved = true;
            }
        }
        Drag::GutterIcon { .. } | Drag::Stripe => {}
    }
    if out.carets_moved {
        apply_scroll(
            editor,
            st,
            &v.geo,
            ScrollRequest::ToCaret {
                scroll_type: ScrollType::Relative,
                animate: false,
            },
        );
    }
    // Beyond the edge the autoscroll timer runs.
    let text = v.geo.text;
    let beyond = p.y < text.y || p.y >= text.bottom() || p.x < text.x || p.x >= text.right();
    st.autoscroll_at = beyond.then_some(pos);
    if beyond {
        autoscroll(opts, editor, st, pos, rt.now);
    } else {
        st.autoscroll_counter = 0;
        st.autoscroll_last = rt.now;
    }
}

/// Selects whole lines from `anchor_line` to `line`.
/// Moves the selection head while the pointer is beyond the edge of the text (spec 2.3): a
/// timer ticks; every `floor(20 / d) + 1` ticks the head moves `1 + floor(d / 20)` lines (or
/// columns), where `d` is the distance beyond the edge in rows scaled to the GUI's pixels.
fn autoscroll(opts: &Opts<'_>, editor: &mut Editor, st: &mut EditorState, pos: Cell, now: f64) {
    let v = view(opts, editor, st);
    let text = v.geo.text;
    let dy = if pos.1 < text.y {
        pos.1 as f32 - text.y as f32
    } else if pos.1 >= text.bottom() {
        pos.1 as f32 - (text.bottom() as f32 - 1.0)
    } else {
        0.0
    };
    let dx = if pos.0 < text.x {
        pos.0 as f32 - text.x as f32
    } else if pos.0 >= text.right() {
        pos.0 as f32 - (text.right() as f32 - 1.0)
    } else {
        0.0
    };
    if dx == 0.0 && dy == 0.0 {
        st.autoscroll_counter = 0;
        st.autoscroll_last = now;
        return;
    }
    let tick = f64::from(motion::EDITOR_AUTOSCROLL_TICK);
    let ticks = (((now - st.autoscroll_last) / tick).floor().max(0.0) as u32).min(8);
    if ticks == 0 {
        return;
    }
    st.autoscroll_last += f64::from(ticks) * tick;
    let (mut steps_y, mut steps_x) = (0isize, 0isize);
    for _ in 0..ticks {
        // A cell is a row: 20 of the GUI's pixels.
        let d = dy.abs().max(dx.abs()) * metrics::AUTOSCROLL_DISTANCE;
        if let Some(n) =
            autoscroll_tick(&mut st.autoscroll_counter, d, metrics::AUTOSCROLL_DISTANCE)
        {
            let n = n as isize;
            if dy != 0.0 {
                steps_y += n * dy.signum() as isize;
            }
            if dx != 0.0 {
                steps_x += n * dx.signum() as isize;
            }
        }
    }
    if steps_y == 0 && steps_x == 0 {
        return;
    }
    // The head moves from where it is.
    let (offset, virtual_cols, new_logical) = {
        let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
        let head = editor.carets.primary().offset;
        let head_visual = ctx.visual_of(head);
        let head_col = ctx.layout(head_visual).col_of(ctx.doc, ctx.tab, head);
        let new_visual =
            (head_visual as isize + steps_y).clamp(0, v.map.visual_lines() as isize - 1) as usize;
        let new_layout = ctx.layout(new_visual);
        let col = (head_col as isize + steps_x).max(0) as usize;
        let (offset, virtual_cols) = new_layout.offset_at(ctx.doc, ctx.tab, col);
        (offset, virtual_cols, new_layout.logical)
    };
    let tab = editor.settings.tab_size;
    match st.drag.clone() {
        Some(Drag::Char { anchor, multi }) => set_selection(editor, anchor, offset, multi),
        Some(Drag::Word { start, end }) => {
            let (ws, we) = word_or_line(editor, offset);
            if offset < start {
                set_selection(editor, end, ws.min(offset), false);
            } else {
                set_selection(editor, start, we.max(offset).max(end), false);
            }
        }
        Some(Drag::Line { start, end }) => {
            let (ls, le) = line_range(editor, offset);
            if ls < start {
                set_selection(editor, end, ls, false);
            } else {
                set_selection(editor, start, le, false);
            }
        }
        Some(Drag::Column { .. }) => {
            let position = LogicalPosition::new(
                new_logical,
                column_of(&editor.document, offset, tab) + virtual_cols,
            );
            editor
                .carets
                .extend_block_selection(&editor.document, tab, position);
        }
        Some(Drag::GutterLines { anchor_line, .. }) => {
            select_lines(editor, anchor_line, new_logical)
        }
        Some(Drag::GutterChar { anchor, .. }) => set_selection(editor, anchor, offset, false),
        _ => return,
    }
    apply_scroll(
        editor,
        st,
        &v.geo,
        ScrollRequest::ToCaret {
            scroll_type: ScrollType::Relative,
            animate: false,
        },
    );
}

/// The autoscroll timer: runs while the pointer rests beyond the edge of the text.
pub(crate) fn tick(opts: &Opts<'_>, editor: &mut Editor, rt: &mut Runtime, st: &mut EditorState) {
    if let (Some(pos), Some(_)) = (st.autoscroll_at, &st.drag) {
        autoscroll(opts, editor, st, pos, rt.now);
    } else {
        st.autoscroll_at = None;
    }
}

fn wheel(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &Runtime,
    st: &mut EditorState,
    p: &Pointer,
    out: &mut EditorOutput,
) {
    if !ours(opts, p.target) {
        return;
    }
    if p.target == Some(opts.menu_id()) {
        super::completion::wheel(
            opts,
            editor,
            rt,
            st,
            if p.kind == PointerKind::ScrollUp {
                -1
            } else {
                1
            },
            out,
        );
        out.consumed = true;
        return;
    }
    let v = view(opts, editor, st);
    out.consumed = true;
    let (dy, dx) = match p.kind {
        PointerKind::ScrollUp if p.mods.contains(KeyModifiers::SHIFT) => (0, -4),
        PointerKind::ScrollDown if p.mods.contains(KeyModifiers::SHIFT) => (0, 4),
        PointerKind::ScrollUp => (-3, 0),
        PointerKind::ScrollDown => (3, 0),
        PointerKind::ScrollLeft => (0, -4),
        _ => (0, 4),
    };
    if dy != 0 {
        apply_scroll(editor, st, &v.geo, ScrollRequest::Lines(dy));
    }
    if dx != 0 && v.geo.wrap_cols.is_none() {
        let content = archipel_kit::editor::model::ContentSize {
            lines: v.map.total_rows(),
            columns: st_max_cols(editor, st),
        };
        let max = archipel_kit::editor::model::max_first_column(
            content,
            v.geo.width() as f64,
            &super::scroll_settings(editor, st),
        );
        st.first_col = (st.first_col as f64 + dx as f64).clamp(0.0, max).round() as usize;
    }
}

fn st_max_cols(editor: &Editor, st: &mut EditorState) -> usize {
    st.max_cols(editor)
}

/// Where a document row is on the scrollbar column: `rows` is the content in rows.
pub(crate) fn stripe_y(rect: ratatui::layout::Rect, row: usize, total: usize) -> u16 {
    let track = rect.height.max(1) as usize;
    rect.y + (row * track / total.max(1)).min(track - 1) as u16
}

fn stripe_press(
    opts: &Opts<'_>,
    editor: &mut Editor,
    st: &mut EditorState,
    geo: &Geo,
    y: u16,
    out: &mut EditorOutput,
) {
    let Some(rect) = geo.stripe else {
        return;
    };
    let map = st.rows(editor, geo.wrap_cols);
    let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
    let total = map.total_rows();
    // A mark under the pointer: the caret goes there and the line is centered.
    let mut marks: Vec<usize> = opts
        .highlights
        .iter()
        .map(|h| h.range.start.min(editor.document.len()))
        .collect();
    if st.find.open {
        marks.extend(
            st.find
                .results(editor)
                .ranges
                .iter()
                .take(2000)
                .map(|r| r.start),
        );
    }
    for offset in marks {
        let (row, _) = row_col(&ctx, &map, offset, 0);
        if stripe_y(rect, row, total) == y {
            editor.carets.set_caret(&editor.document, offset);
            apply_scroll(
                editor,
                st,
                geo,
                ScrollRequest::ToCaret {
                    scroll_type: ScrollType::Center,
                    animate: false,
                },
            );
            out.carets_moved = true;
            return;
        }
    }
    stripe_scroll(opts, editor, st, geo, y);
}

/// Scrolls so that the row at the proportional position of `y` on the scrollbar is centered.
fn stripe_scroll(_opts: &Opts<'_>, editor: &mut Editor, st: &mut EditorState, geo: &Geo, y: u16) {
    let Some(rect) = geo.stripe else {
        return;
    };
    let map = st.rows(editor, geo.wrap_cols);
    let total = map.total_rows();
    let track = rect.height.max(1) as usize;
    let at = (y.saturating_sub(rect.y) as usize).min(track - 1);
    let row = at * total / track;
    let first = row.saturating_sub(geo.rows() / 2);
    st.first_row = first;
    super::clamp_scroll(editor, st, geo);
}
