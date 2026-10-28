use crate::{Avatar, IconKey, Tone, Tooltip};
use serde::{Deserialize, Serialize};

/// An icon button of a toolbar (IntelliJ `ActionButton`): the main toolbar, a tool window
/// header action, the vertical toolbar of a bottom window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolbarButton {
    /// Stable id; the event names it.
    pub id: String,
    /// The icon. A button without icon shows its `label`.
    pub icon: Option<IconKey>,
    /// Text next to (or instead of) the icon.
    pub label: Option<String>,
    /// The icon color role (green Run, red Stop).
    pub tone: Tone,
    /// Whether a click does anything. A disabled button is drawn in the disabled tone and
    /// shows its reason as tooltip.
    pub enabled: bool,
    /// Toggled on (a pressed toggle button).
    pub selected: bool,
    /// A dot in the corner, in this tone.
    pub badge: Option<Tone>,
    /// Hover text. Disabled buttons show it too (the reason).
    pub tooltip: Option<Tooltip>,
    /// The name for accessibility and tests; defaults to the tooltip title.
    pub name: String,
}

impl ToolbarButton {
    /// An enabled button with `icon` named `name`.
    pub fn new(id: impl Into<String>, icon: impl Into<IconKey>, name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: id.into(),
            icon: Some(icon.into()),
            label: None,
            tone: Tone::Default,
            enabled: true,
            selected: false,
            badge: None,
            tooltip: Some(Tooltip::new(name.clone())),
            name,
        }
    }

    /// Sets the tone.
    #[must_use]
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets whether it is enabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the toggled state.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets the tooltip.
    #[must_use]
    pub fn tooltip(mut self, tooltip: Tooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// A flat widget that opens a popup: the project widget, the VCS widget, the run
/// configuration widget, the device selector. Icon or avatar, a label, a chevron.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolbarDropdown {
    /// Stable id; a click reports it and the application opens the popup it stands for.
    pub id: String,
    /// The icon before the label.
    pub icon: Option<IconKey>,
    /// A colored initials badge before the label (the project widget).
    pub avatar: Option<Avatar>,
    /// The text.
    pub label: String,
    /// The text color role.
    pub tone: Tone,
    /// Whether the chevron follows the label.
    pub chevron: bool,
    /// The longest label in characters; a longer one is ellipsised by the renderer.
    pub max_chars: Option<u16>,
    /// Whether it can be clicked.
    pub enabled: bool,
    /// Hover text.
    pub tooltip: Option<Tooltip>,
    /// Characters to drop when the bar is narrow: 0 keeps the widget longest.
    pub shrink_order: u8,
}

impl ToolbarDropdown {
    /// A dropdown with a `label` and a chevron.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            icon: None,
            avatar: None,
            label: label.into(),
            tone: Tone::Strong,
            chevron: true,
            max_chars: None,
            enabled: true,
            tooltip: None,
            shrink_order: 0,
        }
    }

    /// Sets the icon.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<IconKey>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the avatar.
    #[must_use]
    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.avatar = Some(avatar);
        self
    }

    /// Sets the tooltip.
    #[must_use]
    pub fn tooltip(mut self, tooltip: Tooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// Sets the longest label.
    #[must_use]
    pub fn max_chars(mut self, max: u16) -> Self {
        self.max_chars = Some(max);
        self
    }
}

/// One element of a [`Toolbar`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolbarItem {
    /// An icon button.
    Button(ToolbarButton),
    /// A widget that opens a popup.
    Dropdown(ToolbarDropdown),
    /// A segmented control (the Code | Agents switch).
    Segmented {
        /// Stable id.
        id: String,
        /// The option labels.
        options: Vec<String>,
        /// The selected option.
        selected: usize,
    },
    /// A thin divider.
    Separator,
}

impl ToolbarItem {
    /// The id, when the item has one.
    pub fn id(&self) -> Option<&str> {
        match self {
            ToolbarItem::Button(button) => Some(&button.id),
            ToolbarItem::Dropdown(dropdown) => Some(&dropdown.id),
            ToolbarItem::Segmented { id, .. } => Some(id),
            ToolbarItem::Separator => None,
        }
    }
}

/// A toolbar: the items that sit at the start and the items that sit at the end (right
/// aligned) of one bar. The main toolbar is `leading` = hamburger, project, VCS, mode
/// switch and `trailing` = run configuration, Run, Build, Debug, Search Everywhere,
/// Settings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Toolbar {
    /// Stable id of the bar.
    pub id: String,
    /// Items from the start edge.
    pub leading: Vec<ToolbarItem>,
    /// Items from the end edge, in visual order left to right.
    pub trailing: Vec<ToolbarItem>,
}

impl Toolbar {
    /// An empty toolbar.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Self::default()
        }
    }

    /// All items in visual order.
    pub fn items(&self) -> impl Iterator<Item = &ToolbarItem> {
        self.leading.iter().chain(self.trailing.iter())
    }
}

/// What the user did on a toolbar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ToolbarEvent {
    /// A button or dropdown was clicked.
    Click {
        /// Its id.
        id: String,
    },
    /// A right click on a button or dropdown.
    ContextClick {
        /// Its id.
        id: String,
    },
    /// A segment was chosen.
    Select {
        /// The control's id.
        id: String,
        /// The option index.
        index: usize,
    },
}
