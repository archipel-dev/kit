//! Semantic colors: what a color means, resolved from the theme.

use crate::theme::Colors;
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
