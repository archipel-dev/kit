use crate::{IconKey, Tooltip};
use serde::{Deserialize, Serialize};

/// One tab of the editor strip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    /// Stable id (the document's path).
    pub id: String,
    /// The file name.
    pub title: String,
    /// The file icon.
    pub icon: IconKey,
    /// Whether it is the selected tab.
    pub selected: bool,
    /// Unsaved changes: a dot instead of the close button.
    pub dirty: bool,
    /// Pinned: a pin instead of the close button.
    pub pinned: bool,
    /// A preview tab: the title is italic.
    pub preview: bool,
    /// The file has errors: a red curly underline under the title.
    pub error: bool,
    /// Hover text (the full path).
    pub tooltip: Option<Tooltip>,
}

impl Tab {
    /// A plain tab.
    pub fn new(id: impl Into<String>, title: impl Into<String>, icon: impl Into<IconKey>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: icon.into(),
            selected: false,
            dirty: false,
            pinned: false,
            preview: false,
            error: false,
            tooltip: None,
        }
    }
}

/// A strip of tabs.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabBar {
    /// Stable id.
    pub id: String,
    /// The tabs in order.
    pub tabs: Vec<Tab>,
}

impl TabBar {
    /// A strip of `tabs`.
    pub fn new(id: impl Into<String>, tabs: Vec<Tab>) -> Self {
        Self {
            id: id.into(),
            tabs,
        }
    }
}

/// What the user did on a tab strip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TabEvent {
    /// A tab was selected (on press).
    Select {
        /// The tab's id.
        id: String,
    },
    /// A tab's close button was clicked, or the tab was middle clicked.
    Close {
        /// The tab's id.
        id: String,
    },
    /// A pinned tab's pin was clicked: unpin.
    Unpin {
        /// The tab's id.
        id: String,
    },
    /// A right click: open the tab menu.
    Context {
        /// The tab's id.
        id: String,
    },
    /// A tab was dragged to a new place.
    Reorder {
        /// The old index.
        from: usize,
        /// The new index.
        to: usize,
    },
    /// The overflow button was clicked: list all tabs.
    Overflow,
}
