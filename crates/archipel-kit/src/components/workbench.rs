//! The one islands layout every Archipel app uses.

use super::{Orientation, Panel, Side, SplitPane};
use crate::parts::Surface;
use crate::tokens::size;
use egui::{Align, Id, Layout, Margin, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};

/// Where a tool window is docked in a [`Workbench`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Anchor {
    /// The tool window left of the center.
    Left,
    /// The tool window right of the center.
    Right,
    /// The tool window below the center.
    Bottom,
}

/// What the user left of a [`Workbench`]: how big its tool windows are and which are
/// collapsed. The caller owns it (so it can be saved in the app's settings) and the
/// workbench only writes it when the user resizes, collapses or expands a tool window.
///
/// Sizes are in points and survive window resizes: the center island takes the slack. With
/// the `serde` feature the state is `Serialize` and `Deserialize`; fields missing from an
/// older settings file get their defaults.
///
/// Build it with [`Default`] and change the fields; new fields may be added in minor
/// versions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct WorkbenchState {
    /// Width of the left tool window.
    pub left: f32,
    /// Width of the right tool window.
    pub right: f32,
    /// Height of the bottom tool window.
    pub bottom: f32,
    /// Whether the left tool window is collapsed (the resize edge stays).
    pub left_collapsed: bool,
    /// Whether the right tool window is collapsed.
    pub right_collapsed: bool,
    /// Whether the bottom tool window is collapsed.
    pub bottom_collapsed: bool,
}

impl Default for WorkbenchState {
    fn default() -> Self {
        Self {
            left: size::TOOL_WINDOW_DEFAULT,
            right: size::TOOL_WINDOW_DEFAULT,
            bottom: size::TOOL_WINDOW_DEFAULT_HEIGHT,
            left_collapsed: false,
            right_collapsed: false,
            bottom_collapsed: false,
        }
    }
}

impl WorkbenchState {
    /// Whether the tool window at `anchor` is collapsed.
    pub fn is_collapsed(&self, anchor: Anchor) -> bool {
        match anchor {
            Anchor::Left => self.left_collapsed,
            Anchor::Right => self.right_collapsed,
            Anchor::Bottom => self.bottom_collapsed,
        }
    }

    /// Collapses or expands the tool window at `anchor`; its size is kept.
    pub fn set_collapsed(&mut self, anchor: Anchor, collapsed: bool) {
        match anchor {
            Anchor::Left => self.left_collapsed = collapsed,
            Anchor::Right => self.right_collapsed = collapsed,
            Anchor::Bottom => self.bottom_collapsed = collapsed,
        }
    }

    /// Collapses an expanded tool window and expands a collapsed one (a toolbar button,
    /// a shortcut).
    pub fn toggle(&mut self, anchor: Anchor) {
        self.set_collapsed(anchor, !self.is_collapsed(anchor));
    }

    fn default_size(anchor: Anchor) -> f32 {
        let default = Self::default();
        match anchor {
            Anchor::Left => default.left,
            Anchor::Right => default.right,
            Anchor::Bottom => default.bottom,
        }
    }

    fn size(&self, anchor: Anchor) -> f32 {
        match anchor {
            Anchor::Left => self.left,
            Anchor::Right => self.right,
            Anchor::Bottom => self.bottom,
        }
    }

    fn size_mut(&mut self, anchor: Anchor) -> &mut f32 {
        match anchor {
            Anchor::Left => &mut self.left,
            Anchor::Right => &mut self.right,
            Anchor::Bottom => &mut self.bottom,
        }
    }
}

/// What a [`Workbench`] drew: what the center returned, and what the user did this frame.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WorkbenchResponse<R> {
    /// What the center closure returned.
    pub inner: R,
    /// The tool windows whose hide button was clicked. The workbench has already collapsed
    /// them in the state.
    pub hidden: Vec<Anchor>,
    /// The tool windows the user resized, collapsed or expanded by their divider.
    pub resized: Vec<Anchor>,
    /// The divider of every tool window that is shown, for behaviours (tooltips, context
    /// menus) and tests.
    pub dividers: Vec<(Anchor, Response)>,
}

impl<R> WorkbenchResponse<R> {
    /// Whether the user clicked the hide button of the tool window at `anchor`.
    pub fn hide_clicked(&self, anchor: Anchor) -> bool {
        self.hidden.contains(&anchor)
    }

