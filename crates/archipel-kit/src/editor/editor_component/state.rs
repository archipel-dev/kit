//! What the component keeps in the egui context between frames, and the geometry of one frame.

use super::cache::DocCache;
use super::layout::LineLayout;
use super::rows::RowMap;
use crate::editor::model::{CaretSnapshot, Document, Editor, FoldRegion};
use egui::{Pos2, Rect, Vec2};
use std::sync::Arc;

/// An animated jump of the viewport (spec §4.1).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollAnim {
    pub from: Vec2,
    pub to: Vec2,
    pub start: f64,
    pub duration: f64,
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
    /// A rectangular selection (Alt+drag, middle-drag).
    Column,
    /// Dragging in the line number column: selects lines. Starts after the drag sensitivity.
    GutterLines {
        anchor_line: usize,
        started: bool,
        press: Pos2,
    },
    /// Dragging from another part of the gutter: selects from the start of the line.
    GutterChar {
        anchor: usize,
        started: bool,
        press: Pos2,
    },
    /// A press inside the selection: it collapses on release without a drag; a drag starts a
    /// new selection at the press.
    InSelection { offset: usize, press: Pos2 },
    /// A press on a collapsed fold placeholder: it expands on release without a drag.
    OnFold { region_start: usize, press: Pos2 },
}

/// The last unit of a multi-click, for Shift+click and drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unit {
    Word(usize, usize),
    Line(usize, usize),
}

/// The pointer resting on the text.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rest {
    pub offset: usize,
    pub since: f64,
    pub pos: Pos2,
}

#[derive(Clone, Default)]
pub(crate) struct ViewState {
    /// The scroll offset after the last frame.
    pub offset: Vec2,
    pub anim: Option<ScrollAnim>,
    /// Time of the last caret or text activity, for the blink.
    pub last_activity: f64,
    pub last_stamp: u64,
    pub last_carets: CaretSnapshot,
    pub drag: Option<Drag>,
    pub unit: Option<Unit>,
    pub click_time: f64,
    pub click_pos: Pos2,
    pub click_count: u32,
    pub autoscroll_last: f64,
    pub autoscroll_counter: u32,
    pub rest: Option<Rest>,
    pub fold_rest: Option<(usize, f64)>,
    /// The fold chevrons' opacity and the time the pointer left the gutter.
    pub chevron_alpha: f32,
    pub chevron_time: f64,
    pub gutter_left_at: Option<f64>,
    /// The bracket fold regions were computed for this cache.
    pub folds_key: usize,
    /// The text last selected with the mouse: the X11 primary selection of Linux.
    pub primary: String,
    pub preedit: Option<String>,
    /// Whether the viewport was at the end last frame (stick to bottom).
    pub focused_last: bool,
    /// A click on the error stripe asked to center the caret on the next frame.
    pub pending_center: bool,
    /// Where the find bar was last frame.
    pub overlay: Option<Rect>,
    /// The text or the find field, whichever had the focus last.
    pub last_focus: Option<egui::Id>,
    /// Frames left in which to give the focus back after a press on the find bar.
    pub restore_focus: u8,
}

impl ViewState {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Self {
        ctx.data(|d| d.get_temp(id.with("archipel-editor-state")))
            .unwrap_or_default()
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_temp(id.with("archipel-editor-state"), self));
    }
}

/// The document-derived data of one frame.
pub(crate) struct Frame {
    pub cache: Arc<DocCache>,
    /// Collapsed regions that are not inside another collapsed region.
    pub outer: Vec<FoldRegion>,
    pub map: RowMap,
}

impl Frame {
    pub fn layout<'a>(&self, doc: &'a Document, visual: usize, tab: usize) -> LineLayout<'a> {
        LineLayout::build(doc, &self.outer, &self.map, visual, tab)
    }

    /// The visual line an offset is drawn on (offsets hidden in a fold go to its placeholder).
    pub fn visual_of(&self, editor: &Editor, offset: usize) -> usize {
        let offset = match editor.folding.collapsed_region_hiding(offset) {
            Some(region) => region.start,
            None => offset,
        };
        self.map
            .logical_to_visual(editor.document.line_number(offset))
    }
}

/// Where things are on screen in one frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Geo {
    /// The text viewport.
    pub area: Rect,
    /// The scroll offset.
    pub offset: Vec2,
    pub cell: f32,
    pub row_h: f32,
    pub pad: f32,
    /// Screen y of the top of row 0 and screen x of column 0.
    pub y0: f32,
    pub x0: f32,
}

impl Geo {
    pub fn new(area: Rect, offset: Vec2, cell: f32, row_h: f32, pad: f32, ppp: f32) -> Self {
        let top_pad = pad / 2.0;
        // Rows sit on whole pixels so adjacent selection rows leave no seam.
        let y0 = ((area.min.y - offset.y + top_pad) * ppp).round() / ppp;
        Geo {
            area,
            offset,
            cell,
            row_h,
            pad,
            y0,
            x0: area.min.x - offset.x + pad,
        }
    }

    pub fn row_top(&self, row: usize) -> f32 {
        self.y0 + row as f32 * self.row_h
    }

    /// The row under screen y (negative above the first).
    pub fn row_at(&self, y: f32) -> isize {
        ((y - self.y0) / self.row_h).floor() as isize
    }

    /// Screen x of column `col` of a row whose first column is `base`.
    pub fn x_of(&self, col: usize, base: usize) -> f32 {
        self.x0 + (col as f32 - base as f32) * self.cell
    }

    /// The column boundary nearest screen x (fractional columns rounded), relative to the
    /// row's first column.
    pub fn col_at(&self, x: f32) -> isize {
        ((x - self.x0) / self.cell).round() as isize
    }

    pub fn first_visible_row(&self) -> usize {
        self.row_at(self.area.min.y).max(0) as usize
    }

    pub fn last_visible_row(&self, total_rows: usize) -> usize {
        (self.row_at(self.area.max.y).max(0) as usize).min(total_rows.saturating_sub(1))
    }
}
