//! What sits inside a text field besides its text: extension icons
//! (`ExtendableTextField.Extension`) and the history of a search field.

use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{radius, size};
use egui::{Id, Rect, Response, Sense, Ui, WidgetInfo, WidgetType};

/// An icon button inside a text field: IntelliJ's `ExtendableTextField.Extension`.
///
/// Add extensions with [`JBTextField::extension`](crate::ui::JBTextField::extension); the
/// field reports the one that was activated in
/// [`JBTextFieldOutput::extension_clicked`](crate::ui::JBTextFieldOutput::extension_clicked).
/// The kit draws and focuses them; what a click does is up to the caller.
///
/// | | |
/// |---|---|
/// | Position | After the text by default, [before it](Self::icon_before_text) on request. |
/// | Hover | Swaps to the [hovered icon](Self::hovered_icon) and turns the pointer into a hand; over the text it is the text cursor. |
/// | Click | Reports the extension; so do <kbd>Space</kbd> and <kbd>Enter</kbd> on the focused icon. |
/// | Focus | [Focusable](Self::focusable) (reachable with <kbd>Tab</kbd>), on by default. |
/// | Tooltip | The [tooltip](Self::tooltip) after the help tooltip delay. |
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::Glyph;
/// use archipel_kit::ui::{Extension, JBTextField};
///
/// let mut path = String::new();
/// let out = JBTextField::new(&mut path)
///     .name("Path")
///     .extension(Extension::new(Glyph::Folder, "Browse").tooltip("Browse (Shift+Enter)"))
///     .show(ui);
/// if out.extension_clicked == Some(0) { /* open a file chooser */ }
/// # });
/// ```
#[derive(Debug, Clone)]
#[must_use = "add it with `JBTextField::extension`"]
pub struct Extension {
    pub(crate) icon: Icon,
    pub(crate) hovered_icon: Option<Icon>,
    pub(crate) name: String,
    pub(crate) tooltip: Option<String>,
    pub(crate) before_text: bool,
    pub(crate) focusable: bool,
    pub(crate) enabled: bool,
}

impl Extension {
    /// An extension drawn with `icon`; `name` is its accessible name.
    pub fn new(icon: impl Into<Icon>, name: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            hovered_icon: None,
            name: name.into(),
            tooltip: None,
            before_text: false,
            focusable: true,
            enabled: true,
        }
    }

    /// The icon shown while the pointer is over the extension (IntelliJ `getIcon(hovered)`).
    pub fn hovered_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.hovered_icon = Some(icon.into());
        self
    }

    /// Tooltip text (IntelliJ `getTooltip`).
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Puts the icon before the text instead of after it (IntelliJ `isIconBeforeText`).
    pub fn icon_before_text(mut self, before: bool) -> Self {
        self.before_text = before;
        self
    }

    /// Whether <kbd>Tab</kbd> reaches the icon (registry `text.field.extension.buttons.
    /// focusable`, default true).
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// A disabled extension is dimmed and ignores clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Draws one extension in `rect` and reports it: hover icon, hand cursor, focus ring,
/// tooltip, accessible name.
pub(crate) fn extension_button(
    ui: &Ui,
    rect: Rect,
    id: Id,
    extension: &Extension,
    field_enabled: bool,
) -> Response {
    let enabled = field_enabled && extension.enabled;
    let sense = match (enabled, extension.focusable) {
        (false, _) => Sense::hover(),
        (true, true) => Sense::click(),
        (true, false) => Sense::CLICK,
    };
    let response = ui.interact(rect, id, sense);
    response
        .widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, extension.name.as_str()));
    // A hand over the icon; the field's text cursor is only set elsewhere.
    crate::cursor::on_hover_action(&response);
    let colors = Theme::colors(ui.ctx());
    let hovered = response.hovered() && enabled;
    let icon = match (&extension.hovered_icon, hovered) {
        (Some(icon), true) => *icon,
        _ => extension.icon,
    };
    let color = if !enabled {
        colors.text_disabled
    } else if hovered {
        colors.text
    } else {
        colors.icon
    };
    if response.has_focus() {
        crate::parts::focus_ring(ui.painter(), rect, radius::CONTROL, &colors);
    }
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), egui::vec2(size::ICON, size::ICON)),
        icon,
        color,
    );
    if let Some(tooltip) = &extension.tooltip {
        crate::ide::HelpTooltip::new(tooltip.clone()).show_for(&response);
    }
    response
}

/// The remembered entries of a [`SearchTextField`](crate::ui::SearchTextField), newest
/// first. The kit never saves them: keep the value in your state and persist it yourself
/// (IntelliJ saves under a property name).
///
/// ```
/// use archipel_kit::ui::SearchHistory;
///
/// let mut history = SearchHistory::new();
/// history.add("main");
/// history.add("dev");
/// history.add("main");
/// assert_eq!(history.entries(), ["main", "dev"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHistory {
    entries: Vec<String>,
    capacity: usize,
}

impl Default for SearchHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchHistory {
    /// An empty history keeping the default five entries
    /// ([`HISTORY_LENGTH`](size::SEARCH_HISTORY_LENGTH)).
    pub fn new() -> Self {
        Self::with_capacity(size::SEARCH_HISTORY_LENGTH)
    }

    /// An empty history keeping `capacity` entries (at least one).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    /// Remembers `entry` as the newest. An equal older entry moves to the front, the oldest
    /// falls off when the history is full, and blank text is ignored.
    pub fn add(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        if entry.trim().is_empty() {
            return;
        }
        self.entries.retain(|e| *e != entry);
        self.entries.insert(0, entry);
        self.entries.truncate(self.capacity);
    }

    /// The entries, newest first.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Forgets every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Whether there is nothing to show.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
