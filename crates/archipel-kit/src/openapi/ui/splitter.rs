//! Two panes with a draggable divider between them.

use crate::behaviour::{Behaviour, Behaviours};
use crate::cursor;
use crate::parts::focus_ring;
use crate::theme::Theme;
use crate::tokens::{radius, size};
use egui::emath::GuiRounding;
use egui::{
    Align, CornerRadius, CursorIcon, EventFilter, Id, Key, Modifiers, Rect, Response, Sense, Ui,
    UiBuilder, WidgetInfo, pos2, vec2,
};
use std::hash::Hash;

/// How a [`Splitter`] arranges its two panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Orientation {
    /// Side by side: left and right panes, a vertical divider.
    Horizontal,
    /// Stacked: top and bottom panes, a horizontal divider.
    Vertical,
}

/// One side of a [`Splitter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Side {
    /// The left pane (side by side) or the top pane (stacked).
    First,
    /// The right pane (side by side) or the bottom pane (stacked).
    Second,
}

/// Two panes separated by a divider the user drags (or moves with the arrow keys) to
/// resize them. It fills the space it is given; nest one in a pane for three or more.
///
/// The caller owns the split: `ratio` is the share of the space (without the divider) that
/// the first pane gets, from 0 to 1. The component is stateless and only writes `ratio`
/// when the user moves the divider, so persisting it is the caller's job. Each pane keeps
/// at least its minimum size; when the window shrinks the layout is clamped but `ratio` is
/// left alone, so the split springs back when the window grows again.
///
/// One side can be made [collapsible](Splitter::collapsible): the caller owns a `bool`
/// that says whether it is collapsed to nothing, and the divider stays as a thin edge to
/// drag it back out. The `ratio` is not touched while collapsed, so expanding restores it.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::openapi::ui::Splitter;
///
/// let mut ratio = 0.3;
/// Splitter::horizontal("workbench", &mut ratio)
///     .default_ratio(0.3)
///     .min_size(160.0)
///     .label("Resize sidebar")
///     .show(
///         ui,
///         |sidebar| { sidebar.label("Collections"); },
///         |main| { main.label("Request"); },
///     );
/// # });
/// ```
#[must_use = "call `.show(ui, first, second)`"]
pub struct Splitter<'a> {
    id: Id,
    orientation: Orientation,
    ratio: &'a mut f32,
    default_ratio: f32,
    min_first: f32,
    min_second: f32,
    label: String,
    enabled: bool,
    line: bool,
    collapse: Option<Collapse<'a>>,
    behaviours: Behaviours<'a>,
}

/// A collapsible side and the caller's collapsed flag.
struct Collapse<'a> {
    side: Side,
    collapsed: &'a mut bool,
}

/// What [`Splitter::show`] drew: what the panes returned and the divider's response.
pub struct SplitterResponse<A, B> {
    /// What the first pane's closure returned.
    pub first: A,
    /// What the second pane's closure returned.
    pub second: B,
    /// The divider. `changed()` when the user moved it or collapsed or expanded a pane this
    /// frame, `dragged()` while the pointer drags it, `double_clicked()` when it was reset.
    pub divider: Response,
}

impl<'a> Splitter<'a> {
    /// A split of `orientation`. `id_salt` tells splits apart: give every split in a `Ui`
    /// its own.
    pub fn new(id_salt: impl Hash, orientation: Orientation, ratio: &'a mut f32) -> Self {
        Self {
            id: Id::new(id_salt),
            orientation,
            ratio,
            default_ratio: 0.5,
            min_first: size::SPLIT_MIN_PANE,
            min_second: size::SPLIT_MIN_PANE,
            label: "Resize panes".into(),
            enabled: true,
            line: true,
            collapse: None,
            behaviours: Behaviours::default(),
        }
    }

    /// Side by side: the divider is vertical, `ratio` is the left pane's share.
    pub fn horizontal(id_salt: impl Hash, ratio: &'a mut f32) -> Self {
        Self::new(id_salt, Orientation::Horizontal, ratio)
    }

    /// Stacked: the divider is horizontal, `ratio` is the top pane's share.
    pub fn vertical(id_salt: impl Hash, ratio: &'a mut f32) -> Self {
        Self::new(id_salt, Orientation::Vertical, ratio)
    }

