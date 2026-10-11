//! A shared docked islands layout, independent of application content and policy.

use super::{
    IconButton, Panel, Side, SplitPane, StripeButton, ToolStripe, Tooltip, WindowChrome,
    WindowToolbar,
};
use crate::Icon;
use crate::behaviour::WidgetExt;
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
    padded: bool,
    icon: Option<Icon>,
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
    project_color: Option<egui::Color32>,
    glow_anchor: Option<egui::Rect>,
    center_padded: bool,
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
            project_color: None,
            glow_anchor: None,
            center_padded: true,
        }
    }

    /// A titled left tool window. Its header provides Hide and Maximize/Restore actions.
    pub fn left(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.left = Some(Region::new(title, content));
        self
    }
    /// A titled right tool window.
    pub fn right(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.right = Some(Region::new(title, content));
        self
    }
    /// A titled bottom tool window below the main area.
    pub fn bottom(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom = Some(Region::new(title, content));
        self
    }
    /// Gives `region`'s body the panel content inset (default: edge to edge, for trees,
    /// lists and consoles whose rows bring their own inset). Use it for forms and text.
    /// Call it after adding the region.
    pub fn padded(mut self, region: WorkbenchRegion, padded: bool) -> Self {
        let slot = match region {
            WorkbenchRegion::Left => &mut self.left,
            WorkbenchRegion::Right => &mut self.right,
            WorkbenchRegion::Bottom => &mut self.bottom,
        };
        if let Some(slot) = slot {
            slot.padded = padded;
        }
        self
    }

    /// Gives `region` a button with `icon` on a tool window stripe beside the islands, as in
    /// Android Studio: the left stripe holds the left (top group) and bottom (bottom group)
    /// tool windows, the right stripe the right one. The button is filled while the tool
    /// window is open and opens or closes it. Call it after adding the region.
    pub fn icon(mut self, region: WorkbenchRegion, icon: impl Into<Icon>) -> Self {
        let slot = match region {
            WorkbenchRegion::Left => &mut self.left,
            WorkbenchRegion::Right => &mut self.right,
            WorkbenchRegion::Bottom => &mut self.bottom,
        };
        if let Some(slot) = slot {
            slot.icon = Some(icon.into());
        }
        self
    }

    /// Gives the center island the content inset (default `true`). Pass `false` for an
    /// editor: its tab bar sits at the island's top and its text runs to the edges.
    pub fn center_padded(mut self, padded: bool) -> Self {
        self.center_padded = padded;
        self
    }

    /// The project's color: a soft glow of it on the frame behind the toolbar's project
    /// widget and over the top of the stripes, as in Android Studio's Islands UI.
    pub fn project_color(mut self, color: egui::Color32) -> Self {
        self.project_color = Some(color);
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
            let mut toolbar = WindowToolbar::new(self.id.with("toolbar")).chrome(chrome);
            if let Some(color) = self.project_color {
                toolbar = toolbar.project_color(color);
            }
            toolbar.show(ctx, content);
        }
        // The glow continues from the toolbar: anchor it to the window, not the islands.
        self.glow_anchor = Some(ctx.input(|i| i.viewport_rect()));
        if let Some(content) = self.status.take() {
            // Android Studio's status bar: 30 pt on the frame, items 21 pt high.
            egui::TopBottomPanel::bottom(self.id.with("status"))
                .show_separator_line(false)
                .exact_height(tokens::size::STATUS_BAR_HEIGHT)
                .frame(
                    egui::Frame::new()
                        .fill(Theme::colors(ctx).frame)
                        .inner_margin(egui::Margin::symmetric(
                            tokens::size::STATUS_BAR_INSET as i8,
                            0,
                        )),
                )
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        content(ui);
                    });
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
            project_color,
            glow_anchor,
            center_padded,
            ..
        } = self;
        let hide = Cell::new(None);
        let maximize = Cell::new(None);
        let full = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(full, 0, Theme::colors(ui.ctx()).frame);
        if let Some(color) = project_color {
            crate::parts::project_glow(ui.painter(), glow_anchor.unwrap_or(full), color);
        }
        // Tool window stripes on the frame, beside the islands.
        let mut rect = full;
        let stripe_width = tokens::stripe::WIDTH - tokens::size::ISLAND_GAP;
        let button = |region: &Option<Region<'_>>, hidden: bool| {
            region.as_ref().and_then(|r| {
                r.icon
                    .map(|icon| StripeButton::new(icon, r.title.clone()).open(!hidden))
            })
        };
        let left_top = button(&left, state.left_hidden);
        let left_bottom = button(&bottom, state.bottom_hidden);
        let right_top = button(&right, state.right_hidden);
        let mut toggled = None;
        if left_top.is_some() || left_bottom.is_some() {
            let stripe = egui::Rect::from_min_max(
                full.min,
                egui::pos2(full.left() + stripe_width, full.bottom()),
            );
            rect.min.x = stripe.right() + tokens::size::ISLAND_GAP;
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(stripe));
            let has_top = left_top.is_some();
            let mut bar = ToolStripe::new(id.with("left-stripe"));
            if let Some(b) = left_top {
                bar = bar.top(b);
            }
            if let Some(b) = left_bottom {
                bar = bar.bottom(b);
            }
            toggled = match bar.show(&mut column) {
                Some(0) if has_top => Some(WorkbenchRegion::Left),
                Some(_) => Some(WorkbenchRegion::Bottom),
                None => None,
            };
        }
        if let Some(b) = right_top {
            let stripe = egui::Rect::from_min_max(
                egui::pos2(full.right() - stripe_width, full.top()),
                full.max,
            );
            rect.max.x = stripe.left() - tokens::size::ISLAND_GAP;
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(stripe));
            if ToolStripe::new(id.with("right-stripe"))
                .top(b)
                .show(&mut column)
                .is_some()
            {
                toggled = Some(WorkbenchRegion::Right);
            }
        }
        if let Some(region) = toggled {
            let hidden = match region {
                WorkbenchRegion::Left => &mut state.left_hidden,
                WorkbenchRegion::Right => &mut state.right_hidden,
                WorkbenchRegion::Bottom => &mut state.bottom_hidden,
            };
            *hidden = !*hidden;
            if *hidden && state.maximized == Some(region) {
                state.maximized = None;
            }
        }
        let maximized = state.maximized;
        let response = ui
            .scope_builder(egui::UiBuilder::new().id_salt(id).max_rect(rect), |ui| {
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
                                    .line(false)
                                    .collapsible(Side::Second, &mut right_hidden)
                                    .enabled(!auto_right)
                                    .show(
                                        ui,
                                        |ui| center_island(ui, center_padded, center),
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
                            center_island(ui, center_padded, center);
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
                            .line(false)
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
                        .line(false)
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

impl<'a> Region<'a> {
    fn new(title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            title: title.into(),
            content: Box::new(content),
            padded: false,
            icon: None,
        }
    }

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
        // As in the IntelliJ Islands UI, the header actions fade in while the pointer is
        // anywhere over the tool window (or focus is inside it) and fade out when it leaves.
        Panel::new(self.title)
            .hover_actions(true)
            .padded(self.padded)
            .on_hide(|| hide.set(Some(region)))
            .actions(|ui| {
                let button = IconButton::new(Glyph::Fullscreen, label)
                    .with(Tooltip::new(if maximized { "Restore" } else { "Maximize" }));
                if ui.add(button).clicked() {
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

fn center_island(ui: &mut Ui, padded: bool, content: impl FnOnce(&mut Ui)) {
    if !ui.is_visible() {
        return;
    }
    let mut frame = crate::parts::Surface::Island.frame(ui.ctx());
    if !padded {
        frame = frame.inner_margin(0);
    }
    frame.show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        content(ui);
    });
}
