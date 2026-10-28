use crate::{Breadcrumbs, IconKey, Tone, Tooltip};
use serde::{Deserialize, Serialize};

/// One widget of the status bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusBarItem {
    /// Stable id.
    pub id: String,
    /// An icon before the text.
    pub icon: Option<IconKey>,
    /// The text.
    pub text: String,
    /// The text color role.
    pub tone: Tone,
    /// Hover text.
    pub tooltip: Option<Tooltip>,
    /// Whether a click does anything.
    pub clickable: bool,
    /// Which items stay when the bar is narrow: the lowest priority goes first.
    pub priority: u8,
    /// A busy spinner before the text.
    pub spinner: bool,
    /// A progress bar before the text, 0.0 to 1.0.
    pub progress: Option<f32>,
    /// The rows of the widget's menu (a click, or a right click in the GUI, opens it): its
    /// actions and details. Empty when the widget has no menu.
    pub menu: Vec<crate::MenuEntry>,
    /// The text may be cut with an ellipsis to the room that is left (a status message).
    pub elide: bool,
}

impl StatusBarItem {
    /// A clickable secondary-tone item.
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            icon: None,
            text: text.into(),
            tone: Tone::Secondary,
            tooltip: None,
            clickable: true,
            priority: 50,
            spinner: false,
            progress: None,
            elide: false,
            menu: Vec::new(),
        }
    }
}

/// The status bar: breadcrumbs at the left, the widgets at the right.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StatusBar {
    /// The path to the caret.
    pub breadcrumbs: Breadcrumbs,
    /// The widgets in visual order left to right.
    pub items: Vec<StatusBarItem>,
}

/// What the user did on the status bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StatusBarEvent {
    /// A widget was clicked.
    Click {
        /// The item's id.
        id: String,
    },
    /// A breadcrumb was clicked.
    Crumb {
        /// The crumb's id.
        id: String,
    },
}
