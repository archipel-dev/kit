use crate::Shortcut;
use serde::{Deserialize, Serialize};

/// A semantic color role. The renderer maps it to the theme's color of that name, so a
/// props value never carries a color of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Tone {
    /// The default text or icon color.
    #[default]
    Default,
    /// Titles and selected text.
    Strong,
    /// Secondary labels: status bar widgets, row details.
    Secondary,
    /// Hints and info text.
    Info,
    /// Disabled text.
    Disabled,
    /// The accent color.
    Accent,
    /// Run actions (the green play icon).
    Run,
    /// Stop actions (the red stop icon).
    Stop,
    /// Errors.
    Error,
    /// Warnings.
    Warning,
    /// Success.
    Success,
    /// Modified (unsaved or version control).
    Modified,
    /// Hyperlinks.
    Link,
}

/// A tooltip: a title with an optional shortcut and description. The renderer shows it
/// after the hover delay of the GUI (500 ms) and formats the shortcut for its platform.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Tooltip {
    /// The first line, for example `Run 'Main'`.
    pub title: String,
    /// The shortcut shown after the title in parentheses.
    pub shortcut: Option<Shortcut>,
    /// A second, dimmer line.
    pub description: Option<String>,
}

impl Tooltip {
    /// A tooltip with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            shortcut: None,
            description: None,
        }
    }

    /// Adds the shortcut.
    #[must_use]
    pub fn shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Adds the shortcut when there is one.
    #[must_use]
    pub fn shortcut_opt(mut self, shortcut: Option<Shortcut>) -> Self {
        self.shortcut = shortcut;
        self
    }

    /// Adds the description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The one-line text: `Title (Shortcut)` in `notation`.
    pub fn text(&self, notation: crate::Notation) -> String {
        match &self.shortcut {
            Some(shortcut) => format!("{} ({})", self.title, shortcut.label(notation)),
            None => self.title.clone(),
        }
    }
}
