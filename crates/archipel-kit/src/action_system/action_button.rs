//! Square icon-only buttons for toolbars and headers (IntelliJ `ActionButton`).

use super::{
    ActionPlace, AnAction, DataContext, InputKind, Presentation, perform_with_modifiers,
    update_presentation,
};
use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{action_toolbar, radius, size, toolbar};
use egui::{
    Color32, Id, Key, PointerButton, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType,
    pos2, vec2,
};

/// The look an action button is in (IntelliJ `ActionButtonComponent`: `NORMAL`, `POPPED`,
/// `PUSHED`, `SELECTED`), derived in this priority order: disabled is `Normal`; pressed with
/// the pointer still over it, or toggled on, is `Pushed`; hovered is `Popped`; keyboard
/// focus is `Selected`; else `Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ButtonLook {
    /// At rest, or disabled: no fill.
    Normal,
    /// Hovered: the hover fill.
    Popped,
    /// Pressed (pointer still over it) or toggled on: the pressed fill.
    Pushed,
    /// Keyboard focus: the hover fill and the focus ring.
    Selected,
}

/// What [`ActionButton::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ActionButtonResponse {
    /// The widget's response.
    pub response: Response,
    /// The button was activated: released inside after a primary-button press, <kbd>Space</kbd>
    /// while focused, or <kbd>↓</kbd> while focused when it paints a drop-down arrow.
    pub activated: bool,
    /// The action it renders (see [`ActionButton::for_action`]) was performed.
    pub performed: bool,
    /// The look it was drawn in.
    pub look: ButtonLook,
}

/// A transparent, square button showing one icon; hover and press tint it.
///
/// The label is required: it is the tooltip title and the accessible name, since there is no
/// visible text.
///
/// State machine (IntelliJ spec §1): it fires on **release inside**, only for the primary
/// button; a press that started on any action button suppresses the hover look of every other
/// one until release (the pointer origin is shared through the `egui::Context`, not a global);
/// dragging off removes the pressed look and releasing off does nothing. A toggled-on button
/// keeps the pressed look. It is **not in the Tab order** unless a screen reader is active
/// (AccessKit enabled) or [`focusable`](Self::focusable) says so; focused, <kbd>Space</kbd>
/// activates it (and <kbd>↓</kbd> when it has a drop-down arrow).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, action_system::ActionButton};
///
/// if ui.add(ActionButton::new(Glyph::Refresh, "Reload")).clicked() { /* ... */ }
/// let mut pinned = false;
/// if ui.add(ActionButton::new(Glyph::Pin, "Pin tab").selected(pinned)).clicked() {
///     pinned = !pinned;
/// }
/// # });
/// ```
///
/// From an action: the button updates it, shows its presentation (text as tooltip with the
/// shortcut, icon, enabled, toggle state) and performs it on activation.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, action_system::*};
///
/// struct Reload;
/// impl AnAction for Reload {
///     fn id(&self) -> &str { "Reload" }
///     fn template_presentation(&self) -> Presentation {
///         Presentation::new("Reload").icon(Glyph::Refresh).shortcut_text("Ctrl+R")
///     }
///     fn action_performed(&self, _e: &AnActionEvent<'_>) {}
/// }
/// let context = DataContext::empty();
/// let out = ActionButton::for_action(&Reload, &context, &ActionPlace::TOOLBAR).show(ui);
/// let _performed = out.performed;
/// # });
/// ```
#[must_use = "add it with `ui.add(button)` or `.show(ui)`"]
pub struct ActionButton<'a> {
    icon: Icon,
    label: String,
    selected: Option<bool>,
    enabled: bool,
    tone: Option<Tone>,
    small: bool,
    large: bool,
    title_pane: bool,
    dropdown: bool,
    shortcut: Option<String>,
    hint: Option<String>,
    description: Option<String>,
    focusable: Option<bool>,
    target: Option<Target<'a>>,
}

/// The action a button performs.
struct Target<'a> {
    action: &'a dyn AnAction,
    context: &'a DataContext,
    place: &'a ActionPlace,
}