    /// Whether the user moved, collapsed or expanded the divider of `anchor`.
    pub fn resized(&self, anchor: Anchor) -> bool {
        self.resized.contains(&anchor)
    }

    /// Whether the [`WorkbenchState`] changed: save it.
    pub fn state_changed(&self) -> bool {
        !self.hidden.is_empty() || !self.resized.is_empty()
    }
}

type Body<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

struct ToolWindow<'a> {
    title: String,
    body: Body<'a>,
    actions: Option<Body<'a>>,
}

/// The window layout of every Archipel app, in the kit's islands look: the frame color
/// behind everything, and islands with the island radius and [`size::ISLAND_GAP`] between
/// them and around the edge.
///
/// - an optional [`toolbar`](Self::toolbar) at the top and [`status_bar`](Self::status_bar)
///   at the bottom, on the frame;
/// - [`left`](Self::left), [`right`](Self::right) and [`bottom`](Self::bottom) tool windows,
///   each a [`Panel`] with a title bar, [`actions`](Self::actions) and a hide button;
/// - the center island the closure of [`show`](Self::show) fills.
///
/// Every divider is a collapsible [`SplitPane`]. The caller owns a [`WorkbenchState`] with
/// the sizes and collapsed flags; the workbench fills the space it is given.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{StatusBarItem, Workbench, WorkbenchState};
///
/// let mut state = WorkbenchState::default();
/// let out = Workbench::new("window", &mut state)
///     .toolbar(|ui| { ui.label("my-project"); })
///     .left("Project", |ui| { ui.label("src/"); })
///     .bottom("Terminal", |ui| { ui.label("$ cargo run"); })
///     .status_bar(|ui| { ui.add(StatusBarItem::new("main")); })
///     .show(ui, |ui| { ui.label("fn main() {}"); });
/// if out.state_changed() {
///     // save `state` in the settings
/// }
/// # });
/// ```
#[must_use = "call `.show(ui, center)`"]
pub struct Workbench<'a> {
    id: Id,
    state: &'a mut WorkbenchState,
    enabled: bool,
    toolbar: Option<Body<'a>>,
    status_bar: Option<Body<'a>>,
    left: Option<ToolWindow<'a>>,
    right: Option<ToolWindow<'a>>,
    bottom: Option<ToolWindow<'a>>,
}

impl<'a> Workbench<'a> {
    /// A workbench whose sizes and collapsed flags live in `state`. `id_salt` tells
    /// workbenches apart (give each in a `Ui` its own).
    pub fn new(id_salt: impl std::hash::Hash, state: &'a mut WorkbenchState) -> Self {
        Self {
            id: Id::new(id_salt),
            state,
            enabled: true,
            toolbar: None,
            status_bar: None,
            left: None,
            right: None,
            bottom: None,
        }
    }

    /// A locked layout (`false`) has dividers that ignore the pointer and the keyboard.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The main toolbar: a row on the frame above the islands, laid out left to right.
    pub fn toolbar(mut self, toolbar: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.toolbar = Some(Box::new(toolbar));
        self
    }

    /// The status bar: a row on the frame below the islands, left to right. Fill it with
    /// [`StatusBarItem`](super::StatusBarItem)s.
    pub fn status_bar(mut self, status_bar: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.status_bar = Some(Box::new(status_bar));
        self
    }

    /// A tool window left of the center, titled `title`.
    pub fn left(self, title: impl Into<String>, body: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.dock(Anchor::Left, title, body)
    }

    /// A tool window right of the center, titled `title`.
    pub fn right(self, title: impl Into<String>, body: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.dock(Anchor::Right, title, body)
    }

    /// A tool window below the center, titled `title`.
    pub fn bottom(self, title: impl Into<String>, body: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.dock(Anchor::Bottom, title, body)
    }

    /// Toolbar buttons in the title bar of the tool window at `anchor`, as
    /// [`Panel::actions`] (right to left). Call it after the tool window is added.
    pub fn actions(mut self, anchor: Anchor, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        if let Some(tool) = self.tool_mut(anchor) {
            tool.actions = Some(Box::new(actions));
        }
        self
    }

    fn dock(
        mut self,
        anchor: Anchor,
        title: impl Into<String>,
        body: impl FnOnce(&mut Ui) + 'a,
    ) -> Self {
        let tool = ToolWindow {
            title: title.into(),
            body: Box::new(body),
            actions: None,
        };
        *self.slot(anchor) = Some(tool);
        self
    }

