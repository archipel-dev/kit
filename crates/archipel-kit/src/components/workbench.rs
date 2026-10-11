//! A shared docked islands layout, independent of application content and policy.

use super::{IconButton, Panel, Side, SplitPane, WindowChrome, WindowToolbar};
use crate::{Glyph, Theme, egui, tokens};
use egui::{Id, Response, Ui};
use std::cell::Cell;

/// A docked tool-window region that can occupy the entire work area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum WorkbenchRegion {
    /// The left tool window.
    Left,
    /// The right tool window.
    Right,
    /// The bottom tool window.
    Bottom,
}

/// Caller-owned layout state. Persist it in the host's settings if desired.
/// Ratios retain their preferred values when a smaller viewport clamps the layout.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorkbenchState {
    /// Left pane's share of the horizontal work area.
    pub left_ratio: f32,
    /// Center's share of the space remaining beside the right pane.
    pub center_ratio: f32,
    /// Main area's share of the vertical work area above the bottom pane.
    pub top_ratio: f32,
    /// Whether the left pane is hidden.
    pub left_hidden: bool,
    /// Whether the right pane is hidden.
    pub right_hidden: bool,
    /// Whether the bottom pane is hidden.
    pub bottom_hidden: bool,
    /// A maximized tool window, or the regular docked layout.
    pub maximized: Option<WorkbenchRegion>,
}

impl Default for WorkbenchState {
    fn default() -> Self {
        Self {
            left_ratio: 0.22,
            center_ratio: 0.75,
            top_ratio: 0.7,
            left_hidden: false,
            right_hidden: false,
            bottom_hidden: false,
            maximized: None,
        }
    }
}

type Content<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;
struct Region<'a> {
    title: String,
    content: Content<'a>,
}

/// A reusable window shell: optional toolbar and status bar, a center island, and
/// optional left, right and bottom tool windows with keyboard-operable splitters.
/// The caller supplies each region's content and owns all layout state.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// use archipel_kit::components::{Workbench, WorkbenchState};
/// let mut state = WorkbenchState::default();
/// Workbench::new("workspace", &mut state)
///     .left("Browser", |ui| { ui.label("Collections"); })
///     .bottom("Messages", |ui| { ui.label("Ready"); })
///     .show(ctx, |ui| { ui.label("Document"); });
/// # });
/// ```
#[must_use = "show the workbench with .show or .show_inside"]
pub struct Workbench<'a> {
    id: Id,
    state: &'a mut WorkbenchState,
    left: Option<Region<'a>>,
    right: Option<Region<'a>>,
    bottom: Option<Region<'a>>,
    toolbar: Option<(WindowChrome, Content<'a>)>,
    status: Option<Content<'a>>,
}

impl<'a> Workbench<'a> {
    /// Construct a shell with a stable identity and caller-owned layout state.
    pub fn new(id_salt: impl std::hash::Hash, state: &'a mut WorkbenchState) -> Self {
        Self {
            id: Id::new(id_salt),
            state,
            left: None,
            right: None,
            bottom: None,
            toolbar: None,
            status: None,
        }
    }

    /// A titled left tool window. Its header provides Hide and Maximize/Restore actions.
    pub fn left(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.left = Some(Region {
            title: title.into(),
            content: Box::new(content),
        });
        self
    }
    /// A titled right tool window.
    pub fn right(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.right = Some(Region {
            title: title.into(),
            content: Box::new(content),
        });
        self
    }
    /// A titled bottom tool window below the main area.
    pub fn bottom(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom = Some(Region {
            title: title.into(),
            content: Box::new(content),
        });
        self
    }
    /// Window toolbar; use the same chrome when configuring the host viewport.
    pub fn toolbar(mut self, chrome: WindowChrome, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.toolbar = Some((chrome, Box::new(content)));
        self
    }
    /// A status area on the window frame, typically containing `StatusBarItem`s.
    pub fn status(mut self, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.status = Some(Box::new(content));
        self
    }