    /// The share a double click on the divider resets to (default 0.5).
    pub fn default_ratio(mut self, ratio: f32) -> Self {
        self.default_ratio = ratio;
        self
    }

    /// The smallest size, in points, of each pane (default 120).
    pub fn min_size(self, points: f32) -> Self {
        self.min_sizes(points, points)
    }

    /// The smallest sizes, in points, of the first and the second pane.
    pub fn min_sizes(mut self, first: f32, second: f32) -> Self {
        self.min_first = first.max(0.0);
        self.min_second = second.max(0.0);
        self
    }

    /// The divider's accessible name (default "Resize panes"): say what it resizes, like
    /// "Resize sidebar".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Lets `side` collapse to nothing. `collapsed` is the caller's state: set it to collapse
    /// or expand from code (a toolbar button, a command); the user changes it by dragging the
    /// divider further than [`size::SPLIT_COLLAPSE`] past that pane's minimum size, or with
    /// <kbd>Enter</kbd> or <kbd>Space</kbd> on the focused divider. Collapsed, the pane's
    /// closure still runs (hidden, so keep it cheap) and the divider stays at the edge;
    /// dragging it out, double-clicking it or pressing the arrow key toward the content
    /// expands the pane. A double click on an open pane still resets the ratio to the
    /// default; on a collapsed one it only expands. The ratio is never written while
    /// collapsed, and a collapse by dragging puts back the ratio from before the drag, so
    /// expanding with code, <kbd>Enter</kbd> or a double click restores that size; dragging
    /// out follows the pointer.
    /// One side per split: calling this again replaces the earlier one.
    pub fn collapsible(mut self, side: Side, collapsed: &'a mut bool) -> Self {
        self.collapse = Some(Collapse { side, collapsed });
        self
    }

    /// A locked split (`false`) has a divider that ignores the pointer and the keyboard.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the divider draws a line (default `true`). Between islands the gap itself is
    /// the divider, as in the IntelliJ Islands UI: pass `false` and it stays the frame color,
    /// showing only the resize cursor. A collapsed edge and the focus ring still show.
    pub fn line(mut self, line: bool) -> Self {
        self.line = line;
        self
    }

    /// A tooltip, context menu … on the divider.
    pub fn with(mut self, behaviour: impl Behaviour + 'a) -> Self {
        self.behaviours.push(behaviour);
        self
    }

