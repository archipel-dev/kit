//! The mouse (spec §2): placing the caret, multi-clicks, drag selection with autoscroll,
//! multiple carets, column selection, the gutter, and the Ctrl/Cmd-click navigation request.

use super::gutter::GutterLayout;
use super::input::key_modifiers;
use super::state::{Drag, Frame, Geo, Unit, ViewState};
use crate::editor::decorations::{GutterArea, GutterClick, NavigationKind, NavigationRequest};
use crate::editor::model::{Editor, LogicalPosition, Os, ScrollRequest, ScrollType, word_range_at};
use crate::tokens::{editor as metrics, motion};
use egui::{CursorIcon, Event, Modifiers, PointerButton, Pos2, Response, Ui};

/// What the mouse needs.
pub(crate) struct Env<'a> {
    pub geo: &'a Geo,
    pub frame: &'a Frame,
    pub gutter: Option<&'a GutterLayout>,
    pub resp_text: &'a Response,
    pub resp_gutter: Option<&'a Response>,
    /// The find bar of the last frame: presses on it are not the text's.
    pub overlay: Option<egui::Rect>,
    /// The gutter icons: they are buttons of their own.
    pub icons: &'a [crate::editor::GutterIcon],
    pub os: Os,
    pub gutter_click_selects_line: bool,
}

/// What the mouse did in a frame.
#[derive(Default)]
pub(crate) struct MouseOut {
    pub navigation: Option<NavigationRequest>,
    pub gutter_clicks: Vec<GutterClick>,
    pub scroll: Option<ScrollRequest>,
    /// The carets were set by the mouse (so the view follows).
    pub moved: bool,
    pub request_focus: bool,
}

/// Where a point of the screen is in the text.
pub(crate) struct Located {
    pub visual: usize,
    /// The column boundary, counted from the start of the visual line.
    pub col: usize,
    pub offset: usize,
    pub virtual_cols: usize,
    pub logical: usize,
}

/// The text position under `p` (clamped to the document).
pub(crate) fn locate(editor: &Editor, env_geo: &Geo, frame: &Frame, p: Pos2) -> Located {
    let total = frame.map.total_rows();
    let row = env_geo
        .row_at(p.y)
        .clamp(0, total.saturating_sub(1) as isize) as usize;
    let (visual, k) = frame.map.visual_at_row(row);
    let layout = frame.layout(&editor.document, visual, editor.settings.tab_size);
    let base = frame.map.wrap_cols().map_or(0, |wc| k * wc);
    let col = (env_geo.col_at(p.x).max(0) as usize) + base;
    let col = match frame.map.wrap_cols() {
        Some(wc) => col.min(base + wc),
        None => col,
    };
    let (offset, virtual_cols) = layout.offset_at(col);
    Located {
        visual,
        col,
        offset,
        virtual_cols,
        logical: layout.logical,
    }
}

fn inside_selection(editor: &Editor, offset: usize) -> bool {
    editor
        .carets
        .carets()
        .iter()
        .any(|c| c.has_selection() && c.selection_start() <= offset && offset < c.selection_end())
}

fn line_range(editor: &Editor, offset: usize) -> (usize, usize) {
    let doc = &editor.document;
    let line = doc.line_number(offset);
    (doc.line_start_offset(line), doc.line_end_with_break(line))
}

fn word_or_line(editor: &Editor, offset: usize) -> (usize, usize) {
    let doc = &editor.document;
    if let Some(range) = word_range_at(
        doc,
        offset,
        editor.settings.mouse_selection_honors_camel_humps && editor.settings.camel_humps,
    ) {
        return range;
    }
    // No word: the text of the line, without its break.
    let line = doc.line_number(offset);
    (doc.line_start_offset(line), doc.line_end_offset(line))
}

/// Sets the primary selection to `anchor..lead` keeping the other carets when `multi`.
fn set_selection(editor: &mut Editor, anchor: usize, lead: usize, multi: bool) {
    if multi {
        let doc = &editor.document;
        let (anchor, lead) = (doc.snap(anchor), doc.snap(lead));
        let caret = editor.carets.primary_mut();
        caret.select(anchor, lead);
        editor.carets.normalize();
    } else {
        editor.carets.set_selection(&editor.document, anchor, lead);
    }
}

fn logical_position(at: &Located) -> LogicalPosition {
    // Columns past the end of the line count as virtual columns.
    LogicalPosition::new(at.logical, at.col)
}

