//! Small inline labels: counters, tags, keyboard shortcuts and avatars.

use crate::common;
use crate::parts::Tone;
use crate::theme::{Theme, avatar_color};
use crate::tokens::{radius, typography};
use egui::{
    Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetType,
    vec2,
};

/// A rounded count, e.g. the number of problems or unread notifications. Counts above
/// 99 show as `99+`.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::Counter;
///
/// ui.add(Counter::new(3));
/// ui.add(Counter::new(12).accent(true));
/// # });
/// ```
#[must_use = "add it with `ui.add(counter)`"]
pub struct Counter {
    count: usize,
    accent: bool,
}

impl Counter {
    /// A counter showing `count`.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            accent: false,
        }
    }

    /// Fills it with the accent color, to draw attention (new items).
    pub fn accent(mut self, accent: bool) -> Self {
        self.accent = accent;
        self
    }
}

impl Widget for Counter {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let text = if self.count > 99 {
            "99+".to_owned()
        } else {
            self.count.to_string()
        };
        let galley = common::galley(
            ui,
            text.as_str(),
            typography::semibold(ui.ctx(), typography::SMALL),
        );
        let width = (galley.size().x + 10.0).max(16.0);
        let (rect, response) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
        let (fill, color) = if self.accent {
            (colors.accent, colors.text_on_accent)
        } else {
            (colors.counter, colors.counter_text)
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius::PILL), fill);
        common::text(
            ui.painter(),
            rect.center() - galley.size() / 2.0,
            galley,
            color,
        );
        response
    }
}

/// A short label chip: a status ("beta"), a type ("GET"), a branch or a filter.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::Tag;
/// use archipel_kit::Tone;
///
/// ui.add(Tag::new("beta"));
/// ui.add(Tag::new("GET").tone(Tone::Success));
/// # });
/// ```
#[must_use = "add it with `ui.add(tag)`"]
pub struct Tag {
    text: String,
    tone: Option<Tone>,
}

impl Tag {
    /// A neutral tag.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: None,
        }
    }

    /// Tints the tag: text in `tone` on a faint fill of the same color.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }
}

impl Widget for Tag {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let galley = common::galley(
            ui,
            self.text.as_str(),
            typography::semibold(ui.ctx(), typography::SMALL),
        );
        let (rect, response) =
            ui.allocate_exact_size(vec2(galley.size().x + 12.0, 18.0), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &self.text));
        let (fill, text) = match self.tone.map(|tone| tone.color(&colors)) {
            Some(color) => (color.gamma_multiply(0.18), color),
            None => (colors.tag, colors.text_secondary),
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius::CONTROL), fill);
        common::text(
            ui.painter(),
            rect.center() - galley.size() / 2.0,
            galley,
            text,
        );
        response
    }
}

/// A keyboard shortcut drawn as key caps, e.g. `Kbd::new(["⌘", "K"])`.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::Kbd;
///
/// ui.add(Kbd::new(["Ctrl", "Shift", "A"]));
/// ui.add(Kbd::parse("Ctrl+Shift+F"));
/// # });
/// ```
#[must_use = "add it with `ui.add(kbd)`"]
pub struct Kbd {
    keys: Vec<String>,
}

impl Kbd {
    /// Key caps for `keys`, in order.
    pub fn new<K: Into<String>>(keys: impl IntoIterator<Item = K>) -> Self {
        Self {
            keys: keys.into_iter().map(Into::into).collect(),
        }
    }

    /// Splits a shortcut like `Ctrl+Shift+F` at `+` (a lone `+` key stays a key).
    pub fn parse(shortcut: &str) -> Self {
        let mut keys: Vec<String> = Vec::new();
        for part in shortcut.split('+') {
            if part.is_empty() {
                if keys.last().is_none_or(|key| key != "+") {
                    keys.push("+".into());
                }
            } else {
                keys.push(part.trim().to_owned());
            }
        }
        Self { keys }
    }

    /// The keys, in order.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

impl Widget for Kbd {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::SMALL + 1.0);
        let galleys: Vec<_> = self
            .keys
            .iter()
            .map(|key| common::galley(ui, key.as_str(), font.clone()))
            .collect();
        let gap = 3.0;
        let widths: Vec<f32> = galleys
            .iter()
            .map(|g| (g.size().x + 10.0).max(20.0))
            .collect();
        let total = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
        let (rect, response) = ui.allocate_exact_size(vec2(total, 20.0), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, self.keys.join("+")));
        let mut x = rect.left();
        for (galley, width) in galleys.into_iter().zip(widths) {
            let cap = egui::Rect::from_min_size(egui::pos2(x, rect.top()), vec2(width, 20.0));
            ui.painter().rect(
                cap,
                CornerRadius::same(radius::CONTROL),
                colors.frame,
                Stroke::new(1.0_f32, colors.border),
                StrokeKind::Inside,
            );
            common::text(
                ui.painter(),
                cap.center() - galley.size() / 2.0,
                galley,
                colors.text_secondary,
            );
            x += width + gap;
        }
        response
    }
}

/// A square badge with a name's initials on a color derived from the name, for projects,
/// users and workspaces.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::Avatar;
///
/// ui.add(Avatar::new("archipel-ide"));
/// ui.add(Avatar::new("Ada Lovelace").size(32.0).round(true));
/// # });
/// ```
#[must_use = "add it with `ui.add(avatar)`"]
pub struct Avatar {
    name: String,
    size: f32,
    round: bool,
}

impl Avatar {
    /// An avatar for `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: 20.0,
            round: false,
        }
    }

    /// Edge length in points (default 20).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Draws a circle instead of a rounded square (for people).
    pub fn round(mut self, round: bool) -> Self {
        self.round = round;
        self
    }
}

impl Widget for Avatar {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &self.name));
        let corner = if self.round {
            CornerRadius::same(radius::PILL)
        } else {
            CornerRadius::same((self.size * 0.25) as u8)
        };
        ui.painter()
            .rect_filled(rect, corner, avatar_color(&self.name));
        let galley = common::galley(
            ui,
            initials(&self.name).as_str(),
            typography::semibold(ui.ctx(), self.size * 0.45),
        );
        common::text(
            ui.painter(),
            rect.center() - galley.size() / 2.0,
            galley,
            Color32::WHITE,
        );
        response
    }
}

/// Initials of `name`: first letters of the first two words, else its first two letters.
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let initials: String = match words.as_slice() {
        [] => "?".into(),
        [single] => single.chars().take(2).collect(),
        [first, second, ..] => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
    };
    initials.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_follow_word_rules() {
        assert_eq!(initials("archipel"), "AR");
        assert_eq!(initials("my-cool_app"), "MC");
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("---"), "?");
        assert_eq!(initials("ärger"), "ÄR");
    }

    #[test]
    fn kbd_parses_plus_separated_keys() {
        assert_eq!(Kbd::parse("Ctrl+Shift+F").keys(), ["Ctrl", "Shift", "F"]);
        assert_eq!(Kbd::parse("Ctrl++").keys(), ["Ctrl", "+"]);
        assert_eq!(Kbd::parse("F5").keys(), ["F5"]);
    }
}