    /// Draws the panes into `first` (left or top) and `second` (right or bottom) and the
    /// divider between them. Each closure gets a `Ui` that is exactly its pane.
    pub fn show<A, B>(
        mut self,
        ui: &mut Ui,
        first: impl FnOnce(&mut Ui) -> A,
        second: impl FnOnce(&mut Ui) -> B,
    ) -> SplitterResponse<A, B> {
        let colors = Theme::colors(ui.ctx());
        let horizontal = self.orientation == Orientation::Horizontal;
        let divider_id = ui.id().with(self.id).with("divider");

        // Fill the available space, without panicking in unbounded layouts.
        let finite = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
        let avail = ui.available_size_before_wrap();
        let (_, whole) =
            ui.allocate_exact_size(vec2(finite(avail.x), finite(avail.y)), Sense::hover());
        let whole = whole.rect;

        let layout = Geometry::new(
            self.orientation,
            self.default_ratio,
            (self.min_first, self.min_second),
            whole,
        );
        let side = self.collapse.as_ref().map(|c| c.side);
        let was_collapsed = self.collapse.as_ref().is_some_and(|c| *c.collapsed);
        // The first pane's size with the pane collapsed or not.
        let resting = |collapsed: bool, ratio: f32| match side {
            Some(Side::First) if collapsed => 0.0,
            Some(Side::Second) if collapsed => layout.avail,
            _ => layout.first_size(ratio),
        };
        let first_size = resting(was_collapsed, *self.ratio).round_to_pixels(ui.pixels_per_point());
        let (_, bar, _) = layout.rects(first_size);

        // The divider takes part in Tab order before the panes, so an outer divider comes
        // before those of splits nested in its panes, and it moves them in the same frame.
        let grab = bar.expand2(if horizontal {
            vec2(size::SPLIT_GRAB, 0.0)
        } else {
            vec2(0.0, size::SPLIT_GRAB)
        });
        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(
            grab.intersect(whole.expand(size::SPLIT_GRAB)),
            divider_id,
            sense,
        );

        let resize_cursor = if horizontal {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        };
        if self.enabled {
            cursor::on_hover(&response, resize_cursor);
            if response.dragged() {
                // The pointer may leave the thin divider while dragging.
                ui.ctx().set_cursor_icon(resize_cursor);
            }
        }

        let along = |p: egui::Pos2| if horizontal { p.x } else { p.y };
        let mut target = None;
        let mut collapse_to = None;
        if self.enabled {
            let center = along(bar.center());
            if response.drag_started()
                && let Some(pressed) = ui.input(|i| i.pointer.press_origin())
            {
                // Remember where on the divider it was grabbed (the drag only starts after
                // the pointer has moved a little), so the divider doesn't jump.
                ui.data_mut(|d| d.insert_temp(divider_id, along(pressed) - center));
            }
            if response.drag_started() {
                // The ratio from before the drag, which a collapse by dragging restores.
                ui.data_mut(|d| d.insert_temp(divider_id.with("ratio"), *self.ratio));
            }
            if response.dragged()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let grabbed: f32 = ui.data(|d| d.get_temp(divider_id)).unwrap_or(0.0);
                let start = if horizontal {
                    whole.left()
                } else {
                    whole.top()
                };
                let wanted = along(pointer) - grabbed - size::SPLIT_DIVIDER / 2.0 - start;
                target = Some(wanted);
                if let Some(side) = side {
                    // How big the collapsible pane would be if it followed the pointer,
                    // against its minimum: collapse past the threshold, expand again only
                    // once it is close to its minimum (so it doesn't flicker in between).
                    let (pane, min) = match side {
                        Side::First => (wanted, self.min_first),
                        Side::Second => (layout.avail - wanted, self.min_second),
                    };
                    if !was_collapsed && pane < min - size::SPLIT_COLLAPSE {
                        collapse_to = Some(true);
                    } else if was_collapsed && pane >= min - size::SPLIT_COLLAPSE / 2.0 {
                        collapse_to = Some(false);
                    }
                    if collapse_to.unwrap_or(was_collapsed) {
                        target = None;
                    }
                }
            }
            if response.double_clicked() {
                if was_collapsed {
                    collapse_to = Some(false);
                } else {
                    target = Some(layout.default_size());
                }
            }
            if response.has_focus() {
                let (back, forward) = if horizontal {
                    (Key::ArrowLeft, Key::ArrowRight)
                } else {
                    (Key::ArrowUp, Key::ArrowDown)
                };
                // The arrow keys move the divider, not the focus.
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        divider_id,
                        EventFilter {
                            horizontal_arrows: horizontal,
                            vertical_arrows: !horizontal,
                            ..EventFilter::default()
                        },
                    );
                });
                let press = |key| ui.input_mut(|i| i.count_and_consume_key(Modifiers::NONE, key));
                let steps = press(forward) as f32 - press(back) as f32;
                let toggle = side.is_some() && press(Key::Enter) + press(Key::Space) > 0;
                if toggle {
                    collapse_to = Some(!was_collapsed);
                } else if was_collapsed {
                    // Only the arrow toward the content does anything: it expands.
                    let toward = match side {
                        Some(Side::Second) => steps < 0.0,
                        _ => steps > 0.0,
                    };
                    if toward {
                        collapse_to = Some(false);
                    }
                } else if steps != 0.0 {
                    target = Some(first_size + steps * size::SPLIT_KEY_STEP);
                } else if press(Key::Home) > 0 {
                    target = Some(f32::NEG_INFINITY);
                } else if press(Key::End) > 0 {
                    target = Some(f32::INFINITY);
                }
            }
        }
        let collapsed = collapse_to.unwrap_or(was_collapsed);
        if collapsed != was_collapsed
            && let Some(collapse) = self.collapse.as_mut()
        {
            *collapse.collapsed = collapsed;
            response.mark_changed();
            ui.ctx().request_repaint();
        }
        if collapsed && !was_collapsed && response.dragged() {
            // Dragging into the collapse moved the ratio down to the minimum on the way:
            // put back the size from before the drag, so expanding restores that.
            if let Some(before) = ui.data(|d| d.get_temp::<f32>(divider_id.with("ratio"))) {
                *self.ratio = before;
            }
        }
        let mut shown_size = if collapsed == was_collapsed {
            first_size
        } else {
            resting(collapsed, *self.ratio)
        };
        if let Some(wanted) = target {
            let size = layout.clamp(wanted);
            let new_ratio = layout.ratio_of(size);
            if (new_ratio - layout.ratio(*self.ratio)).abs() > 1e-4 {
                *self.ratio = new_ratio;
                response.mark_changed();
                ui.ctx().request_repaint();
            }
            shown_size = size;
        }
        let (first_rect, bar, second_rect) = layout.rects(shown_size);
        let mut pane = |rect: Rect, name: &str, hidden: bool, add: &mut dyn FnMut(&mut Ui)| {
            let mut builder = UiBuilder::new()
                .id_salt((self.id, name))
                .max_rect(rect)
                .layout(egui::Layout::top_down(Align::Min));
            if hidden {
                builder = builder.invisible();
            }
            let mut child = ui.new_child(builder);
            child.set_clip_rect(if hidden {
                Rect::from_min_size(rect.min, vec2(0.0, 0.0))
            } else {
                rect.intersect(ui.clip_rect())
            });
            add(&mut child);
        };
        // A collapsed pane is laid out hidden at its minimum size, so what it holds doesn't
        // reflow into a sliver.
        let hidden = |this: Side, rect: Rect, min: f32| {
            let hidden = collapsed && side == Some(this);
            let width = |len: f32| len.min(min.max(1.0));
            let rect = match (hidden, horizontal, this) {
                (false, ..) => rect,
                (true, true, Side::First) => {
                    Rect::from_min_size(whole.min, vec2(width(whole.width()), whole.height()))
                }
                (true, true, Side::Second) => Rect::from_min_max(
                    pos2(whole.right() - width(whole.width()), whole.top()),
                    whole.max,
                ),
                (true, false, Side::First) => {
                    Rect::from_min_size(whole.min, vec2(whole.width(), width(whole.height())))
                }
                (true, false, Side::Second) => Rect::from_min_max(
                    pos2(whole.left(), whole.bottom() - width(whole.height())),
                    whole.max,
                ),
            };
            (hidden, rect)
        };
        let (mut first_out, mut second_out) = (None, None);
        let (mut first, mut second) = (Some(first), Some(second));
        let (hide_first, first_rect) = hidden(Side::First, first_rect, self.min_first);
        let (hide_second, second_rect) = hidden(Side::Second, second_rect, self.min_second);
        pane(first_rect, "first", hide_first, &mut |ui| {
            first_out = first.take().map(|f| f(ui));
        });
        pane(second_rect, "second", hide_second, &mut |ui| {
            second_out = second.take().map(|f| f(ui));
        });

        let (mut min_ratio, mut max_ratio) = layout.ratio_range();
        match side {
            Some(Side::First) => min_ratio = 0.0,
            Some(Side::Second) => max_ratio = 1.0,
            None => {}
        }
        let shown_ratio = layout.ratio_of(shown_size);
        response.widget_info(|| {
            WidgetInfo::slider(self.enabled, f64::from(shown_ratio * 100.0), &self.label)
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Splitter);
            node.set_orientation(if horizontal {
                egui::accesskit::Orientation::Vertical
            } else {
                egui::accesskit::Orientation::Horizontal
            });
            node.set_min_numeric_value(f64::from(min_ratio * 100.0));
            node.set_max_numeric_value(f64::from(max_ratio * 100.0));
            if side.is_some() {
                node.set_expanded(!collapsed);
            }
            node.set_numeric_value_step(f64::from(layout.ratio_of(size::SPLIT_KEY_STEP) * 100.0));
        });

        if ui.is_rect_visible(grab) {
            let active =
                self.enabled && (response.dragged() || response.is_pointer_button_down_on());
            let (thickness, color) = if active {
                (2.0, colors.accent)
            } else if self.enabled && response.hovered() {
                // A collapsed edge already looks like a hovered divider: take the accent
                // color, so it shows it can be grabbed.
                (
                    2.0,
                    if collapsed {
                        colors.accent
                    } else {
                        colors.control_border
                    },
                )
            } else if collapsed {
                // A collapsed pane's edge must stay easy to see.
                (2.0, colors.control_border)
            } else {
                (1.0, colors.separator)
            };
            // Without a line, only a collapsed edge is drawn: the gap is the divider.
            let color = if self.line || collapsed {
                color
            } else {
                egui::Color32::TRANSPARENT
            };
            let line = if horizontal {
                Rect::from_center_size(bar.center(), vec2(thickness, bar.height()))
            } else {
                Rect::from_center_size(bar.center(), vec2(bar.width(), thickness))
            };
            let painter = ui.painter();
            painter.rect_filled(line, CornerRadius::same(radius::UNDERLINE), color);
            if response.has_focus() {
                focus_ring(painter, bar, radius::UNDERLINE, &colors);
            }
        }

        let divider = self.behaviours.attach(ui, response);
        SplitterResponse {
            first: first_out.expect("the first pane is drawn exactly once"),
            second: second_out.expect("the second pane is drawn exactly once"),
            divider,
        }
    }
}

