//! The data of a notification: its type, group and actions.

use crate::ui::Status;

/// What a notification is about: IntelliJ's `NotificationType`. It picks the icon and tint
/// (as a [`Status`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationType {
    /// Neutral information (`INFORMATION`).
    Information,
    /// Something needs attention (`WARNING`).
    Warning,
    /// Something failed (`ERROR`).
    Error,
    /// An update of the IDE or of a plugin is available (`IDE_UPDATE`): the promo style.
    IdeUpdate,
}

impl From<NotificationType> for Status {
    fn from(kind: NotificationType) -> Self {
        match kind {
            NotificationType::Information => Status::Info,
            NotificationType::Warning => Status::Warning,
            NotificationType::Error => Status::Error,
            NotificationType::IdeUpdate => Status::Promo,
        }
    }
}

/// How a notification of a group is shown: IntelliJ's `NotificationDisplayType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NotificationDisplayType {
    /// A balloon that hides by itself after 10 s.
    #[default]
    Balloon,
    /// A balloon that stays up to 300 s unless the user closes it.
    StickyBalloon,
    /// A balloon pointing at a tool window's stripe button. It times out like a regular
    /// balloon; where it is anchored is up to the caller (see
    /// [`NotificationGroup::tool_window_id`]).
    ToolWindow,
    /// Never shown: the notification only goes to the history.
    None,
}

/// The kind of a notification and how its notifications are shown: IntelliJ's
/// `NotificationGroup`, identified by its display id.
///
/// ```
/// use archipel_kit::notification::{NotificationDisplayType, NotificationGroup};
///
/// let group = NotificationGroup::sticky_balloon_group("Build");
/// assert_eq!(group.display_id(), "Build");
/// assert_eq!(group.display_type(), NotificationDisplayType::StickyBalloon);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotificationGroup {
    display_id: String,
    display_type: NotificationDisplayType,
    tool_window_id: Option<String>,
}

impl NotificationGroup {
    /// A group with an explicit display type.
    pub fn new(display_id: impl Into<String>, display_type: NotificationDisplayType) -> Self {
        Self {
            display_id: display_id.into(),
            display_type,
            tool_window_id: None,
        }
    }

    /// A group of balloons that hide by themselves (`NotificationGroup.balloonGroup`).
    pub fn balloon_group(display_id: impl Into<String>) -> Self {
        Self::new(display_id, NotificationDisplayType::Balloon)
    }

    /// A group of balloons that stay until the user closes them.
    pub fn sticky_balloon_group(display_id: impl Into<String>) -> Self {
        Self::new(display_id, NotificationDisplayType::StickyBalloon)
    }

    /// A group that is only logged (`NotificationGroup.logOnlyGroup`).
    pub fn log_only_group(display_id: impl Into<String>) -> Self {
        Self::new(display_id, NotificationDisplayType::None)
    }

    /// A group of balloons for the tool window `tool_window_id`
    /// (`NotificationGroup.toolWindowGroup`).
    pub fn tool_window_group(
        display_id: impl Into<String>,
        tool_window_id: impl Into<String>,
    ) -> Self {
        let mut group = Self::new(display_id, NotificationDisplayType::ToolWindow);
        group.tool_window_id = Some(tool_window_id.into());
        group
    }

    /// The id that names the group in settings and in the history.
    pub fn display_id(&self) -> &str {
        &self.display_id
    }

    /// How the group's notifications are shown.
    pub fn display_type(&self) -> NotificationDisplayType {
        self.display_type
    }

    /// The tool window a [`NotificationDisplayType::ToolWindow`] group points at.
    pub fn tool_window_id(&self) -> Option<&str> {
        self.tool_window_id.as_deref()
    }
}

impl Default for NotificationGroup {
    /// An unnamed balloon group.
    fn default() -> Self {
        Self::balloon_group("")
    }
}

/// An action a notification offers, shown as a link: IntelliJ's `NotificationAction`.
///
/// Performing an action expires its notification, unless the action
/// [keeps it](NotificationAction::keep_notification).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotificationAction {
    pub(crate) text: String,
    pub(crate) keep: bool,
}

impl NotificationAction {
    /// An action reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            keep: false,
        }
    }

    /// Whether performing the action leaves the notification open (default: it expires).
    pub fn keep_notification(mut self, keep: bool) -> Self {
        self.keep = keep;
        self
    }

    /// The link text.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl<T: Into<String>> From<T> for NotificationAction {
    fn from(text: T) -> Self {
        Self::new(text)
    }
}
