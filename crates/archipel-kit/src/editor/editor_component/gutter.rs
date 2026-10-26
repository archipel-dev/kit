//! The gutter (spec §5): line numbers, change markers, icon lanes and fold chevrons, left to
//! right, on the editor background.

use super::state::{Frame, Geo, ViewState};
use crate::editor::decorations::{ChangeKind, ChangeMarker, GutterArea, GutterClick, GutterIcon};
use crate::editor::model::Editor;
use crate::icons;
use crate::theme::EditorColors;
use crate::tokens::{editor as metrics, motion};
use egui::{
    Align2, Color32, CursorIcon, FontId, Id, Painter, Pos2, Rect, Sense, Stroke, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// What the gutter shows.
#[derive(Clone, Copy)]
pub(crate) struct GutterCfg {
    pub numbers: bool,
    pub lines: usize,
    pub change_markers: bool,
    pub lanes: usize,
    pub folding: bool,
}

impl GutterCfg {
    pub fn is_empty(&self) -> bool {
        !self.numbers && !self.change_markers && self.lanes == 0 && !self.folding
    }
}

/// The gutter's columns on screen.
#[derive(Clone, Debug)]
pub(crate) struct GutterLayout {
    pub rect: Rect,
    pub numbers: Option<(f32, f32)>,
    pub change: Option<(f32, f32)>,
    pub lanes: Vec<(f32, f32)>,
    pub fold: Option<(f32, f32)>,
}

/// The width of the gutter for `cfg`; `number_cell` is the width of a digit of the line
/// number font.
pub(crate) fn width(cfg: &GutterCfg, number_cell: f32) -> f32 {
    if cfg.is_empty() {
        return 0.0;
    }
    let mut w = metrics::GUTTER_ANNOTATION;
    if cfg.numbers {
        let digits = cfg.lines.max(1).to_string().len().max(2);
        w += metrics::GUTTER_GAP + digits as f32 * number_cell + metrics::GUTTER_GAP;
    }
    if cfg.change_markers {
        w += metrics::CHANGE_MARKER_AREA;
    }
    if cfg.lanes > 0 {
        w += cfg.lanes as f32 * metrics::GUTTER_LANE + metrics::GUTTER_AFTER_ICONS;
    }
    if cfg.folding {
        w += metrics::FOLD_COLUMN;
    }
    w
}

impl GutterLayout {
    pub fn new(rect: Rect, cfg: &GutterCfg, number_cell: f32) -> GutterLayout {
        let mut x = rect.min.x + metrics::GUTTER_ANNOTATION;
        let mut layout = GutterLayout {
            rect,
            numbers: None,
            change: None,
            lanes: Vec::new(),
            fold: None,
        };
        if cfg.numbers {
            let digits = cfg.lines.max(1).to_string().len().max(2);
            x += metrics::GUTTER_GAP;
            let w = digits as f32 * number_cell;
            layout.numbers = Some((x, x + w));
            x += w + metrics::GUTTER_GAP;
        }
        if cfg.change_markers {
            layout.change = Some((x, x + metrics::CHANGE_MARKER_AREA));
            x += metrics::CHANGE_MARKER_AREA;
        }
        for _ in 0..cfg.lanes {
            layout.lanes.push((x, x + metrics::GUTTER_LANE));
            x += metrics::GUTTER_LANE;
        }
        if cfg.lanes > 0 {
            x += metrics::GUTTER_AFTER_ICONS;
        }
        if cfg.folding {
            layout.fold = Some((x, x + metrics::FOLD_COLUMN));
        }
        layout
    }

    /// Which part of the gutter screen x falls in.
    pub fn area_at(&self, x: f32) -> GutterArea {
        let inside = |range: Option<(f32, f32)>| range.is_some_and(|(a, b)| x >= a && x < b);
        if let Some(lane) = self.lanes.iter().position(|&(a, b)| x >= a && x < b) {
            return GutterArea::Icon(lane);
        }
        if inside(self.change) {
            return GutterArea::ChangeMarker;
        }
        // The numbers column reaches the gaps around it: a click beside a number is on it.
        if let Some((a, b)) = self.numbers
            && x >= self.rect.min.x
            && x < b + metrics::GUTTER_GAP
            && x >= a - metrics::GUTTER_GAP - metrics::GUTTER_ANNOTATION
        {
            return GutterArea::LineNumber;
        }
        GutterArea::Other
    }
}

/// The fold region drawn at a line, as an index into the model's regions.
pub(crate) fn region_at_line(editor: &Editor, line: usize) -> Option<usize> {
    let doc = &editor.document;
    let start = doc.line_start_offset(line);
    let end = doc.line_end_offset(line);
    let regions = editor.folding.regions();
    let first = regions.partition_point(|r| r.start < start);
    let mut found = None;
    for (i, region) in regions.iter().enumerate().skip(first) {
        if region.start > end {
            break;
        }
        if editor.folding.is_offset_folded(region.start) {
            continue;
        }
        if !region.expanded {
            return Some(i);
        }
        found.get_or_insert(i);
    }
    found
}

/// What to paint.
pub(crate) struct GutterInput<'a> {
    pub editor: &'a mut Editor,
    pub frame: &'a Frame,
    pub geo: &'a Geo,
    pub layout: &'a GutterLayout,
    pub cfg: GutterCfg,
    pub colors: EditorColors,
    pub number_font: &'a FontId,
    pub icons: &'a [GutterIcon],
    pub markers: &'a [ChangeMarker],
    pub first_row: usize,
    pub last_row: usize,
    pub enabled: bool,
    pub id: Id,
}

