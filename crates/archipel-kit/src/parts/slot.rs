//! Slots: places in a component that take the caller's content.

use super::{State, Tone};
use crate::common;
use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{size, typography};
use egui::{Layout, Rect, Ui, UiBuilder, pos2, vec2};

/// Content for a place in a component, such as the trailing end of a row: an icon, dimmed
/// text, or any widgets. Text and icons are painted in the component's state (dimmed when
/// it is disabled, strong when it is selected); widgets are drawn in a child `Ui` laid out
/// right to left, so put the rightmost one first.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{action_system::ActionButton, ui::ColoredListCellRenderer};
/// use archipel_kit::parts::Slot;
/// use archipel_kit::Glyph;
///
/// ColoredListCellRenderer::new("build.gradle.kts")
///     .trailing(Slot::text("modified"))
///     .show(ui);
/// ColoredListCellRenderer::new("app")
///     .trailing(Slot::ui(|ui| {
///         ui.add(ActionButton::new(Glyph::Close, "Remove app").small(true));
///     }))
///     .show(ui);
/// # });
/// ```
pub struct Slot<'a> {
    kind: SlotKind<'a>,
}

enum SlotKind<'a> {
    Icon(Icon, Tone),
    Text(String, Tone),
    Ui(f32, Box<dyn FnOnce(&mut Ui) + 'a>),
}

impl<'a> Slot<'a> {
    /// An icon in the icon gray.
    pub fn icon(icon: impl Into<Icon>) -> Self {
        Self::icon_tone(icon, Tone::Icon)
    }

    /// An icon colored by meaning.
    pub fn icon_tone(icon: impl Into<Icon>, tone: Tone) -> Self {
        Self {
            kind: SlotKind::Icon(icon.into(), tone),
        }
    }

    /// Dimmed text: a count, a shortcut, a status.
    pub fn text(text: impl Into<String>) -> Self {
        Self::text_tone(text, Tone::Muted)
    }

    /// Text colored by meaning.
    pub fn text_tone(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            kind: SlotKind::Text(text.into(), tone),
        }
    }

    /// Widgets drawn by `add` in a square one row high: room for one small icon button
    /// (`ActionButton::small`). Use [`Slot::ui_width`] for more.
    pub fn ui(add: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self::ui_width(size::ROW_HEIGHT, add)
    }

    /// Widgets drawn by `add` in a space `width` points wide.
    pub fn ui_width(width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            kind: SlotKind::Ui(width, Box::new(add)),
        }
    }

    /// Whether the slot holds widgets (which take input) rather than painted content.
    pub fn is_interactive(&self) -> bool {
        matches!(self.kind, SlotKind::Ui(..))
    }

    /// Width the content needs in `ui`.
    pub fn width(&self, ui: &Ui) -> f32 {
        match &self.kind {
            SlotKind::Icon(..) => size::ICON,
            SlotKind::Text(text, _) => {
                common::galley(ui, text.as_str(), typography::body(typography::BODY))
                    .size()
                    .x
            }
            SlotKind::Ui(width, _) => *width,
        }
    }

    /// Shows the content right-aligned in `rect` (vertically centered), in `state`.
    pub fn show(self, ui: &mut Ui, rect: Rect, state: State) {
        let colors = Theme::colors(ui.ctx());
        match self.kind {
            SlotKind::Icon(icon, tone) => {
                let icon_rect = Rect::from_min_size(
                    pos2(
                        rect.right() - size::ICON,
                        rect.center().y - size::ICON / 2.0,
                    ),
                    vec2(size::ICON, size::ICON),
                );
                icons::paint(ui.painter(), icon_rect, icon, state.icon(tone, &colors));
            }
            SlotKind::Text(text, tone) => {
                let galley = common::galley(ui, text.as_str(), typography::body(typography::BODY));
                let x = rect.right() - galley.size().x;
                common::text_left(ui.painter(), rect, x, galley, state.text(tone, &colors));
            }
            SlotKind::Ui(_, add) => {
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(rect)
                        .layout(Layout::right_to_left(egui::Align::Center)),
                );
                if !state.enabled {
                    child.disable();
                }
                add(&mut child);
            }
        }
    }
}
