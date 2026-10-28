use crate::ToolbarButton;
use serde::{Deserialize, Serialize};

/// A content tab in a tool window header (`Problems  File 1  Project Errors 3`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelTab {
    /// Stable id.
    pub id: String,
    /// The title.
    pub title: String,
    /// A count after the title, in a secondary tone.
    pub count: Option<u32>,
    /// Whether it is the selected tab.
    pub selected: bool,
    /// Whether it has a close button.
    pub closable: bool,
}

/// The header of a tool window or panel: the title, an optional view selector chevron,
/// content tabs, action buttons at the right and the Hide button last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelHeader {
    /// Stable id (the tool window id).
    pub id: String,
    /// The title.
    pub title: String,
    /// A chevron after the title: the title is a view selector (Project).
    pub view_selector: bool,
    /// Content tabs after the title.
    pub tabs: Vec<PanelTab>,
    /// Action buttons, right aligned, in visual order left to right.
    pub actions: Vec<ToolbarButton>,
    /// Whether the last button is Hide (`─`).
    pub hide: bool,
    /// Whether the window has keyboard focus (a brighter title).
    pub focused: bool,
}

impl PanelHeader {
    /// A header with a title and a Hide button.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            view_selector: false,
            tabs: Vec::new(),
            actions: Vec::new(),
            hide: true,
            focused: false,
        }
    }
}

/// What the user did on a header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PanelHeaderEvent {
    /// An action button was clicked.
    Action {
        /// The button's id.
        id: String,
    },
    /// The Hide button was clicked.
    Hide,
    /// The title (view selector) was clicked.
    Title,
    /// A tab was selected.
    SelectTab {
        /// The tab's id.
        id: String,
    },
    /// A tab's close button was clicked, or the tab was middle clicked.
    CloseTab {
        /// The tab's id.
        id: String,
    },
    /// The header was double clicked: maximize.
    Maximize,
}