/// The geometry of a split: how big the first pane may be, and where everything goes.
struct Geometry {
    horizontal: bool,
    default_ratio: f32,
    min: (f32, f32),
    whole: Rect,
    /// Space shared by the panes: the whole minus the divider.
    avail: f32,
}

impl Geometry {
    fn new(orientation: Orientation, default_ratio: f32, min: (f32, f32), whole: Rect) -> Self {
        let horizontal = orientation == Orientation::Horizontal;
        let length = if horizontal {
            whole.width()
        } else {
            whole.height()
        };
        Self {
            horizontal,
            default_ratio,
            min,
            whole,
            avail: (length - size::SPLIT_DIVIDER).max(0.0),
        }
    }

    /// The range of sizes the first pane may have. When there is not room for both
    /// minimums, the space is shared in proportion to them.
    fn range(&self) -> (f32, f32) {
        let (a, b) = self.min;
        if self.avail >= a + b {
            (a, self.avail - b)
        } else if a + b > 0.0 {
            let shared = self.avail * a / (a + b);
            (shared, shared)
        } else {
            (0.0, 0.0)
        }
    }

    fn clamp(&self, size: f32) -> f32 {
        let (lo, hi) = self.range();
        size.clamp(lo, hi)
    }

    /// `ratio` if usable, else the default (a caller may hand in NaN or garbage).
    fn ratio(&self, ratio: f32) -> f32 {
        let usable = |r: f32| r.is_finite().then(|| r.clamp(0.0, 1.0));
        usable(ratio)
            .or_else(|| usable(self.default_ratio))
            .unwrap_or(0.5)
    }