impl<'a> ActionButton<'a> {
    /// A button showing `icon`, described by `label` (tooltip and accessible name).
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            selected: None,
            enabled: true,
            tone: None,
            small: false,
            large: false,
            title_pane: false,
            dropdown: false,
            shortcut: None,
            hint: None,
            description: None,
            focusable: None,
            target: None,
        }
    }

    /// A button rendering `presentation`: its icon (the help icon when it has none, where
    /// IntelliJ shows `AllIcons.Toolbar.Unknown`), its text as label, shortcut text and
    /// description, enabled and toggle state, and a drop-down arrow for a popup group.
    pub fn from_presentation(presentation: &Presentation) -> Self {
        let mut button = Self::new(
            presentation.icon.unwrap_or(Icon::Glyph(Glyph::Help)),
            presentation.text.clone(),
        );
        button.selected = presentation.selected;
        button.enabled = presentation.enabled;
        button.dropdown = presentation.popup_group;
        button.shortcut.clone_from(&presentation.shortcut_text);
        if !presentation.description.is_empty() {
            button.description = Some(presentation.description.clone());
        }
        button
    }

    /// A button for `action` in `place`: updates it, renders its presentation (see
    /// [`from_presentation`](Self::from_presentation)) and, when activated,
    /// [`perform`](super::perform)s it. A popup group is not performed: use
    /// [`ActionToolbar`](super::ActionToolbar), which opens it.
    pub fn for_action(
        action: &'a dyn AnAction,
        context: &'a DataContext,
        place: &'a ActionPlace,
    ) -> Self {
        let presentation = update_presentation(action, context, place);
        let mut button = Self::from_presentation(&presentation);
        button.target = Some(Target {
            action,
            context,
            place,
        });
        button
    }

    /// Makes it a toggle: toggled-on buttons keep the pressed look.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Disabled buttons are dimmed and ignore everything.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A 20-point button for actions inside rows and tabs, which are 24 points high.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// A 30-point button with a 20-point icon, for the main window toolbar (as in Android
    /// Studio).
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// A 40-point square title pane button (minimize, maximize, close) with a square hover
    /// fill (`TitlePane.Button.preferredSize`, IntelliJ spec §14).
    pub fn title_pane(mut self, title_pane: bool) -> Self {
        self.title_pane = title_pane;
        self
    }

    /// Meaning of the icon's color (default: the icon gray), e.g. [`Tone::Run`].
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Paints a small drop-down arrow after the icon (a group that opens a popup). Focused,
    /// <kbd>↓</kbd> activates it.
    pub fn dropdown(mut self, dropdown: bool) -> Self {
        self.dropdown = dropdown;
        self
    }

    /// The shortcut shown in the tooltip, grey after the title.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// A longer explanation under the tooltip title. IntelliJ shows it only for
    /// whitelisted actions, so it is never taken from the presentation by itself: the caller
    /// decides.
    pub fn tooltip_description(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Whether the button is in the Tab order. The default follows IntelliJ: only while a
    /// screen reader is active. Pass `true` to make a toolbar keyboard-reachable always.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = Some(focusable);
        self
    }

    /// The description of the action under the pointer, for a status bar (IntelliJ publishes
    /// it on enter and clears it on exit). Empty when no described button is hovered.
    pub fn hovered_description(ctx: &egui::Context) -> Option<String> {
        let (frame, text): (u64, String) = ctx.data(|d| d.get_temp(status_key()))?;
        (frame + 1 >= ctx.cumulative_frame_nr() && !text.is_empty()).then_some(text)
    }

    /// Draws the button and reports what happened.
    pub fn show(self, ui: &mut Ui) -> ActionButtonResponse {
        let ctx = ui.ctx().clone();
        let colors = Theme::colors(&ctx);
        let focusable = self.focusable.unwrap_or_else(|| {
            ctx.data(|d| d.get_temp(screen_reader_key()))
                .unwrap_or(false)
        });
        let sense = match (self.enabled, focusable) {
            (false, _) => Sense::hover(),
            (true, true) => Sense::click(),
            (true, false) => Sense::CLICK,
        };
        let (edge, icon_edge) = if self.title_pane {
            (toolbar::TITLE_BUTTON, size::ICON)
        } else if self.large {
            (toolbar::BUTTON, toolbar::ICON)
        } else if self.small {
            (size::ICON_BUTTON_SMALL, size::ICON)
        } else {
            (size::ICON_BUTTON, size::ICON)
        };
        let width = edge
            + if self.dropdown {
                action_toolbar::ARROW_WIDTH
            } else {
                0.0
            };
        let (rect, response) = ui.allocate_exact_size(vec2(width, edge), sense);
        // Remember whether a screen reader is listening, for the next frame's sense.
        let reader = ctx.accesskit_node_builder(response.id, |_| ()).is_some();
        ctx.data_mut(|d| d.insert_temp(screen_reader_key(), reader));
        crate::cursor::on_hover_action(&response);
        let toggled = self.selected.unwrap_or(false);
        response.widget_info(|| match self.selected {
            Some(selected) => {
                WidgetInfo::selected(WidgetType::Button, self.enabled, selected, &self.label)
            }
            None => WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.label),
        });

        // Shared pointer origin: which action button a press started on.
        let primary_down = ui.input(|i| i.pointer.primary_down());
        let mut origin: Option<Id> = ctx.data(|d| d.get_temp(origin_key()));
        if !primary_down {
            origin = None;
        }
        let pressed_here = self.enabled
            && primary_down
            && response.is_pointer_button_down_on()
            && !ui.input(|i| i.pointer.button_down(PointerButton::Secondary));
        if pressed_here && origin.is_none() {
            origin = Some(response.id);
        }
        match origin {
            Some(id) => ctx.data_mut(|d| d.insert_temp(origin_key(), id)),
            None => ctx.data_mut(|d| d.remove::<Id>(origin_key())),
        }
        let elsewhere = origin.is_some_and(|id| id != response.id);
        let over = response.contains_pointer();
        let hovered = self.enabled && response.hovered() && !elsewhere;
        let pressed = pressed_here && over;
        let focused = self.enabled && response.has_focus();
        let look = if !self.enabled {
            ButtonLook::Normal
        } else if pressed || toggled {
            ButtonLook::Pushed
        } else if hovered {
            ButtonLook::Popped
        } else if focused {
            ButtonLook::Selected
        } else {
            ButtonLook::Normal
        };

        if hovered {
            let text = self.description.clone().unwrap_or_default();
            let frame = ctx.cumulative_frame_nr();
            ctx.data_mut(|d| d.insert_temp(status_key(), (frame, text)));
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let fill = match look {
                ButtonLook::Normal => Color32::TRANSPARENT,
                ButtonLook::Popped | ButtonLook::Selected => colors.hover,
                ButtonLook::Pushed => colors.pressed,
            };
            // Title pane buttons fill their square; toolbar buttons are rounder than the
            // 22 pt ones inside islands (measured on Android Studio).
            let corner = if self.title_pane {
                0
            } else if self.large {
                radius::ICON_BUTTON
            } else {
                radius::ICON_BUTTON_SMALL
            };
            painter.rect_filled(rect, corner, fill);
            if focused {
                common::focus_ring(painter, rect, corner, &colors);
            }
            let color = if !self.enabled {
                colors.text_disabled
            } else {
                self.tone.unwrap_or(Tone::Icon).color(&colors)
            };
            let icon_rect = Rect::from_center_size(
                pos2(rect.left() + edge / 2.0, rect.center().y),
                vec2(icon_edge, icon_edge),
            );
            icons::paint(painter, icon_rect, self.icon, color);
            if self.dropdown {
                let arrow = Rect::from_center_size(
                    pos2(
                        rect.right() - action_toolbar::ARROW_WIDTH / 2.0 - 1.0,
                        rect.center().y,
                    ),
                    vec2(action_toolbar::ARROW_ICON, action_toolbar::ARROW_ICON),
                );
                icons::paint(painter, arrow, Glyph::ChevronDown, color);
            }
        }

        let mut tooltip = crate::ide::HelpTooltip::new(self.label.clone());
        if let Some(shortcut) = &self.shortcut {
            tooltip = tooltip.shortcut(shortcut.clone());
        }
        if let Some(hint) = &self.hint {
            tooltip = tooltip.hint(hint.clone());
        }
        tooltip.show_for(&response);

        // Activation: release inside after a primary press (no Cmd on macOS), Space while
        // focused, Down while focused with an arrow. Enter does not activate in IntelliJ.
        let enter = focused && ui.input(|i| i.key_pressed(Key::Enter));
        let pointer_release = ui.input(|i| i.pointer.primary_released());
        let mac_cmd = ui.input(|i| i.modifiers.mac_cmd);
        let mut activated =
            self.enabled && response.clicked() && !mac_cmd && !(enter && !pointer_release);
        let down = focused
            && self.dropdown
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown));
        activated |= down;
        let mut performed = false;
        if activated
            && !self.dropdown
            && let Some(target) = &self.target
        {
            let input = if pointer_release {
                InputKind::Mouse
            } else {
                InputKind::Keyboard
            };
            let modifiers = ui.input(|i| i.modifiers);
            performed = perform_with_modifiers(
                target.action,
                target.context,
                target.place,
                input,
                modifiers,
            );
        }
        ActionButtonResponse {
            response,
            activated,
            performed,
            look,
        }
    }
}

fn origin_key() -> Id {
    Id::new("archipel-action-button-pointer-origin")
}

fn status_key() -> Id {
    Id::new("archipel-action-button-status")
}

fn screen_reader_key() -> Id {
    Id::new("archipel-action-button-screen-reader")
}

impl Widget for ActionButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