    fn slot(&mut self, anchor: Anchor) -> &mut Option<ToolWindow<'a>> {
        match anchor {
            Anchor::Left => &mut self.left,
            Anchor::Right => &mut self.right,
            Anchor::Bottom => &mut self.bottom,
        }
    }

    fn tool_mut(&mut self, anchor: Anchor) -> Option<&mut ToolWindow<'a>> {
        self.slot(anchor).as_mut()
    }

    /// Draws the workbench into all the space `ui` has, with `center` in the center island.
    pub fn show<R>(
        mut self,
        ui: &mut Ui,
        center: impl FnOnce(&mut Ui) -> R,
    ) -> WorkbenchResponse<R> {
        let finite = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
        let avail = ui.available_size_before_wrap();
        let (rect, _) =
            ui.allocate_exact_size(vec2(finite(avail.x), finite(avail.y)), Sense::hover());
        let mut window = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(rect));
        let frame = Surface::Chrome.frame(ui.ctx());

        let mut left = Dock::new(self.left.take(), Anchor::Left, self.state, self.enabled);
        let mut right = Dock::new(self.right.take(), Anchor::Right, self.state, self.enabled);
        let mut bottom = Dock::new(self.bottom.take(), Anchor::Bottom, self.state, self.enabled);
        let (toolbar, status_bar, id) = (self.toolbar.take(), self.status_bar.take(), self.id);

        let mut inner_out = None;
        frame.show(&mut window, |ui| {
            ui.set_min_size(ui.available_size());
            let gap = size::ISLAND_GAP;
            let inner = ui.max_rect();
            let mut body = inner;
            if let Some(toolbar) = toolbar {
                let bar = Rect::from_min_size(inner.min, vec2(inner.width(), size::ICON_BUTTON));
                bar_row(ui, bar, toolbar);
                body.min.y = (bar.bottom() + gap).min(inner.bottom());
            }
            if let Some(status) = status_bar {
                let top = (inner.bottom() - size::CONTROL_HEIGHT_SMALL).max(body.top());
                let bar = Rect::from_min_max(pos2(inner.left(), top), inner.max);
                bar_row(ui, bar, status);
                body.max.y = (bar.top() - gap).max(body.top());
            }
            let mut islands = ui.new_child(UiBuilder::new().max_rect(body));
            // Left and right run the full height; the bottom sits under the center.
            let rest_min = size::CENTER_MIN
                + if right.tool.is_some() && !right.collapsed {
                    size::SPLIT_DIVIDER + size::TOOL_WINDOW_MIN
                } else {
                    0.0
                };
            inner_out = Some(dock_split(
                &mut islands,
                id,
                Anchor::Left,
                &mut left,
                rest_min,
                |ui| {
                    dock_split(ui, id, Anchor::Right, &mut right, size::CENTER_MIN, |ui| {
                        dock_split(
                            ui,
                            id,
                            Anchor::Bottom,
                            &mut bottom,
                            size::CENTER_MIN_HEIGHT,
                            |ui| center_island(ui, center),
                        )
                    })
                },
            ));
        });

        let mut response = WorkbenchResponse {
            inner: inner_out.expect("the center island is drawn exactly once"),
            hidden: Vec::new(),
            resized: Vec::new(),
            dividers: Vec::new(),
        };
        for (anchor, mut dock) in [
            (Anchor::Left, left),
            (Anchor::Right, right),
            (Anchor::Bottom, bottom),
        ] {
            if dock.hidden {
                response.hidden.push(anchor);
            }
            if dock.resized {
                response.resized.push(anchor);
            }
            if let Some(divider) = dock.divider.take() {
                response.dividers.push((anchor, divider));
            }
            dock.write_back(anchor, self.state);
        }
        if response.state_changed() {
            ui.ctx().request_repaint();
        }
        response
    }
}

/// A tool window's place: the state it was drawn with, and what the user did to it.
struct Dock<'a> {
    tool: Option<ToolWindow<'a>>,
    size: f32,
    collapsed: bool,
    hidden: bool,
    resized: bool,
    enabled: bool,
    divider: Option<Response>,
}

