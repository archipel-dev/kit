//! Caller-owned tool window state: [`ToolWindow`], its [`Content`]s ([`ContentManager`]) and
//! the [`ToolWindowEvent`]s a [`ToolWindowPane`](crate::wm::ToolWindowPane) reports.

use crate::icons::Icon;
use crate::wm::ToolWindowAnchor;

/// One content (tab) of a tool window, like a run configuration's console or a "Problems"
/// view.
///
/// IntelliJ counterpart: `com.intellij.ui.content.Content` (display name, closeable, pinned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// A stable id, to find the content again after tabs came and went.
    pub id: String,
    /// The tab's title (IntelliJ `getDisplayName`).
    pub display_name: String,
    /// Whether the user can close it from its tab (IntelliJ `isCloseable`, default `true`).
    pub closeable: bool,
    /// A pinned content cannot be closed by the user (IntelliJ `isPinned`).
    pub pinned: bool,
}

impl Content {
    /// A closeable content.
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            closeable: true,
            pinned: false,
        }
    }

    /// Sets whether the tab can be closed.
    #[must_use]
    pub fn closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    /// Pins the content: it cannot be closed from its tab.
    #[must_use]
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    fn can_close(&self) -> bool {
        self.closeable && !self.pinned
    }
}

/// The contents of one tool window and which one is selected.
///
/// IntelliJ counterpart: `com.intellij.ui.content.ContentManager` (`addContent`,
/// `removeContent`, `setSelectedContent`, `getSelectedContent`). Owned by the caller; the
/// tool window's header shows the contents as tabs (when there is more than one, or when
/// [`ToolWindowPane::tool_window`](crate::wm::ToolWindowPane::tool_window) is given a
/// [`ToolWindow`] that has any) and writes the user's choice back into it.
///
/// ```
/// use archipel_kit::wm::{Content, ContentManager};
///
/// let mut contents = ContentManager::new();
/// contents.add_content(Content::new("file", "File"));
/// contents.add_content(Content::new("project", "Project Errors"));
/// assert_eq!(contents.selected_content().map(|c| c.display_name.as_str()), Some("File"));
/// contents.select_by_id("project");
/// assert_eq!(contents.selected_index(), Some(1));
/// contents.remove_content(1);
/// assert_eq!(contents.selected_index(), Some(0));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentManager {
    contents: Vec<Content>,
    selected: Option<usize>,
}

impl ContentManager {
    /// No contents.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `content` after the others. The first content becomes the selected one.
    pub fn add_content(&mut self, content: Content) {
        self.contents.push(content);
        if self.selected.is_none() {
            self.selected = Some(0);
        }
    }

    /// Removes the content at `index` and returns it. The selection moves to the content
    /// that took its place (or the one before, at the end).
    pub fn remove_content(&mut self, index: usize) -> Option<Content> {
        if index >= self.contents.len() {
            return None;
        }
        let removed = self.contents.remove(index);
        self.selected = match self.selected {
            _ if self.contents.is_empty() => None,
            Some(selected) if selected > index => Some(selected - 1),
            Some(selected) if selected == index => Some(index.min(self.contents.len() - 1)),
            other => other,
        };
        Some(removed)
    }

    /// The contents, in tab order.
    pub fn contents(&self) -> &[Content] {
        &self.contents
    }

    /// How many contents there are.
    pub fn len(&self) -> usize {
        self.contents.len()
    }

    /// Whether there are no contents.
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    /// The selected content's index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// The selected content (IntelliJ `getSelectedContent`).
    pub fn selected_content(&self) -> Option<&Content> {
        self.selected.and_then(|index| self.contents.get(index))
    }

    /// Selects the content at `index`; out of range does nothing.
    pub fn select_index(&mut self, index: usize) {
        if index < self.contents.len() {
            self.selected = Some(index);
        }
    }

    /// Selects the content with `id`; returns whether there is one.
    pub fn select_by_id(&mut self, id: &str) -> bool {
        match self.contents.iter().position(|content| content.id == id) {
            Some(index) => {
                self.selected = Some(index);
                true
            }
            None => false,
        }
    }

    /// The tab titles, the selected index (0 when empty) and which tabs can be closed.
    pub(crate) fn tabs(&self) -> (Vec<String>, usize, Vec<bool>) {
        (
            self.contents
                .iter()
                .map(|content| content.display_name.clone())
                .collect(),
            self.selected.unwrap_or(0),
            self.contents.iter().map(Content::can_close).collect(),
        )
    }
}

