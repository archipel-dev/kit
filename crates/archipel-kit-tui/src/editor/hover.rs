//! Hover, as Neovim's LSP floats do it: the pointer rests on the text for half a second and a
//! float opens with the messages of the diagnostics under it; resting on a fold placeholder
//! previews the folded lines; resting on a gutter icon or an error stripe mark names it.
//!
//! The rules are the shared ones of `archipel_kit::editor::logic::hover` (rest tolerance,
//! delays, which highlights apply, most severe first); this module measures in cells and
//! draws the float. Any key or press dismisses it until the pointer moves to something else.

use super::layout::Ctx;
use super::mouse::{View, locate, stripe_y};
use super::{EditorOutput, EditorState, Geo, Opts, row_col};
use crate::Look;
use crate::components::{fill_rect, put, text_width, wrap_text};
use crate::runtime::Runtime;
use crate::tooltip::frame;
use archipel_kit::editor::logic::hover::{
    HoverTarget, Rest, fold_dwell, highlight_at, messages_at, preview_lines, remaining, stripe_hit,
};
use archipel_kit::editor::model::Editor;
use archipel_kit::editor::{GutterArea, HighlightSeverity};
use archipel_kit::tokens::motion;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use std::rc::Rc;

/// Lines of a folded region a preview shows.
const PREVIEW_LINES: usize = 12;
/// The widest a float's text gets, in cells.
const FLOAT_WIDTH: u16 = 60;

/// What the hover tracking remembers between frames.
#[derive(Default)]
pub(crate) struct HoverState {
    rest: Option<Rest>,
    fold: Option<(usize, f64)>,
    /// A key or a press dismissed the float: it stays away until the pointer moves on.
    off: bool,
    /// When the editor next needs a frame for a delay that is running.
    pub wake: Option<f64>,
    /// The rectangle of the float drawn last, for tests and for hit testing.
    pub rect: Option<Rect>,
}

impl HoverState {
    /// A key or a press: the float goes away.
    pub fn dismiss(&mut self) {
        self.off = true;
        self.wake = None;
        self.rect = None;
    }
}

/// A float to draw: styled lines, hanging from a cell.
pub(crate) struct Float {
    lines: Vec<Vec<(String, Style)>>,
    /// The cell the float points at (the pointer).
    anchor: (u16, u16),
    /// Open to the left of the anchor instead of under it (the scrollbar column).
    left: bool,
}

fn severity_name(severity: HighlightSeverity) -> &'static str {
    match severity {
        HighlightSeverity::Error => "Error",
        HighlightSeverity::Warning => "Warning",
        HighlightSeverity::WeakWarning => "Weak warning",
        HighlightSeverity::Info => "Info",
        _ => "Result",
    }
}

/// The severity's color as a label on the float: the scheme's effect color, mixed toward the
/// text color until it reads (a yellow warning on a light float).
fn severity_color(look: &Look, severity: HighlightSeverity) -> ratatui::style::Color {
    let ed = &look.theme.editor;
    let c = match severity {
        HighlightSeverity::Error => ed.error_effect,
        HighlightSeverity::Warning => ed.warning_effect,
        HighlightSeverity::WeakWarning => ed.weak_warning_effect,
        _ => return look.palette.blue,
    };
    look.readable(u32::from_be_bytes([0, c.r(), c.g(), c.b()]))
}

/// The diagnostics under `offset` as float lines: the severity in its color, then the message.
fn diagnostics_float(
    look: &Look,
    opts: &Opts<'_>,
    offset: usize,
    anchor: (u16, u16),
    left: bool,
    width: u16,
) -> Option<Float> {
    let p = look.palette;
    let found = messages_at(opts.highlights, offset);
    if found.is_empty() {
        return None;
    }
    let mut lines: Vec<Vec<(String, Style)>> = Vec::new();
    for (n, (_, h)) in found.iter().enumerate() {
        if n > 0 {
            lines.push(Vec::new());
        }
        lines.push(vec![(
            format!("● {}", severity_name(h.severity)),
            Style::default()
                .fg(severity_color(look, h.severity))
                .add_modifier(Modifier::BOLD),
        )]);
        for line in wrap_text(h.message.as_deref().unwrap_or(""), width) {
            lines.push(vec![(line, Style::default().fg(p.white))]);
        }
    }
    Some(Float {
        lines,
        anchor,
        left,
    })
}