impl<'a> Dock<'a> {
    fn new(
        tool: Option<ToolWindow<'a>>,
        anchor: Anchor,
        state: &WorkbenchState,
        enabled: bool,
    ) -> Self {
        let size = state.size(anchor);
        Self {
            tool,
            // A garbage size (NaN from a hand-edited settings file) falls back.
            size: if size.is_finite() {
                size.max(0.0)
            } else {
                WorkbenchState::default_size(anchor)
            },
            collapsed: state.is_collapsed(anchor),
            hidden: false,
            resized: false,
            enabled,
            divider: None,
        }
    }

    fn write_back(&self, anchor: Anchor, state: &mut WorkbenchState) {
        *state.size_mut(anchor) = self.size;
        state.set_collapsed(anchor, self.collapsed || self.hidden);
    }
}

/// `tool` and the `rest` of the workbench, either side of one collapsible [`SplitPane`]
/// (just `rest` without a tool window). The tool window's size is kept in points, so a
/// resized window gives or takes the space from `rest`, which `rest_min` protects.
fn dock_split<R>(
    ui: &mut Ui,
    id: Id,
    anchor: Anchor,
    dock: &mut Dock<'_>,
    rest_min: f32,
    rest: impl FnOnce(&mut Ui) -> R,
) -> R {
    let Some(tool) = dock.tool.take() else {
        return rest(ui);
    };
    let horizontal = anchor != Anchor::Bottom;
    let tool_first = anchor == Anchor::Left;
    let avail = ui.available_size_before_wrap();
    let total = (if horizontal { avail.x } else { avail.y } - size::SPLIT_DIVIDER).max(1.0);
    // The split's ratio is the first pane's share.
    let share = |points: f32| {
        let tool_share = (points / total).clamp(0.0, 1.0);
        if tool_first {
            tool_share
        } else {
            1.0 - tool_share
        }
    };
    let mut ratio = share(dock.size);
    let before = ratio;
    let (tool_min, side) = if horizontal {
        (
            size::TOOL_WINDOW_MIN,
            if tool_first {
                Side::First
            } else {
                Side::Second
            },
        )
    } else {
        (size::TOOL_WINDOW_MIN_HEIGHT, Side::Second)
    };
    let mins = if tool_first {
        (tool_min, rest_min)
    } else {
        (rest_min, tool_min)
    };
    let split = SplitPane::new(
        (id, anchor),
        if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        },
        &mut ratio,
    )
    .default_ratio(share(WorkbenchState::default_size(anchor)))
    .min_sizes(mins.0, mins.1)
    .label(format!("Resize {}", tool.title))
    .enabled(dock.enabled)
    .collapsible(side, &mut dock.collapsed);

    let hidden = &mut dock.hidden;
    let tool_ui = |ui: &mut Ui| *hidden = tool_window(ui, tool);
    let (out, divider) = if tool_first {
        let r = split.show(ui, tool_ui, rest);
        (r.second, r.divider)
    } else {
        let r = split.show(ui, rest, tool_ui);
        (r.first, r.divider)
    };
    if divider.changed() {
        dock.resized = true;
    }
    dock.divider = Some(divider);
    if (ratio - before).abs() > 1e-4 {
        let tool_share = if tool_first { ratio } else { 1.0 - ratio };
        dock.size = tool_share * total;
    }
    out
}

/// A tool window island: a [`Panel`] with the title, actions and hide button, filling its
/// pane. Returns whether the hide button was clicked.
fn tool_window(ui: &mut Ui, tool: ToolWindow<'_>) -> bool {
    let mut hide = false;
    let mut panel = Panel::new(tool.title.as_str()).on_hide(|| hide = true);
    if let Some(actions) = tool.actions {
        panel = panel.actions(actions);
    }
    let body = tool.body;
    panel.show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        body(ui);
    });
    hide
}

/// The center: an island with no padding, so editors and canvases run to its edge.
fn center_island<R>(ui: &mut Ui, center: impl FnOnce(&mut Ui) -> R) -> R {
    Surface::Island
        .frame(ui.ctx())
        .inner_margin(Margin::ZERO)
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
            center(ui)
        })
        .inner
}

/// A bar on the frame (toolbar, status bar): `content` laid out left to right.
fn bar_row(ui: &mut Ui, rect: Rect, content: Body<'_>) {
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    row.spacing_mut().item_spacing.x = size::TOOLBAR_GAP;
    row.set_clip_rect(rect.intersect(ui.clip_rect()));
    content(&mut row);
}