    /// Render a whole window. Hidden or temporarily constrained region callbacks are
    /// not invoked. The host can restore hidden panes by changing the corresponding flag.
    pub fn show(mut self, ctx: &egui::Context, center: impl FnOnce(&mut Ui)) -> Response {
        if let Some((chrome, content)) = self.toolbar.take() {
            WindowToolbar::new(self.id.with("toolbar"))
                .chrome(chrome)
                .show(ctx, content);
        }
        if let Some(content) = self.status.take() {
            egui::TopBottomPanel::bottom(self.id.with("status"))
                .show_separator_line(false)
                .frame(crate::parts::Surface::Chrome.frame(ctx))
                .show(ctx, |ui| {
                    ui.horizontal(content);
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Theme::colors(ctx).frame)
                    .inner_margin(tokens::size::ISLAND_GAP as i8),
            )
            .show(ctx, |ui| self.show_inside(ui, center))
            .response
    }

    /// Render only the islands into a bounded area, for embedding or previews.
    /// Toolbar and status callbacks are used only by `show`.
    pub fn show_inside(self, ui: &mut Ui, center: impl FnOnce(&mut Ui)) -> Response {
        let Self {
            id,
            state,
            mut left,
            mut right,
            mut bottom,
            ..
        } = self;
        let hide = Cell::new(None);
        let maximize = Cell::new(None);
        let maximized = state.maximized;
        let rect = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(rect, 0, Theme::colors(ui.ctx()).frame);
        let response = ui
            .scope_builder(egui::UiBuilder::new().id_salt(id), |ui| {
                let maximized_region = match maximized {
                    Some(WorkbenchRegion::Left) if !state.left_hidden => left.take(),
                    Some(WorkbenchRegion::Right) if !state.right_hidden => right.take(),
                    Some(WorkbenchRegion::Bottom) if !state.bottom_hidden => bottom.take(),
                    _ => None,
                };
                if let Some(region) = maximized_region {
                    region.show(
                        ui,
                        maximized.unwrap_or(WorkbenchRegion::Left),
                        true,
                        &hide,
                        &maximize,
                    );
                    return;
                }
                // Missing/hidden maximized panes must never hide the center.
                state.maximized = None;
                let min = tokens::size::SPLIT_MIN_PANE;
                let center_min = tokens::size::WORKBENCH_CENTER_MIN;
                let gap = tokens::size::SPLIT_DIVIDER;
                let left_open = left.is_some() && !state.left_hidden;
                let right_open = right.is_some() && !state.right_hidden;
                let auto_right = right_open
                    && rect.width()
                        < center_min
                            + min * (1 + usize::from(left_open)) as f32
                            + gap * (1 + usize::from(left_open)) as f32;
                let auto_left = left_open && rect.width() < center_min + min + gap;
                let auto_bottom = bottom.is_some()
                    && !state.bottom_hidden
                    && rect.height() < center_min + min + gap;
                let mut left_hidden = state.left_hidden || auto_left;
                let mut right_hidden = state.right_hidden || auto_right;
                let mut bottom_hidden = state.bottom_hidden || auto_bottom;
                let main = |ui: &mut Ui| {
                    let middle = |ui: &mut Ui| {
                        if let Some(region) = right {
                            let split =
                                SplitPane::horizontal(id.with("right"), &mut state.center_ratio)
                                    .default_ratio(0.75)
                                    .min_sizes(center_min, min)
                                    .label("Resize right tool window")
                                    .collapsible(Side::Second, &mut right_hidden)
                                    .enabled(!auto_right)
                                    .show(
                                        ui,
                                        |ui| center_island(ui, center),
                                        |ui| {
                                            if ui.is_visible() {
                                                region.show(
                                                    ui,
                                                    WorkbenchRegion::Right,
                                                    false,
                                                    &hide,
                                                    &maximize,
                                                );
                                            }
                                        },
                                    );
                            if split.divider.changed() {
                                state.right_hidden = right_hidden;
                            }
                        } else {
                            center_island(ui, center);
                        }
                    };
                    if let Some(region) = left {
                        let split = SplitPane::horizontal(id.with("left"), &mut state.left_ratio)
                            .default_ratio(0.22)
                            .min_sizes(
                                min,
                                center_min
                                    + if right_open && !auto_right {
                                        min + gap
                                    } else {
                                        0.0
                                    },
                            )
                            .label("Resize left tool window")
                            .collapsible(Side::First, &mut left_hidden)
                            .enabled(!auto_left)
                            .show(
                                ui,
                                |ui| {
                                    if ui.is_visible() {
                                        region.show(
                                            ui,
                                            WorkbenchRegion::Left,
                                            false,
                                            &hide,
                                            &maximize,
                                        );
                                    }
                                },
                                middle,
                            );
                        if split.divider.changed() {
                            state.left_hidden = left_hidden;
                        }
                    } else {
                        middle(ui);
                    }
                };
                if let Some(region) = bottom {
                    let split = SplitPane::vertical(id.with("bottom"), &mut state.top_ratio)
                        .default_ratio(0.7)
                        .min_sizes(center_min, min)
                        .label("Resize bottom tool window")
                        .collapsible(Side::Second, &mut bottom_hidden)
                        .enabled(!auto_bottom)
                        .show(ui, main, |ui| {
                            if ui.is_visible() {
                                region.show(ui, WorkbenchRegion::Bottom, false, &hide, &maximize);
                            }
                        });
                    if split.divider.changed() {
                        state.bottom_hidden = bottom_hidden;
                    }
                } else {
                    main(ui);
                }
            })
            .response;
        if let Some(region) = hide.get() {
            match region {
                WorkbenchRegion::Left => state.left_hidden = true,
                WorkbenchRegion::Right => state.right_hidden = true,
                WorkbenchRegion::Bottom => state.bottom_hidden = true,
            }
            state.maximized = None;
        } else if let Some(region) = maximize.get() {
            state.maximized = if maximized == Some(region) {
                None
            } else {
                Some(region)
            };
        }
        response
    }
}

impl Region<'_> {
    fn show(
        self,
        ui: &mut Ui,
        region: WorkbenchRegion,
        maximized: bool,
        hide: &Cell<Option<WorkbenchRegion>>,
        maximize: &Cell<Option<WorkbenchRegion>>,
    ) {
        let label = format!(
            "{} {}",
            if maximized { "Restore" } else { "Maximize" },
            self.title
        );
        Panel::new(self.title)
            .on_hide(|| hide.set(Some(region)))
            .actions(|ui| {
                if ui.add(IconButton::new(Glyph::Fullscreen, label)).clicked() {
                    maximize.set(Some(region));
                }
            })
            .show(ui, |ui| {
                ui.set_min_height((ui.available_height() - tokens::size::SECTION_SPACING).max(0.0));
                crate::parts::desktop(egui::ScrollArea::both())
                    .auto_shrink([false, false])
                    .show(ui, self.content);
            });
    }
}

fn center_island(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    if !ui.is_visible() {
        return;
    }
    crate::parts::Surface::Island
        .frame(ui.ctx())
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            content(ui);
        });
}
