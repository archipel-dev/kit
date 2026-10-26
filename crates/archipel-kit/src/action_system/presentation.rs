//! The render state of an action: [`Presentation`].

use crate::Icon;

/// What an action looks like in one place and frame: text, icon, enabled, visible, toggle
/// state.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.Presentation`. In IntelliJ it is
/// a mutable, observable property bag owned by each button or menu item, with change
/// listeners that repaint. Here it is a plain value: [`update_presentation`] computes a fresh
/// one per frame from the action's template and its `update`, and the renderer takes it by
/// value. There are no listeners and no client properties.
///
/// The default is enabled and visible with no text, like a fresh IntelliJ presentation.
///
/// ```
/// use archipel_kit::action_system::Presentation;
///
/// let presentation = Presentation::new("Reload").description("Reload from disk").enabled(false);
/// assert_eq!(presentation.text, "Reload");
/// assert!(!presentation.enabled && presentation.visible);
/// ```
///
/// [`update_presentation`]: crate::action_system::update_presentation
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Presentation {
    /// The label (IntelliJ `Presentation.getText`). Mnemonics are not modelled.
    pub text: String,
    /// The longer explanation, shown in tooltips and the status bar (IntelliJ
    /// `Presentation.getDescription`).
    pub description: String,
    /// The icon (IntelliJ `Presentation.getIcon`). Disabled, hovered and selected variants
    /// are the renderer's job.
    pub icon: Option<Icon>,
    /// Whether the action can be performed (IntelliJ `Presentation.isEnabled`).
    pub enabled: bool,
    /// Whether the action is shown at all (IntelliJ `Presentation.isVisible`).
    pub visible: bool,
    /// `Some` for toggles: whether it is on (IntelliJ `Toggleable.isSelected`, the
    /// `"selected"` property). `None` for ordinary actions.
    pub selected: Option<bool>,
    /// A group that opens as a submenu or drop-down instead of flattening into its parent
    /// (IntelliJ `Presentation.isPopupGroup`).
    pub popup_group: bool,
    /// A popup group that can also be performed itself, like a split button (IntelliJ
    /// `Presentation.isPerformGroup`).
    pub perform_group: bool,
    /// A group with no visible children is hidden (IntelliJ
    /// `Presentation.isHideGroupIfEmpty`).
    pub hide_group_if_empty: bool,
    /// A group with no visible children is shown disabled (IntelliJ
    /// `Presentation.isDisableGroupIfEmpty`).
    pub disable_group_if_empty: bool,
    /// Whether a popup stays open after the action runs (IntelliJ
    /// `Presentation.getKeepPopupOnPerform`, reduced to a flag).
    pub keep_popup_on_perform: bool,
    /// The shortcut as text, for menus and tooltips. Filled in from a keymap by
    /// [`flatten_with_shortcuts`](crate::action_system::flatten_with_shortcuts) or by the
    /// action itself (IntelliJ shows `KeymapUtil.getFirstKeyboardShortcutText`).
    pub shortcut_text: Option<String>,
    /// Why the action is disabled, for a tooltip. No IntelliJ counterpart; IntelliJ puts
    /// this in the description.
    pub disabled_reason: Option<String>,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            text: String::new(),
            description: String::new(),
            icon: None,
            enabled: true,
            visible: true,
            selected: None,
            popup_group: false,
            perform_group: false,
            hide_group_if_empty: false,
            disable_group_if_empty: false,
            keep_popup_on_perform: false,
            shortcut_text: None,
            disabled_reason: None,
        }
    }
}

impl Presentation {
    /// An enabled, visible presentation with `text` (IntelliJ `Presentation(String text)`).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Sets the text (IntelliJ `Presentation.setText`).
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the description (IntelliJ `Presentation.setDescription`).
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Sets the icon (IntelliJ `Presentation.setIcon`).
    #[must_use]
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets enabled (IntelliJ `Presentation.setEnabled`).
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets visible (IntelliJ `Presentation.setVisible`).
    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets enabled and visible together (IntelliJ `Presentation.setEnabledAndVisible`),
    /// the usual result of an `update` that checks one condition.
    #[must_use]
    pub fn enabled_and_visible(mut self, on: bool) -> Self {
        self.enabled = on;
        self.visible = on;
        self
    }

    /// Makes this a toggle with the given state (IntelliJ `Toggleable.setSelected`).
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Marks a group as a popup (IntelliJ `Presentation.setPopupGroup`).
    #[must_use]
    pub fn popup_group(mut self, popup: bool) -> Self {
        self.popup_group = popup;
        self
    }

    /// Lets a popup group be performed itself (IntelliJ `Presentation.setPerformGroup`).
    #[must_use]
    pub fn perform_group(mut self, perform: bool) -> Self {
        self.perform_group = perform;
        self
    }

    /// Hides a group without visible children (IntelliJ `setHideGroupIfEmpty`).
    #[must_use]
    pub fn hide_group_if_empty(mut self, hide: bool) -> Self {
        self.hide_group_if_empty = hide;
        self
    }

    /// Disables a group without visible children (IntelliJ `setDisableGroupIfEmpty`).
    #[must_use]
    pub fn disable_group_if_empty(mut self, disable: bool) -> Self {
        self.disable_group_if_empty = disable;
        self
    }

    /// Keeps the popup open after the action runs (IntelliJ `setKeepPopupOnPerform`).
    #[must_use]
    pub fn keep_popup_on_perform(mut self, keep: bool) -> Self {
        self.keep_popup_on_perform = keep;
        self
    }

    /// Sets the shortcut text (IntelliJ shows it from the keymap).
    #[must_use]
    pub fn shortcut_text(mut self, text: impl Into<String>) -> Self {
        self.shortcut_text = Some(text.into());
        self
    }

    /// Sets why the action is disabled.
    #[must_use]
    pub fn disabled_reason(mut self, reason: impl Into<String>) -> Self {
        self.disabled_reason = Some(reason.into());
        self
    }

    /// Whether the text is not empty (IntelliJ `Presentation.hasText`).
    pub fn has_text(&self) -> bool {
        !self.text.is_empty()
    }

    /// Whether the action is a toggle that is on (IntelliJ `Toggleable.isSelected`).
    pub fn is_selected(&self) -> bool {
        self.selected == Some(true)
    }

    /// Whether the action is shown and can be performed.
    pub fn is_actionable(&self) -> bool {
        self.enabled && self.visible
    }
}