/// A tool window's description and contents: what its stripe button and header show.
///
/// IntelliJ counterpart: `com.intellij.openapi.wm.ToolWindow` (id, title, icon, anchor,
/// content manager). The caller owns it; whether it is shown is in the
/// [`ToolWindowPaneState`](crate::wm::ToolWindowPaneState), the way IntelliJ's
/// `ToolWindowManager` holds it.
///
/// ```
/// use archipel_kit::Glyph;
/// use archipel_kit::wm::{Content, ToolWindow, ToolWindowAnchor};
///
/// let mut problems = ToolWindow::new("Problems", "Problems", Glyph::Warning, ToolWindowAnchor::Bottom)
///     .activate_number(6);
/// problems.contents.add_content(Content::new("file", "File"));
/// assert_eq!(problems.activate_number, Some(6));
/// ```
#[derive(Debug, Clone)]
pub struct ToolWindow {
    /// A stable id (IntelliJ `getId`).
    pub id: String,
    /// The title shown in the header and as the stripe button's name.
    pub title: String,
    /// The stripe button's icon.
    pub icon: Icon,
    /// Where it docks.
    pub anchor: ToolWindowAnchor,
    /// The digit of its `Alt+digit` shortcut (IntelliJ `ActivateXToolWindow`), shown in the
    /// stripe button's tooltip.
    pub activate_number: Option<u8>,
    /// The tabs of its header.
    pub contents: ContentManager,
}

impl ToolWindow {
    /// A tool window with no contents.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        icon: impl Into<Icon>,
        anchor: ToolWindowAnchor,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: icon.into(),
            anchor,
            activate_number: None,
            contents: ContentManager::new(),
        }
    }

    /// Binds `Alt+digit` (1 to 9) to activating it.
    #[must_use]
    pub fn activate_number(mut self, digit: u8) -> Self {
        self.activate_number = Some(digit.clamp(1, 9));
        self
    }
}

/// What the tool windows of a [`ToolWindowPane`](crate::wm::ToolWindowPane) report this
/// frame, for the application to act on (the pane applies the layout changes itself).
///
/// Read them with
/// [`ToolWindowPaneState::take_events`](crate::wm::ToolWindowPaneState::take_events).
/// IntelliJ's counterparts are the `ToolWindowManager` calls the same gestures make:
/// `activate`, `hide`, `hideAll`, `focusEditor`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ToolWindowEvent {
    /// The user asked for the tool window to become the active one: a click on its header,
    /// `Alt+digit` on a visible or hidden window, or a stripe button of a hidden one. Move
    /// the keyboard focus into it.
    Activated(ToolWindowAnchor),
    /// The tool window was hidden (close click on its header or stripe button, `Shift+Esc`,
    /// `Alt+digit` on the active window, the Hide button).
    Hidden(ToolWindowAnchor),
    /// `Ctrl+Shift+F12` hid every tool window.
    AllHidden,
    /// `Esc` in a tool window whose content did not use it: move the focus back to the
    /// editor (IntelliJ `FocusEditor`).
    FocusEditor(ToolWindowAnchor),
    /// The tool window was maximized or restored (double click on its header,
    /// `Ctrl+Shift+'`, the Maximize button); `true` when it is maximized now.
    MaximizeToggled(ToolWindowAnchor, bool),
    /// The user closed the content tab at that index of the tool window (it is not removed
    /// from the [`ContentManager`] unless the pane was given the [`ToolWindow`]).
    ContentCloseRequested(ToolWindowAnchor, usize),
    /// A popup trigger on the tool window's header: show the tool window popup (place
    /// [`ActionPlace::TOOLWINDOW_POPUP`](crate::action_system::ActionPlace::TOOLWINDOW_POPUP))
    /// at that point, unless the header already opened its own options group.
    HeaderMenuRequested(ToolWindowAnchor, egui::Pos2),
    /// A popup trigger on the tool window's stripe button: show its menu (move to, resize,
    /// view mode) at that point.
    StripeMenuRequested(ToolWindowAnchor, egui::Pos2),
    /// A stripe button was dragged and dropped on another stripe group: the application
    /// moves the tool window to the `to` anchor.
    MovedToAnchor {
        /// The tool window that was dragged.
        window: ToolWindowAnchor,
        /// The stripe group it was dropped on.
        to: ToolWindowAnchor,
    },
}