/// Tracks the pointer and builds the float for what it rests on; fills the hover fields of
/// `out`. Call it every frame after the rows were drawn.
pub(crate) fn update(
    opts: &Opts<'_>,
    editor: &Editor,
    rt: &Runtime,
    st: &mut EditorState,
    geo: &Geo,
    map: &Rc<super::layout::RowMap>,
    out: &mut EditorOutput,
) -> Option<Float> {
    st.hover.wake = None;
    let (Some((x, y)), Some(id)) = (rt.pointer_pos(), rt.hovered()) else {
        st.hover.rest = None;
        st.hover.fold = None;
        return None;
    };
    let ours = id == opts.id || id == opts.gutter_id() || id == opts.stripe_id();
    if !ours || st.drag.is_some() || st.menu.is_some() {
        st.hover.rest = None;
        st.hover.fold = None;
        return None;
    }
    let now = rt.now;
    let look = &rt.look;
    let width = FLOAT_WIDTH.min(geo.text.width.saturating_sub(4)).max(12);
    let view = View {
        geo: geo.clone(),
        map: map.clone(),
    };
    let pos = (f32::from(x), f32::from(y));
    // The key of what is under the pointer: an offset in the text, or a distinct number for
    // a gutter cell or a stripe row, so that moving between them restarts the rest.
    if id == opts.id {
        let at = locate(editor, &view, st, x, y);
        let (waited, restarted) = Rest::observe(&mut st.hover.rest, at.offset, pos, now, 0.5);
        if restarted {
            st.hover.off = false;
            st.hover.fold = None;
        }
        if st.hover.off {
            return None;
        }
        let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
        let layout = ctx.layout(at.visual);
        if let Some(seg) = layout.fold_at(at.col) {
            let waited = fold_dwell(&mut st.hover.fold, seg.off0, now);
            if waited >= f64::from(motion::FOLD_PREVIEW_DELAY) {
                out.fold_preview = Some(seg.off0..seg.off1);
                let text = preview_lines(
                    editor.document.text(),
                    seg.off0..seg.off1,
                    PREVIEW_LINES + 1,
                );
                let p = look.palette;
                let mut lines: Vec<Vec<(String, Style)>> = text
                    .iter()
                    .take(PREVIEW_LINES)
                    .map(|l| vec![(l.replace('\t', "    "), Style::default().fg(p.light_grey))])
                    .collect();
                if text.len() > PREVIEW_LINES {
                    lines.push(vec![(
                        "…".to_owned(),
                        Style::default().fg(p.grey).add_modifier(Modifier::BOLD),
                    )]);
                }
                return Some(Float {
                    lines,
                    anchor: (x, y),
                    left: false,
                });
            }
            st.hover.wake = Some(now + remaining(motion::FOLD_PREVIEW_DELAY, waited));
            return None;
        }
        st.hover.fold = None;
        if waited >= f64::from(motion::EDITOR_TOOLTIP_DELAY) && at.virtual_cols == 0 {
            let highlight = highlight_at(opts.highlights, at.offset);
            out.hover = Some(HoverTarget {
                offset: at.offset,
                highlight,
            });
            return diagnostics_float(look, opts, at.offset, (x, y), false, width);
        }
        st.hover.wake = Some(now + remaining(motion::EDITOR_TOOLTIP_DELAY, waited));
        return None;
    }
    if id == opts.stripe_id() {
        let stripe = geo.stripe?;
        let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
        let total = map.total_rows();
        let len = editor.document.len();
        let hit = stripe_hit(opts.highlights, usize::from(y), |offset| {
            let (row, _) = row_col(&ctx, map, offset.min(len), 0);
            usize::from(stripe_y(stripe, row, total))
        });
        out.stripe_hover = hit;
        let key = usize::MAX - usize::from(y);
        let (waited, restarted) = Rest::observe(&mut st.hover.rest, key, pos, now, 0.5);
        if restarted {
            st.hover.off = false;
        }
        let hit = hit?;
        if st.hover.off {
            return None;
        }
        if waited >= f64::from(motion::EDITOR_TOOLTIP_DELAY) {
            let start = opts.highlights[hit].range.start;
            return diagnostics_float(look, opts, start, (stripe.x, y), true, width);
        }
        st.hover.wake = Some(now + remaining(motion::EDITOR_TOOLTIP_DELAY, waited));
        return None;
    }
    // The gutter: an icon names itself after the delay.
    if let GutterArea::Icon(lane) = geo.gutter_area(x) {
        let at = locate(editor, &view, st, geo.text.x, y);
        let icon = opts
            .icons
            .iter()
            .find(|i| i.lane == lane && i.line == at.logical)?;
        let key = usize::MAX / 2 - (at.logical * 64 + lane);
        let (waited, restarted) = Rest::observe(&mut st.hover.rest, key, pos, now, 0.5);
        if restarted {
            st.hover.off = false;
        }
        if st.hover.off {
            return None;
        }
        if waited >= f64::from(motion::EDITOR_TOOLTIP_DELAY) {
            let p = look.palette;
            return Some(Float {
                lines: vec![vec![(icon.name.clone(), Style::default().fg(p.white))]],
                anchor: (x, y),
                left: false,
            });
        }
        st.hover.wake = Some(now + remaining(motion::EDITOR_TOOLTIP_DELAY, waited));
        return None;
    }
    st.hover.rest = None;
    None
}

/// Draws the float inside `bounds`: under the anchor cell, above it when there is no room,
/// left of it for the scrollbar column.
pub(crate) fn draw(
    look: &Look,
    st: &mut EditorState,
    float: &Float,
    bounds: Rect,
    buf: &mut Buffer,
) {
    let p = look.palette;
    let inner_w = float
        .lines
        .iter()
        .map(|l| l.iter().map(|(t, _)| text_width(t)).sum::<u16>())
        .max()
        .unwrap_or(0);
    let width = (inner_w + 4).min(bounds.width);
    let height = (float.lines.len() as u16 + 2).min(bounds.height);
    if width < 5 || height < 3 {
        st.hover.rect = None;
        return;
    }
    let (ax, ay) = float.anchor;
    let x = if float.left {
        ax.saturating_sub(width)
    } else {
        ax.min(bounds.right().saturating_sub(width))
    }
    .max(bounds.x);
    let y = if float.left {
        ay.min(bounds.bottom().saturating_sub(height))
    } else if ay + 1 + height <= bounds.bottom() {
        ay + 1
    } else {
        ay.saturating_sub(height)
    }
    .max(bounds.y);
    let rect = Rect::new(x, y, width, height);
    st.hover.rect = Some(rect);
    let bg = p.one_bg;
    fill_rect(buf, rect, Style::default().bg(bg).fg(p.white));
    frame(buf, rect, Style::default().fg(p.grey).bg(bg));
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height - 2,
    );
    for (n, line) in float.lines.iter().enumerate() {
        let row = inner.y + n as u16;
        let mut cx = inner.x;
        for (text, style) in line {
            cx = put(buf, inner, cx, row, text, style.bg(bg));
        }
    }
}
