use crate::{IconKey, Tone, Tooltip};
use serde::{Deserialize, Serialize};

/// A tool window button of a stripe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StripeButton {
    /// Stable id (the tool window's id).
    pub id: String,
    /// The icon.
    pub icon: IconKey,
    /// The hover text: the title and the `Alt+digit` shortcut.
    pub tooltip: Tooltip,
    /// Whether the window is shown (the accent fill).
    pub selected: bool,
    /// Whether the window has keyboard focus (a visible window without focus is neutral).
    pub focused: bool,
    /// A dot in the corner, in this tone: errors, a running build, agents.
    pub badge: Option<Tone>,
}

impl StripeButton {
    /// A hidden window's button.
    pub fn new(id: impl Into<String>, icon: impl Into<IconKey>, tooltip: Tooltip) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            tooltip,
            selected: false,
            focused: false,
            badge: None,
        }
    }
}

/// The edge a stripe sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StripeAnchor {
    /// The left edge.
    Left,
    /// The right edge.
    Right,
}

/// A stripe: the buttons of the windows docked at one edge, in a top group and a bottom
/// group with a separator between them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolStripe {
    /// The edge.
    pub anchor: StripeAnchor,
    /// The group from the top, in order.
    pub top: Vec<StripeButton>,
    /// The group at the bottom, in order from the bottom edge upward.
    pub bottom: Vec<StripeButton>,
}

impl ToolStripe {
    /// A stripe with no buttons.
    pub fn new(anchor: StripeAnchor) -> Self {
        Self {
            anchor,
            top: Vec::new(),
            bottom: Vec::new(),
        }
    }

    /// All buttons, top group first.
    pub fn buttons(&self) -> impl Iterator<Item = &StripeButton> {
        self.top.iter().chain(self.bottom.iter())
    }
}

/// What the user did on a stripe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StripeEvent {
    /// A click: show the window, or hide it when it is shown.
    Toggle {
        /// The button's id.
        id: String,
    },
    /// A middle click or Shift click: hide the window.
    Hide {
        /// The button's id.
        id: String,
    },
    /// A right click: open the stripe menu (View Mode, Move to, Hide).
    Context {
        /// The button's id.
        id: String,
    },
}
