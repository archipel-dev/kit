//! Status bar widgets.

use crate::behaviour::{Behaviour, Behaviours};
use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::ide::HelpTooltip;
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
/// use archipel_kit::wm::StatusBarWidget;
/// use archipel_kit::{Glyph, Tone};
///
/// ui.horizontal(|ui| {
///     if ui.add(StatusBarWidget::new("main").icon(Glyph::Branch).chevron(true)).clicked() {
///         // open the branches popup
///     }
///     ui.add(StatusBarWidget::new("UTF-8"));
///     ui.add(StatusBarWidget::new("2 errors").tone(Tone::Error));
/// });
/// # });
/// ```
#[must_use = "add it with `ui.add(item)`"]
pub struct StatusBarWidget<'a> {
    text: String,
    icon: Option<Icon>,
    chevron: bool,
    tone: Tone,
    enabled: bool,
    max_width: Option<f32>,
    hover_effect: bool,
    alignment: f32,
    reserve: Option<String>,
    name: Option<String>,
    behaviours: Behaviours<'a>,
}

impl<'a> StatusBarWidget<'a> {
    /// An item reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            chevron: false,
            tone: Tone::Default,
            enabled: true,
            max_width: None,
            hover_effect: true,
            alignment: 0.0,
            reserve: None,
            name: None,
            behaviours: Behaviours::default(),
        }
    }

    /// Whether hover and press fill the widget (default `true`). IntelliJ paints the effect
    /// for the status bar's right panel only (spec section 9: arc 4, the hover color for
    /// hover and press); widgets of the left and center panels have none and are drawn
    /// without it.
    pub fn hover_effect(mut self, hover_effect: bool) -> Self {
        self.hover_effect = hover_effect;
        self
    }

    /// Where the text sits in the width the widget has beyond it, from `0.0` (left, the
    /// default) to `1.0` (right) (IntelliJ `TextPresentation.getAlignment`).
    pub fn text_alignment(mut self, alignment: f32) -> Self {
        self.alignment = alignment.clamp(0.0, 1.0);
        self
    }

    /// Reserves the width of `prototype` for the text, so a value that changes (a line and
    /// column) does not move the widgets beside it (IntelliJ
    /// `TextPresentation.getMaxValue`).
    pub fn reserve_width_of(mut self, prototype: impl Into<String>) -> Self {
        self.reserve = Some(prototype.into());
        self
    }

    /// The accessible name and tooltip when the text alone does not name it (an icon-only
    /// widget).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
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

impl Widget for StatusBarWidget<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        let text = common::galley(ui, self.text.as_str(), font.clone());
        // The text's room: at least the prototype's, when there is one.
        let text_room = match &self.reserve {
            Some(prototype) => common::galley(ui, prototype.as_str(), font.clone())
                .size()
                .x
                .max(text.size().x),
            None => text.size().x,
        };
        let gap = if self.text.is_empty() {
            0.0
        } else {
            size::ICON_TEXT_GAP
        };
        let icon = if self.icon.is_some() {
            size::ICON + gap
        } else {
            0.0
        };
        let chevron = if self.chevron { size::ICON } else { 0.0 };
        let metrics = crate::tokens::metrics(ui.ctx());
        let padding = metrics.status_item_padding;
        let extra = 2.0 * padding + icon + chevron;
        let limit = self
            .max_width
            .unwrap_or_else(|| ui.available_width())
            .max(extra);
        let width = (text_room + extra).min(limit);
        let elided = text_room + extra > limit && text.size().x + extra > limit;
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, metrics.status_item_height), sense);
        crate::cursor::on_hover_action(&response);
        let name = self.name.as_deref().unwrap_or(&self.text);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, name));
        if ui.is_rect_visible(rect) {
            let mut state = State::of(&response);
            if !self.hover_effect {
                state.hovered = false;
                state.pressed = false;
            }
            let painter = ui.painter();
            // Hover and press share one fill (Islands: the same token for both).
            let fill = if state.enabled && (state.hovered || state.pressed) {
                colors.hover
            } else {
                egui::Color32::TRANSPARENT
            };
            if fill != egui::Color32::TRANSPARENT {
                painter.rect_filled(rect, CornerRadius::same(metrics.status_hover_radius), fill);
            }
            if response.has_focus() {
                common::focus_ring(painter, rect, radius::STATUS_FOCUS, &colors);
            }
            let mut x = rect.left() + padding;
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
                x += size::ICON + gap;
            }
            let room = rect.right() - padding - chevron - x;
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
            // Aligned in the room the widget has beyond the text.
            let spare = (room - galley.size().x).max(0.0);
            common::text_left(painter, rect, x + spare * self.alignment, galley, color);
            if self.chevron {
                let chevron_rect = Rect::from_min_size(
                    pos2(
                        rect.right() - padding - size::ICON,
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
        if !self.behaviours.has_tooltip() {
            if elided {
                HelpTooltip::new(self.text.clone()).show_for(&response);
            } else if let Some(name) = &self.name {
                HelpTooltip::new(name.clone()).show_for(&response);
            }
        }
        self.behaviours.attach(ui, response)
    }
}
