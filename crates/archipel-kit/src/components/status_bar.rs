//! Status bar widgets.

use super::{Tooltip, common};
use crate::behaviour::{Behaviour, Behaviours};
use crate::icons::{self, Glyph, Icon};
use crate::parts::{State, Tone};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{CornerRadius, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

/// A flat widget of the status bar or a toolbar: an optional icon, text and an optional
/// chevron (it opens a popup). Hover and press fill it like a toolbar button; it has the
/// action cursor and is a button to screen readers, named by its text.
///
/// It takes at most the available width, or [`max_width`](Self::max_width): longer text is
/// elided with `…` and shown in full as the tooltip. Put it on a
/// [`Surface::Chrome`](crate::parts::Surface::Chrome) bar and add behaviours with
/// [`with`](crate::behaviour::WidgetExt::with).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::StatusBarItem;
/// use archipel_kit::{Glyph, Tone};
///
/// ui.horizontal(|ui| {
///     if ui.add(StatusBarItem::new("main").icon(Glyph::Branch).chevron(true)).clicked() {
///         // open the branches popup
///     }
///     ui.add(StatusBarItem::new("UTF-8"));
///     ui.add(StatusBarItem::new("2 errors").tone(Tone::Error));
/// });
/// # });
/// ```
#[must_use = "add it with `ui.add(item)`"]
pub struct StatusBarItem<'a> {
    text: String,
    icon: Option<Icon>,
    chevron: bool,
    tone: Tone,
    enabled: bool,
    max_width: Option<f32>,
    behaviours: Behaviours<'a>,
}

impl<'a> StatusBarItem<'a> {
    /// An item reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            chevron: false,
            tone: Tone::Default,
            enabled: true,
            max_width: None,
            behaviours: Behaviours::default(),
        }
    }

    /// Icon before the text.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// A chevron after the text: the item opens a popup.
    pub fn chevron(mut self, chevron: bool) -> Self {
        self.chevron = chevron;
        self
    }

    /// Meaning of the text and icon color, e.g. [`Tone::Error`].
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Disabled items are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Adds `behaviour` (a tooltip, context menu …) to the item. A tooltip replaces the one
    /// the item shows for elided text.
    pub fn with(mut self, behaviour: impl Behaviour + 'a) -> Self {
        self.behaviours.push(behaviour);
        self
    }

    /// The widest the item may be (default: the available width).
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }
}

impl Widget for StatusBarItem<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        let text = common::galley(ui, self.text.as_str(), font.clone());
        let icon = if self.icon.is_some() {
            size::ICON + size::ICON_TEXT_GAP
        } else {
            0.0
        };
        let chevron = if self.chevron { size::ICON } else { 0.0 };
        let extra = 2.0 * size::STATUS_ITEM_PADDING + icon + chevron;
        let limit = self
            .max_width
            .unwrap_or_else(|| ui.available_width())
            .max(extra);
        let width = (text.size().x + extra).min(limit);
        let elided = text.size().x + extra > limit;
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, size::STATUS_ITEM_HEIGHT), sense);
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.text));
        if ui.is_rect_visible(rect) {
            let state = State::of(&response);
            let painter = ui.painter();
            let fill = state.flat_fill(&colors);
            if fill != egui::Color32::TRANSPARENT {
                painter.rect_filled(rect, CornerRadius::same(radius::CONTROL), fill);
            }
            if response.has_focus() {
                common::focus_ring(painter, rect, radius::CONTROL, &colors);
            }
            let mut x = rect.left() + size::STATUS_ITEM_PADDING;
            if let Some(icon) = self.icon {
                let icon_rect = Rect::from_min_size(
                    pos2(x, rect.center().y - size::ICON / 2.0),
                    vec2(size::ICON, size::ICON),
                );
                let tone = if self.tone == Tone::Default {
                    Tone::Icon
                } else {
                    self.tone
                };
                icons::paint(painter, icon_rect, icon, state.icon(tone, &colors));
                x += size::ICON + size::ICON_TEXT_GAP;
            }
            let room = rect.right() - size::STATUS_ITEM_PADDING - chevron - x;
            let galley = if elided {
                common::galley_elided(ui, &self.text, font, room.max(0.0))
            } else {
                text
            };
            // As in Android Studio: dimmed at rest, full strength under the pointer.
            let color = if self.tone == Tone::Default && state.enabled {
                if state.hovered || state.pressed || state.focused {
                    colors.text
                } else {
                    colors.text_secondary
                }
            } else {
                state.text(self.tone, &colors)
            };
            common::text_left(painter, rect, x, galley, color);
            if self.chevron {
                let chevron_rect = Rect::from_min_size(
                    pos2(
                        rect.right() - size::STATUS_ITEM_PADDING - size::ICON,
                        rect.center().y - size::ICON / 2.0,
                    ),
                    vec2(size::ICON, size::ICON),
                );
                icons::paint(
                    painter,
                    chevron_rect,
                    Glyph::ChevronDown,
                    state.icon(Tone::Icon, &colors),
                );
            }
        }
        if elided && !self.behaviours.has_tooltip() {
            Tooltip::new(self.text.clone()).show_for(&response);
        }
        self.behaviours.attach(ui, response)
    }
}

