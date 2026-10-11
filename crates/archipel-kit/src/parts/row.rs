//! The content of a row: icon, label, secondary text, trailing text.

use super::{State, Tone};
use crate::components::common;
use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{size, typography};
use egui::{Rect, Ui, pos2, vec2};

/// What a row shows, laid out the same in lists, trees, menus and popups: an icon column,
/// the label, dimmed secondary text after it, and right-aligned trailing text (a shortcut, a
/// count).
#[derive(Debug, Clone, Default)]
pub struct RowContent<'a> {
    /// Icon in the icon column.
    pub icon: Option<Icon>,
    /// Meaning of the icon's color (default: the icon gray).
    pub icon_tone: Option<Tone>,
    /// Keep the icon column even without an icon, so labels of mixed rows line up (menus).
    pub reserve_icon: bool,
    /// The label.
    pub label: &'a str,
    /// Meaning of the label's color.
    pub label_tone: Tone,
    /// Dimmed text after the label.
    pub secondary: Option<&'a str>,
    /// Right-aligned dimmed text.
    pub trailing: Option<&'a str>,
}

impl<'a> RowContent<'a> {
    /// A row reading `label`.
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            ..Self::default()
        }
    }

    /// Width the content needs (without trailing padding).
    pub fn min_width(&self, ui: &Ui) -> f32 {
        let font = typography::body(typography::BODY);
        let text = |s: &str| common::galley(ui, s, font.clone()).size().x;
        let icon = if self.icon.is_some() || self.reserve_icon {
            size::ICON + size::ICON_TEXT_GAP
        } else {
            0.0
        };
        icon + text(self.label)
            + self
                .secondary
                .map_or(0.0, |s| size::SECONDARY_GAP + text(s))
            + self.trailing.map_or(0.0, |s| 32.0 + text(s))
    }

    /// Paints the content in `rect`, starting at `x`, in `state`.
    pub fn paint(&self, ui: &Ui, rect: Rect, x: f32, state: State) {
        let colors = Theme::colors(ui.ctx());
        let painter = ui.painter();
        let mut x = x;
        if self.icon.is_some() || self.reserve_icon {
            let icon_rect = Rect::from_min_size(
                pos2(x, rect.center().y - size::ICON / 2.0),
                vec2(size::ICON, size::ICON),
            );
            if let Some(icon) = self.icon {
                let tone = self.icon_tone.unwrap_or(Tone::Icon);
                icons::paint(painter, icon_rect, icon, state.icon(tone, &colors));
            }
            x = icon_rect.right() + size::ICON_TEXT_GAP;
        }
        let font = typography::body(typography::BODY);
        let label = common::galley(ui, self.label, font.clone());
        let label_width = label.size().x;
        common::text_left(
            painter,
            rect,
            x,
            label,
            state.text(self.label_tone, &colors),
        );
        if let Some(secondary) = self.secondary {
            let galley = common::galley(ui, secondary, font.clone());
            let x = x + label_width + size::SECONDARY_GAP;
            common::text_left(painter, rect, x, galley, state.text(Tone::Muted, &colors));
        }
        if let Some(trailing) = self.trailing {
            let galley = common::galley(ui, trailing, font);
            let x = rect.right() - 8.0 - galley.size().x;
            common::text_left(painter, rect, x, galley, state.text(Tone::Muted, &colors));
        }
    }
}