/// Runs the mouse of this frame.
pub(crate) fn handle(
    ui: &Ui,
    editor: &mut Editor,
    st: &mut ViewState,
    env: &Env<'_>,
    out: &mut MouseOut,
) {
    let now = ui.input(|i| i.time);
    let events: Vec<Event> = ui.input(|i| i.events.clone());
    for event in &events {
        match event {
            Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers,
            } => press(editor, st, env, out, *pos, *button, *modifiers, now),
            Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers,
            } => release(editor, st, env, out, *pos, *button, *modifiers),
            Event::PointerMoved(pos) if st.drag.is_some() => {
                drag(editor, st, env, out, *pos);
            }
            _ => {}
        }
    }
    // A stationary pointer outside the viewport keeps the autoscroll running.
    if st.drag.is_some() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos())
            && ui.input(|i| i.pointer.primary_down() || i.pointer.middle_down())
        {
            drag(editor, st, env, out, pos);
            autoscroll(ui, editor, st, env, out, pos, now);
        }
    } else {
        st.autoscroll_counter = 0;
        st.autoscroll_last = now;
    }
}

#[allow(clippy::too_many_arguments)]
fn press(
    editor: &mut Editor,
    st: &mut ViewState,
    env: &Env<'_>,
    out: &mut MouseOut,
    pos: Pos2,
    button: PointerButton,
    modifiers: Modifiers,
    now: f64,
) {
    let in_gutter = env.gutter.is_some_and(|g| g.rect.contains(pos))
        && env.resp_gutter.is_some_and(|r| r.contains_pointer());
    let in_text = env.geo.area.contains(pos)
        && env.resp_text.contains_pointer()
        && !env.overlay.is_some_and(|rect| rect.contains(pos));
    if !(in_text || in_gutter) {
        return;
    }
    out.request_focus = true;
    let km = key_modifiers(modifiers, env.os);
    let nav_modifier = match env.os {
        Os::MacOs => km.command,
        Os::WindowsLinux => km.ctrl,
    };
    if in_gutter {
        gutter_press(editor, st, env, out, pos, button, modifiers, now);
        return;
    }
    let at = locate(editor, env.geo, env.frame, pos);
    let doc_tab = editor.settings.tab_size;
    match button {
        PointerButton::Secondary => {
            if !inside_selection(editor, at.offset) {
                editor.carets.set_caret(&editor.document, at.offset);
                out.moved = true;
            }
        }
        PointerButton::Middle => {
            if modifiers.shift && modifiers.alt {
                // Alt+Shift+middle: a rectangle from the existing anchor.
                let position = logical_position(&at);
                editor
                    .carets
                    .extend_block_selection(&editor.document, doc_tab, position);
            } else {
                let position = logical_position(&at);
                editor
                    .carets
                    .start_block_selection(&editor.document, doc_tab, position);
            }
            st.drag = Some(Drag::Column);
            st.click_pos = pos;
            out.moved = true;
        }
        PointerButton::Primary => {
            primary_press(editor, st, env, out, pos, modifiers, &at, nav_modifier, now);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn primary_press(
    editor: &mut Editor,
    st: &mut ViewState,
    env: &Env<'_>,
    out: &mut MouseOut,
    pos: Pos2,
    modifiers: Modifiers,
    at: &Located,
    nav_modifier: bool,
    now: f64,
) {
    let tab = editor.settings.tab_size;
    // Multi-click counting.
    if now - st.click_time <= motion::MULTI_CLICK_INTERVAL as f64
        && pos.distance(st.click_pos) <= 4.0
    {
        st.click_count += 1;
    } else {
        st.click_count = 1;
        st.unit = None;
    }
    st.click_time = now;
    st.click_pos = pos;
    let layout = env
        .frame
        .layout(&editor.document, at.visual, editor.settings.tab_size);

    // A click on a collapsed fold placeholder expands it on release.
    if st.click_count == 1
        && !modifiers.shift
        && !modifiers.alt
        && let Some(seg) = layout.fold_at(at.col)
    {
        st.drag = Some(Drag::OnFold {
            region_start: seg.off0,
            press: pos,
        });
        return;
    }

    if modifiers.alt && modifiers.shift {
        // Alt+Shift+click: add a caret, or remove the one that is there.
        st.unit = None;
        if let Some(index) = editor
            .carets
            .carets()
            .iter()
            .position(|c| c.offset == at.offset && !c.has_selection())
        {
            editor.carets.remove_caret(index);
            out.moved = true;
            return;
        }
        if editor.carets.add_caret(&editor.document, at.offset) {
            st.drag = Some(Drag::Char {
                anchor: at.offset,
                multi: true,
            });
        }
        out.moved = true;
        return;
    }
    if modifiers.alt && !modifiers.shift && !nav_modifier {
        // Alt+click and drag: column selection.
        let position = logical_position(at);
        editor
            .carets
            .start_block_selection(&editor.document, tab, position);
        st.drag = Some(Drag::Column);
        st.unit = None;
        out.moved = true;
        return;
    }
    if nav_modifier && !modifiers.shift {
        out.navigation = Some(NavigationRequest {
            offset: at.offset,
            kind: if modifiers.alt {
                NavigationKind::Implementation
            } else {
                NavigationKind::Declaration
            },
        });
        editor.carets.set_caret(&editor.document, at.offset);
        st.drag = None;
        out.moved = true;
        return;
    }
    if modifiers.shift {
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
        out.moved = true;
        return;
    }
    match st.click_count {
        1 => {
            if inside_selection(editor, at.offset) {
                st.drag = Some(Drag::InSelection {
                    offset: at.offset,
                    press: pos,
                });
            } else {
                editor.carets.set_caret(&editor.document, at.offset);
                st.drag = Some(Drag::Char {
                    anchor: at.offset,
                    multi: false,
                });
                st.unit = None;
                out.moved = true;
            }
        }
        2 => {
            let (s, e) = word_or_line(editor, at.offset);
            set_selection(editor, s, e, false);
            st.unit = Some(Unit::Word(s, e));
            st.drag = Some(Drag::Word { start: s, end: e });
            out.moved = true;
        }
        _ => {
            let (s, e) = line_range(editor, at.offset);
            set_selection(editor, s, e, false);
            st.unit = Some(Unit::Line(s, e));
            st.drag = Some(Drag::Line { start: s, end: e });
            out.moved = true;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn gutter_press(
    editor: &mut Editor,
    st: &mut ViewState,
    env: &Env<'_>,
    out: &mut MouseOut,
    pos: Pos2,
    button: PointerButton,
    modifiers: Modifiers,
    now: f64,
) {
    let Some(gutter) = env.gutter else {
        return;
    };
    let at = locate(editor, env.geo, env.frame, pos);
    let line = at.logical;
    let area = gutter.area_at(pos.x);
    // Icons and fold chevrons are buttons: they report their own clicks.
    if let GutterArea::Icon(lane) = area
        && env.icons.iter().any(|i| i.line == line && i.lane == lane)
    {
        return;
    }
    if gutter.fold.is_some_and(|(a, b)| pos.x >= a && pos.x < b)
        && super::gutter::region_at_line(editor, line).is_some()
    {
        return;
    }
    st.click_time = now;
    if button != PointerButton::Primary {
        out.gutter_clicks.push(GutterClick {
            line,
            area,
            button,
            modifiers,
        });
        return;
    }
    let doc = &editor.document;
    let line_start = doc.line_start_offset(line);
    let line_break_end = doc.line_end_with_break(line);
    if modifiers.shift && area == GutterArea::LineNumber {
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
            press: pos,
        });
        out.moved = true;
        return;
    }
    if area == GutterArea::LineNumber {
        if env.gutter_click_selects_line {
            set_selection(editor, line_start, line_break_end, false);
            st.drag = Some(Drag::GutterLines {
                anchor_line: line,
                started: true,
                press: pos,
            });
            out.moved = true;
        } else {
            st.drag = Some(Drag::GutterLines {
                anchor_line: line,
                started: false,
                press: pos,
            });
        }
    } else {
        st.drag = Some(Drag::GutterChar {
            anchor: line_start,
            started: false,
            press: pos,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn release(
    editor: &mut Editor,
    st: &mut ViewState,
    env: &Env<'_>,
    out: &mut MouseOut,
    pos: Pos2,
    button: PointerButton,
    modifiers: Modifiers,
) {
    let Some(state) = st.drag.take() else {
        return;
    };
    match state {
        Drag::InSelection { offset, press } => {
            if pos.distance(press) < 2.0 {
                editor.carets.set_caret(&editor.document, offset);
                out.moved = true;
            }
        }
        Drag::OnFold {
            region_start,
            press,
        } => {
            if pos.distance(press) <= 4.0
                && let Some(index) = editor
                    .folding
                    .regions()
                    .iter()
                    .position(|r| r.start == region_start && !r.expanded)
            {
                editor.toggle_fold_region(index, false);
                out.moved = true;
            }
        }
        Drag::GutterLines {
            anchor_line,
            started: false,
            press,
        } => {
            if pos.distance(press) < metrics::DRAG_SENSITIVITY {
                let at = locate(editor, env.geo, env.frame, press);
                out.gutter_clicks.push(GutterClick {
                    line: anchor_line.min(editor.document.line_count() - 1),
                    area: GutterArea::LineNumber,
                    button,
                    modifiers,
                });
                let _ = at;
            }
        }
        Drag::GutterChar {
            started: false,
            press,
            ..
        } => {
            if pos.distance(press) < metrics::DRAG_SENSITIVITY {
                let at = locate(editor, env.geo, env.frame, press);
                let area = env.gutter.map_or(GutterArea::Other, |g| g.area_at(press.x));
                out.gutter_clicks.push(GutterClick {
                    line: at.logical,
                    area,
                    button,
                    modifiers,
                });
            }
        }
        Drag::Char { .. } | Drag::Word { .. } | Drag::Line { .. } | Drag::Column => {
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

fn drag(editor: &mut Editor, st: &mut ViewState, env: &Env<'_>, out: &mut MouseOut, pos: Pos2) {
    let Some(state) = st.drag.clone() else {
        return;
    };
    let tab = editor.settings.tab_size;
    // Inside the text area the pointer maps to a position; in the gutter or beyond the edges,
    // to the nearest one.
    let at = locate(editor, env.geo, env.frame, pos);
    match state {
        Drag::Char { anchor, multi } => {
            set_selection(editor, anchor, at.offset, multi);
            out.moved = true;
        }
        Drag::Word { start, end } => {
            let (ws, we) = word_or_line(editor, at.offset);
            if at.offset < start {
                set_selection(editor, end, ws.min(at.offset), false);
            } else {
                set_selection(editor, start, we.max(at.offset).max(end), false);
            }
            out.moved = true;
        }
        Drag::Line { start, end } => {
            let (ls, le) = line_range(editor, at.offset);
            if ls < start {
                set_selection(editor, end, ls, false);
            } else {
                set_selection(editor, start, le, false);
            }
            out.moved = true;
        }
        Drag::Column => {
            let position = logical_position(&at);
            editor
                .carets
                .extend_block_selection(&editor.document, tab, position);
            out.moved = true;
        }
        Drag::GutterLines {
            anchor_line,
            started,
            press,
        } => {
            if !started && pos.distance(press) < metrics::DRAG_SENSITIVITY {
                return;
            }
            st.drag = Some(Drag::GutterLines {
                anchor_line,
                started: true,
                press,
            });
            let doc = &editor.document;
            let anchor_line = anchor_line.min(doc.line_count() - 1);
            let line = at.logical;
            let (anchor, lead) = if line >= anchor_line {
                (
                    doc.line_start_offset(anchor_line),
                    doc.line_end_with_break(line),
                )
            } else {
                (
                    doc.line_end_with_break(anchor_line),
                    doc.line_start_offset(line),
                )
            };
            set_selection(editor, anchor, lead, false);
            out.moved = true;
        }
        Drag::GutterChar {
            anchor,
            started,
            press,
        } => {
            if !started && pos.distance(press) < metrics::DRAG_SENSITIVITY {
                return;
            }
            st.drag = Some(Drag::GutterChar {
                anchor,
                started: true,
                press,
            });
            set_selection(editor, anchor, at.offset, false);
            out.moved = true;
        }
        Drag::InSelection { offset, press } => {
            if pos.distance(press) >= 2.0 {
                // No drag and drop: a drag from inside the selection selects anew.
                st.drag = Some(Drag::Char {
                    anchor: offset,
                    multi: false,
                });
                set_selection(editor, offset, at.offset, false);
                out.moved = true;
            }
        }
        Drag::OnFold {
            region_start,
            press,
        } => {
            if pos.distance(press) > 4.0 {
                st.drag = Some(Drag::Char {
                    anchor: region_start,
                    multi: false,
                });
                set_selection(editor, region_start, at.offset, false);
                out.moved = true;
            }
        }
    }
    if out.moved && out.scroll.is_none() {
        out.scroll = Some(ScrollRequest::ToCaret {
            scroll_type: ScrollType::Relative,
            animate: false,
        });
    }
}

/// Moves the selection head while the pointer is beyond the viewport's edge (spec §2.3): a
/// timer ticks; every `floor(20 / d) + 1` ticks the head moves `1 + floor(d / 20)` lines (or
/// columns), where `d` is the distance in pixels beyond the edge.
fn autoscroll(
    ui: &Ui,
    editor: &mut Editor,
    st: &mut ViewState,
    env: &Env<'_>,
    out: &mut MouseOut,
    pos: Pos2,
    now: f64,
) {
    let area = env.geo.area;
    let dy = if pos.y < area.min.y {
        pos.y - area.min.y
    } else if pos.y > area.max.y {
        pos.y - area.max.y
    } else {
        0.0
    };
    let dx = if pos.x < area.min.x {
        pos.x - area.min.x
    } else if pos.x > area.max.x {
        pos.x - area.max.x
    } else {
        0.0
    };
    if dx == 0.0 && dy == 0.0 {
        st.autoscroll_counter = 0;
        st.autoscroll_last = now;
        return;
    }
    let tick = motion::EDITOR_AUTOSCROLL_TICK as f64;
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f32(
            motion::EDITOR_AUTOSCROLL_TICK,
        ));
    let ticks = (((now - st.autoscroll_last) / tick).floor() as u32).min(8);
    if ticks == 0 {
        return;
    }
    st.autoscroll_last += ticks as f64 * tick;
    let (mut steps_y, mut steps_x) = (0isize, 0isize);
    for _ in 0..ticks {
        st.autoscroll_counter += 1;
        let d = dy.abs().max(dx.abs());
        let every = (metrics::AUTOSCROLL_DISTANCE / d).floor() as u32 + 1;
        if st.autoscroll_counter >= every {
            st.autoscroll_counter = 0;
            let n = 1 + (d / metrics::AUTOSCROLL_DISTANCE).floor() as isize;
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
    let tab = editor.settings.tab_size;
    let head = editor.carets.primary().offset;
    let doc_visual = env.frame.visual_of(editor, head);
    let layout = env.frame.layout(&editor.document, doc_visual, tab);
    let head_col = layout.col_of(head);
    let new_visual = (doc_visual as isize + steps_y)
        .clamp(0, env.frame.map.visual_lines() as isize - 1) as usize;
    let new_layout = env.frame.layout(&editor.document, new_visual, tab);
    let col = (head_col as isize + steps_x).max(0) as usize;
    let (offset, _) = new_layout.offset_at(col);
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
        Some(Drag::Column) => {
            let position = LogicalPosition::new(new_layout.logical, col);
            editor
                .carets
                .extend_block_selection(&editor.document, tab, position);
        }
        Some(Drag::GutterLines { anchor_line, .. }) => {
            let doc = &editor.document;
            let line = new_layout.logical;
            let (anchor, lead) = if line >= anchor_line {
                (
                    doc.line_start_offset(anchor_line),
                    doc.line_end_with_break(line),
                )
            } else {
                (
                    doc.line_end_with_break(anchor_line),
                    doc.line_start_offset(line),
                )
            };
            set_selection(editor, anchor, lead, false);
        }
        Some(Drag::GutterChar { anchor, .. }) => set_selection(editor, anchor, offset, false),
        _ => return,
    }
    out.moved = true;
    out.scroll = Some(ScrollRequest::ToCaret {
        scroll_type: ScrollType::Relative,
        animate: false,
    });
}

/// The cursor for the pointer at `pos` over the text: the arrow over a selection (it signals
/// that it can be dragged), the hand over a collapsed fold, the text beam elsewhere.
pub(crate) fn cursor_at(editor: &Editor, env: &Env<'_>, pos: Pos2) -> CursorIcon {
    let at = locate(editor, env.geo, env.frame, pos);
    let layout = env
        .frame
        .layout(&editor.document, at.visual, editor.settings.tab_size);
    if layout.fold_at(at.col).is_some() {
        return CursorIcon::PointingHand;
    }
    if at.virtual_cols == 0 && inside_selection(editor, at.offset) {
        return CursorIcon::Default;
    }
    CursorIcon::Text
}
