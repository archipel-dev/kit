//! Semantic colors: what a color means, resolved from the theme.

use crate::theme::{Colors, Theme};
use egui::Color32;

/// What a text or icon color means. Components take a `Tone`, never a raw color, so every
/// tool says "error" in the same red and every theme stays consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tone {
    /// Normal text.
    #[default]
    Default,
    /// Emphasized text: titles, selected rows.
    Strong,
    /// Secondary text: details, captions.
    Secondary,
    /// The faintest readable text: placeholders, hints, timestamps.
    Muted,
    /// Disabled controls.
    Disabled,
    /// The accent: active states, primary actions.
    Accent,
    /// Links and inline actions.
    Link,
    /// Success, passed, added.
    Success,
    /// Warnings.
    Warning,
    /// Errors, failures.
    Error,
    /// Run actions (green play).
    Run,
    /// Stop actions (red stop).
    Stop,
    /// Version control: modified.
    Modified,
    /// Version control: added.
    Added,
    /// Version control: untracked.
    Untracked,
    /// Version control: ignored.
    Ignored,
    /// Icons in their normal gray.
    Icon,
}

impl Tone {
    /// Every tone, for tests and pickers.
    pub const ALL: [Tone; 17] = [
        Tone::Default,
        Tone::Strong,
        Tone::Secondary,
        Tone::Muted,
        Tone::Disabled,
        Tone::Accent,
        Tone::Link,
        Tone::Success,
        Tone::Warning,
        Tone::Error,
        Tone::Run,
        Tone::Stop,
        Tone::Modified,
        Tone::Added,
        Tone::Untracked,
        Tone::Ignored,
        Tone::Icon,
    ];

    /// The color of this tone in `colors`.
    pub fn color(self, colors: &Colors) -> Color32 {
        match self {
            Tone::Default => colors.text,
            Tone::Strong => colors.text_strong,
            Tone::Secondary => colors.text_secondary,
            Tone::Muted => colors.text_placeholder,
            Tone::Disabled => colors.text_disabled,
            Tone::Accent => colors.accent,
            Tone::Link => colors.link,
            Tone::Success => colors.success,
            Tone::Warning => colors.warning,
            Tone::Error => colors.error,
            Tone::Run => colors.run,
            Tone::Stop => colors.stop,
            Tone::Modified => colors.vcs_modified,
            Tone::Added => colors.vcs_added,
            Tone::Untracked => colors.vcs_untracked,
            Tone::Ignored => colors.vcs_ignored,
            Tone::Icon => colors.icon,
        }
    }

    /// The color of this tone in the theme of `ctx`.
    pub fn resolve(self, ctx: &egui::Context) -> Color32 {
        self.color(&crate::Theme::colors(ctx))
    }

    /// Tones that carry information and must stay readable on the island (everything but
    /// [`Tone::Disabled`], which is dim on purpose).
    pub fn is_readable(self) -> bool {
        self != Tone::Disabled
    }
}

impl Tone {
    /// The tone of an HTTP method: GET green, POST amber, PUT blue, PATCH accent, DELETE red,
    /// HEAD and OPTIONS secondary. Case-insensitive; unknown methods are [`Tone::Default`].
    pub fn method(method: &str) -> Tone {
        match method.to_ascii_uppercase().as_str() {
            "GET" => Tone::Success,
            "POST" => Tone::Warning,
            "PUT" => Tone::Link,
            "PATCH" => Tone::Accent,
            "DELETE" => Tone::Error,
            "HEAD" | "OPTIONS" => Tone::Secondary,
            _ => Tone::Default,
        }
    }

    /// The tone of an HTTP status code: 1xx secondary, 2xx success, 3xx link, 4xx warning,
    /// 5xx error. Anything else (0, 600+) is [`Tone::Muted`], e.g. "no response".
    pub fn status(code: u16) -> Tone {
        match code {
            100..=199 => Tone::Secondary,
            200..=299 => Tone::Success,
            300..=399 => Tone::Link,
            400..=499 => Tone::Warning,
            500..=599 => Tone::Error,
            _ => Tone::Muted,
        }
    }
}

/// The type of a value in a data view (JSON tree, document table, response viewer). Its color
/// comes from the editor syntax colors of the theme, so values look like they do in the code
/// editor. See `docs/theming.md` for the mapping.
///
/// This is separate from [`Tone`] because the colors live in `EditorColors`, not `Colors`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// Text.
    String,
    /// Integers and floats.
    Number,
    /// `true` and `false`.
    Boolean,
    /// `null` and missing values.
    Null,
    /// Dates and timestamps.
    Date,
    /// Object ids and other identifiers.
    Id,
    /// Binary data (BinData, bytes).
    Binary,
    /// Keys and property names.
    Key,
    /// Braces, brackets, commas and colons.
    Punctuation,
}

impl ValueKind {
    /// Every kind, for tests and legends.
    pub const ALL: [ValueKind; 9] = [
        ValueKind::String,
        ValueKind::Number,
        ValueKind::Boolean,
        ValueKind::Null,
        ValueKind::Date,
        ValueKind::Id,
        ValueKind::Binary,
        ValueKind::Key,
        ValueKind::Punctuation,
    ];

    /// The color of this kind in `theme`.
    pub fn color(self, theme: &Theme) -> Color32 {
        let editor = &theme.editor;
        match self {
            ValueKind::String => editor.string,
            ValueKind::Number => editor.number,
            ValueKind::Boolean => editor.keyword,
            ValueKind::Null => editor.comment,
            ValueKind::Date => editor.constant,
            ValueKind::Id => editor.type_name,
            ValueKind::Binary => editor.annotation,
            ValueKind::Key => editor.property,
            ValueKind::Punctuation => editor.punctuation,
        }
    }

    /// The color of this kind in the theme of `ctx`.
    pub fn resolve(self, ctx: &egui::Context) -> Color32 {
        self.color(&Theme::current(ctx))
    }

    /// Display name, e.g. for legends.
    pub fn label(self) -> &'static str {
        match self {
            ValueKind::String => "String",
            ValueKind::Number => "Number",
            ValueKind::Boolean => "Boolean",
            ValueKind::Null => "Null",
            ValueKind::Date => "Date",
            ValueKind::Id => "ObjectId",
            ValueKind::Binary => "Binary",
            ValueKind::Key => "Key",
            ValueKind::Punctuation => "Punctuation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_method_tones() {
        assert_eq!(Tone::status(200), Tone::Success);
        assert_eq!(Tone::status(404), Tone::Warning);
        assert_eq!(Tone::status(503), Tone::Error);
        assert_eq!(Tone::status(0), Tone::Muted);
        assert_eq!(Tone::method("get"), Tone::Success);
        assert_eq!(Tone::method("DELETE"), Tone::Error);
        assert_eq!(Tone::method("PROPFIND"), Tone::Default);
    }

    #[test]
    fn value_kinds_are_distinct_per_theme() {
        for theme in [Theme::dark(), Theme::light()] {
            let colors: std::collections::HashSet<_> =
                ValueKind::ALL.iter().map(|k| k.color(&theme)).collect();
            assert_eq!(colors.len(), ValueKind::ALL.len(), "{}", theme.name);
        }
    }
}
