use crate::{IconKey, Tone, Tooltip};
use serde::{Deserialize, Serialize};

/// One crumb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crumb {
    /// Stable id.
    pub id: String,
    /// The text.
    pub text: String,
    /// An icon before the text.
    pub icon: Option<IconKey>,
    /// The text color role.
    pub tone: Tone,
    /// Hover text.
    pub tooltip: Option<Tooltip>,
    /// Whether a click does anything.
    pub clickable: bool,
}

impl Crumb {
    /// A secondary-tone crumb.
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            icon: None,
            tone: Tone::Secondary,
            tooltip: None,
            clickable: true,
        }
    }
}

/// The path to the caret: project, directories, file, then the declarations around the
/// caret. The renderer draws the separators and, when space runs out, shortens from the
/// left but never drops the last crumb.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Breadcrumbs {
    /// The crumbs in order.
    pub crumbs: Vec<Crumb>,
}

/// What the user did on breadcrumbs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BreadcrumbEvent {
    /// A crumb was clicked.
    Click {
        /// The crumb's id.
        id: String,
    },
}