    fn first_size(&self, ratio: f32) -> f32 {
        self.clamp(self.ratio(ratio) * self.avail)
    }

    fn default_size(&self) -> f32 {
        self.clamp(self.ratio(self.default_ratio) * self.avail)
    }

    fn ratio_of(&self, size: f32) -> f32 {
        if self.avail > 0.0 {
            (size / self.avail).clamp(0.0, 1.0)
        } else {
            self.ratio(self.default_ratio)
        }
    }

    fn ratio_range(&self) -> (f32, f32) {
        let (lo, hi) = self.range();
        (self.ratio_of(lo), self.ratio_of(hi))
    }

    /// First pane, divider and second pane for a first pane of `first` points.
    fn rects(&self, first: f32) -> (Rect, Rect, Rect) {
        let w = self.whole;
        let d = size::SPLIT_DIVIDER;
        if self.horizontal {
            (
                Rect::from_min_max(w.min, pos2(w.left() + first, w.bottom())),
                Rect::from_min_max(
                    pos2(w.left() + first, w.top()),
                    pos2(w.left() + first + d, w.bottom()),
                ),
                Rect::from_min_max(pos2(w.left() + first + d, w.top()), w.max),
            )
        } else {
            (
                Rect::from_min_max(w.min, pos2(w.right(), w.top() + first)),
                Rect::from_min_max(
                    pos2(w.left(), w.top() + first),
                    pos2(w.right(), w.top() + first + d),
                ),
                Rect::from_min_max(pos2(w.left(), w.top() + first + d), w.max),
            )
        }
    }
}