/// A path of places, like Android Studio's navigation path at the left of the status bar
/// (`project › src › main.rs`) or a settings page's location. Each crumb is a flat button
/// with an optional icon; dimmed chevrons separate them. Text is dimmed at rest and full
/// strength under the pointer. The response says which crumb was clicked.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, Icon, components::Breadcrumbs};
///
/// let clicked = Breadcrumbs::new()
///     .crumb("archipel", Some(Glyph::Folder.into()))
///     .crumb("main.rs", Some(Icon::for_file("main.rs")))
///     .show(ui)
///     .clicked;
/// # let _ = clicked;
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
#[derive(Default)]
pub struct Breadcrumbs {
    crumbs: Vec<(String, Option<Icon>)>,
}

impl Breadcrumbs {
    /// An empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a crumb reading `label`, with an optional icon.
    pub fn crumb(mut self, label: impl Into<String>, icon: Option<Icon>) -> Self {
        self.crumbs.push((label.into(), icon));
        self
    }

    /// Draws the path in a row, [`size::STATUS_ITEM_HEIGHT`] high.
    pub fn show(self, ui: &mut Ui) -> BreadcrumbsResponse {
        let colors = Theme::colors(ui.ctx());
        let mut clicked = None;
        let mut crumbs = Vec::with_capacity(self.crumbs.len());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::BREADCRUMB_GAP;
            let count = self.crumbs.len();
            for (index, (label, icon)) in self.crumbs.iter().enumerate() {
                let galley = common::galley(ui, label.as_str(), typography::body(typography::BODY));
                let icon_width = if icon.is_some() {
                    size::ICON + size::ICON_TEXT_GAP
                } else {
                    0.0
                };
                let width = 2.0 * size::BREADCRUMB_PADDING + icon_width + galley.size().x;
                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, size::STATUS_ITEM_HEIGHT), Sense::click());
                crate::cursor::on_hover_action(&response);
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
                if response.clicked() {
                    clicked = Some(index);
                }
                let state = State::of(&response);
                let enabled = response.enabled();
                if ui.is_rect_visible(rect) {
                    let painter = ui.painter();
                    let fill = state.flat_fill(&colors);
                    if fill != egui::Color32::TRANSPARENT {
                        painter.rect_filled(rect, CornerRadius::same(radius::CONTROL), fill);
                    }
                    if response.has_focus() {
                        common::focus_ring(painter, rect, radius::CONTROL, &colors);
                    }
                    let mut x = rect.left() + size::BREADCRUMB_PADDING;
                    if let Some(icon) = *icon {
                        let icon_rect = Rect::from_min_size(
                            pos2(x, rect.center().y - size::ICON / 2.0),
                            vec2(size::ICON, size::ICON),
                        );
                        icons::paint(painter, icon_rect, icon, state.icon(Tone::Icon, &colors));
                        x += icon_width;
                    }
                    let color = if !enabled {
                        colors.text_disabled
                    } else if state.hovered || state.pressed || state.focused {
                        colors.text
                    } else {
                        colors.text_secondary
                    };
                    common::text_left(painter, rect, x, galley, color);
                }
                crumbs.push(response);
                if index + 1 < count {
                    let (chevron, _) = ui.allocate_exact_size(
                        vec2(size::ICON, size::STATUS_ITEM_HEIGHT),
                        Sense::hover(),
                    );
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(chevron.center(), vec2(size::ICON, size::ICON)),
                        Glyph::ChevronRight,
                        colors.text_placeholder,
                    );
                }
            }
        });
        BreadcrumbsResponse { clicked, crumbs }
    }
}

/// What [`Breadcrumbs::show`] drew.
pub struct BreadcrumbsResponse {
    /// The index of the crumb clicked this frame.
    pub clicked: Option<usize>,
    /// Each crumb's response, in order.
    pub crumbs: Vec<Response>,
}