/// What the gutter reported.
#[derive(Default)]
pub(crate) struct GutterOut {
    pub clicks: Vec<GutterClick>,
    pub toggled: bool,
}

/// Paints the gutter and its widgets. The gutter's own press and drag handling is in the
/// mouse module; this draws, and operates the icons and the fold chevrons.
pub(crate) fn show(ui: &mut Ui, st: &mut ViewState, input: GutterInput<'_>) -> GutterOut {
    let GutterInput {
        editor,
        frame,
        geo,
        layout,
        cfg,
        colors,
        number_font,
        icons: gutter_icons,
        markers,
        first_row,
        last_row,
        enabled,
        id,
    } = input;
    let mut out = GutterOut::default();
    if cfg.is_empty() || geo.area.height() <= 0.0 {
        return out;
    }
    let now = ui.input(|i| i.time);
    let painter = ui
        .painter()
        .with_clip_rect(layout.rect.intersect(ui.clip_rect()));
    let tab = editor.settings.tab_size;

    // The chevrons of expanded regions show while the pointer is over the gutter.
    let hovering = ui.rect_contains_pointer(layout.rect);
    if hovering {
        st.gutter_left_at = None;
    } else if st.gutter_left_at.is_none() && st.chevron_alpha > 0.0 {
        st.gutter_left_at = Some(now);
    }
    let dt = (now - st.chevron_time).clamp(0.0, 0.25) as f32;
    st.chevron_time = now;
    let target_up = hovering;
    let fading_out = st
        .gutter_left_at
        .is_some_and(|left| now - left >= motion::FOLD_CHEVRON_FADE_OUT_DELAY as f64);
    if target_up {
        st.chevron_alpha = (st.chevron_alpha + dt / motion::FOLD_CHEVRON_FADE_IN).min(1.0);
    } else if fading_out {
        st.chevron_alpha = (st.chevron_alpha - dt / motion::FOLD_CHEVRON_FADE_OUT).max(0.0);
    }
    if (target_up && st.chevron_alpha < 1.0) || (!target_up && st.chevron_alpha > 0.0) {
        ui.ctx().request_repaint();
    }

    let caret_lines: Vec<usize> = editor
        .carets
        .carets()
        .iter()
        .map(|c| editor.document.line_number(c.offset))
        .collect();

    // Classic: the folding outline is always there: square anchors at both ends of every
    // region and a line along the region the caret is in (looks spec §2.12).
    let outline = crate::tokens::metrics(ui.ctx()).folding == crate::look::FoldingStyle::Outline;
    let caret_region = caret_lines.first().and_then(|&caret| {
        editor
            .folding
            .regions()
            .iter()
            .filter(|region| region.expanded)
            .map(|region| {
                (
                    editor.document.line_number(region.start),
                    editor.document.line_number(region.end),
                )
            })
            .filter(|&(a, b)| a <= caret && caret <= b && a < b)
            .min_by_key(|&(a, b)| b - a)
    });

    let mut last_logical = usize::MAX;
    let mut row = first_row;
    while row <= last_row {
        let (v, k) = frame.map.visual_at_row(row);
        if k != 0 {
            row += 1;
            continue;
        }
        let line = frame.map.visual_to_logical(v);
        if line == last_logical {
            row += 1;
            continue;
        }
        last_logical = line;
        let top = geo.row_top(row);
        let row_rect =
            |x0: f32, x1: f32| Rect::from_min_max(pos2(x0, top), pos2(x1, top + geo.row_h));
        // Line number.
        if let Some((_, right)) = layout.numbers {
            let color = if caret_lines.contains(&line) && enabled {
                colors.line_number_caret
            } else {
                colors.line_number
            };
            painter.text(
                pos2(right, top + geo.row_h / 2.0),
                Align2::RIGHT_CENTER,
                (line + 1).to_string(),
                number_font.clone(),
                color,
            );
        }
        // Change markers.
        if let Some((x0, _)) = layout.change {
            for marker in markers {
                if marker.lines.contains(&line)
                    || (marker.kind == ChangeKind::Deleted && marker.lines.start == line)
                {
                    let color = match marker.kind {
                        ChangeKind::Added => colors.change_added,
                        ChangeKind::Modified => colors.change_modified,
                        ChangeKind::Deleted => colors.change_deleted,
                    };
                    let x = x0 + (metrics::CHANGE_MARKER_AREA - metrics::CHANGE_MARKER) / 2.0;
                    let rect = if marker.kind == ChangeKind::Deleted {
                        Rect::from_min_size(pos2(x, top - 1.0), vec2(metrics::CHANGE_MARKER, 2.0))
                    } else {
                        Rect::from_min_size(pos2(x, top), vec2(metrics::CHANGE_MARKER, geo.row_h))
                    };
                    painter.rect_filled(rect, 0.0, color);
                }
            }
        }
        // Icons: operated like buttons so they are reachable and named.
        for icon in gutter_icons.iter().filter(|i| i.line == line) {
            let Some(&(x0, x1)) = layout.lanes.get(icon.lane) else {
                continue;
            };
            let rect = row_rect(x0, x1);
            let resp = ui.interact(
                rect,
                id.with(("gutter-icon", icon.lane, icon.line)),
                if enabled {
                    Sense::CLICK
                } else {
                    Sense::hover()
                },
            );
            resp.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Button, enabled, icon.name.clone())
            });
            let color = icon.tone.resolve(ui.ctx());
            let size = vec2(metrics::GUTTER_ICON, metrics::GUTTER_ICON);
            icons::paint(
                &painter,
                Rect::from_center_size(rect.center(), size),
                icon.glyph,
                color,
            );
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() || resp.secondary_clicked() {
                out.clicks.push(GutterClick {
                    line,
                    area: GutterArea::Icon(icon.lane),
                    button: if resp.secondary_clicked() {
                        egui::PointerButton::Secondary
                    } else {
                        egui::PointerButton::Primary
                    },
                    modifiers: ui.input(|i| i.modifiers),
                });
            }
        }
        // Classic outline: the line along the caret's region and the end anchors.
        if outline && let Some((x0, x1)) = layout.fold {
            let center_x = (x0 + x1) / 2.0;
            let anchor = metrics::FOLD_ANCHOR;
            if let Some((a, b)) = caret_region
                && line >= a
                && line <= b
            {
                let y0 = if line == a {
                    top + geo.row_h / 2.0 + anchor / 2.0
                } else {
                    top
                };
                let y1 = if line == b {
                    top + geo.row_h / 2.0 - anchor / 2.0
                } else {
                    top + geo.row_h
                };
                if y1 > y0 {
                    painter.vline(center_x, y0..=y1, Stroke::new(1.0_f32, colors.fold_marker));
                }
            }
            let ends_here = editor.folding.regions().iter().any(|region| {
                region.expanded
                    && editor.document.line_number(region.end) == line
                    && editor.document.line_number(region.start) < line
            });
            if ends_here {
                paint_anchor(
                    &painter,
                    pos2(center_x, top + geo.row_h / 2.0),
                    AnchorKind::End,
                    colors.fold_marker,
                );
            }
        }
        // Fold chevron.
        if let (Some((x0, x1)), Some(index)) = (layout.fold, region_at_line(editor, line)) {
            let region = &editor.folding.regions()[index];
            let collapsed = !region.expanded;
            let first_line = editor.document.line_number(region.start);
            let last_line = editor.document.line_number(region.end);
            let rect = row_rect(x0, x1);
            let resp = ui.interact(
                rect,
                id.with(("fold", region.start)),
                if enabled {
                    Sense::CLICK
                } else {
                    Sense::hover()
                },
            );
            let verb = if collapsed { "Unfold" } else { "Fold" };
            resp.widget_info(|| {
                WidgetInfo::labeled(
                    WidgetType::Button,
                    enabled,
                    format!("{verb} lines {} to {}", first_line + 1, last_line + 1),
                )
            });
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Default);
            }
            let alpha = if collapsed { 1.0 } else { st.chevron_alpha };
            // The chevron of the focused or hovered button stays visible.
            let alpha = if resp.has_focus() || resp.hovered() {
                1.0
            } else {
                alpha
            };
            if outline {
                paint_anchor(
                    &painter,
                    rect.center(),
                    if collapsed {
                        AnchorKind::Collapsed
                    } else {
                        AnchorKind::Expanded
                    },
                    colors.fold_marker,
                );
            } else if alpha > 0.0 {
                paint_chevron(
                    &painter,
                    rect.center(),
                    collapsed,
                    colors.fold_marker.gamma_multiply(alpha),
                );
            }
            if resp.clicked() {
                let recursive = ui.input(|i| i.modifiers.alt);
                editor.toggle_fold_region(index, recursive);
                out.toggled = true;
            }
        }
        row += 1;
    }
    let _ = tab;
    out
}

/// The 9 px fold chevron (spec §5): down for an expanded region, right for a collapsed one,
/// a 1 px stroke.
pub(crate) fn paint_chevron(painter: &Painter, center: Pos2, collapsed: bool, color: Color32) {
    let h = metrics::FOLD_ICON / 2.0;
    let points = if collapsed {
        // `M3 8 L6.5 4.5 L3 1` in the 9 px box, mirrored: points right.
        vec![
            pos2(center.x - h + 3.0, center.y - h + 1.0),
            pos2(center.x - h + 6.5, center.y),
            pos2(center.x - h + 3.0, center.y + h - 1.0),
        ]
    } else {
        // `M8 2.75 L4.5 6.25 L1 2.75`: points down.
        vec![
            pos2(center.x - h + 1.0, center.y - h + 2.75),
            pos2(center.x, center.y - h + 6.25),
            pos2(center.x + h - 1.0, center.y - h + 2.75),
        ]
    };
    painter.add(egui::Shape::line(points, Stroke::new(1.0_f32, color)));
}

/// Which classic folding anchor to draw.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AnchorKind {
    /// The start of an expanded region: a square with a minus.
    Expanded,
    /// A collapsed region: a square with a plus.
    Collapsed,
    /// The end of an expanded region: a square with a minus.
    End,
}

/// A classic folding outline anchor (spec: square +/- anchors, size
/// `min(4, lineHeight / 2 - 2) * 2`), a 1 px outline.
fn paint_anchor(painter: &Painter, center: Pos2, kind: AnchorKind, color: Color32) {
    let half = metrics::FOLD_ANCHOR / 2.0;
    let rect = Rect::from_center_size(center, vec2(2.0 * half, 2.0 * half));
    let stroke = Stroke::new(1.0_f32, color);
    painter.rect_stroke(rect.shrink(0.5), 0.0, stroke, egui::StrokeKind::Inside);
    let arm = half - 2.0;
    painter.hline((center.x - arm)..=(center.x + arm), center.y, stroke);
    if kind == AnchorKind::Collapsed {
        painter.vline(center.x, (center.y - arm)..=(center.y + arm), stroke);
    }
}
