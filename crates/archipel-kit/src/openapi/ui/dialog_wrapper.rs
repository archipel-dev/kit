//! The modal dialog.

use super::DialogValidation;
use crate::common;
use crate::icons::{self, Glyph};
use crate::parts::Surface;
use crate::theme::Theme;
use crate::tokens::{size, typography};
use crate::ui::Button;
use egui::{Align, Id, Key, Layout, Sense, Ui, vec2};

/// Custom dialog content.
type Content<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// `OK_EXIT_CODE`: the dialog was confirmed (IntelliJ `DialogWrapper.OK_EXIT_CODE`).
pub const OK_EXIT_CODE: i32 = 0;
/// `CANCEL_EXIT_CODE`: the dialog was cancelled.
pub const CANCEL_EXIT_CODE: i32 = 1;
/// `CLOSE_EXIT_CODE`: the dialog was closed without an answer; the same value as cancel.
pub const CLOSE_EXIT_CODE: i32 = 1;
/// `NEXT_USER_EXIT_CODE`: the first code for the dialog's own extra actions.
pub const NEXT_USER_EXIT_CODE: i32 = 2;

/// What the user did with a [`DialogWrapper`].
///
/// [`ExitCode::code`] gives IntelliJ's number for it: [`OK_EXIT_CODE`] for the primary
/// button, [`CANCEL_EXIT_CODE`] for the secondary one and for dismissing, and
/// [`NEXT_USER_EXIT_CODE`] for the tertiary action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExitCode {
    /// The primary button, or <kbd>Enter</kbd> (and <kbd>Y</kbd> in a yes/no dialog).
    Primary,
    /// The secondary button (and <kbd>N</kbd> in a yes/no dialog).
    Secondary,
    /// The tertiary button (left-aligned, e.g. "Reset").
    Tertiary,
    /// <kbd>Escape</kbd>, the close button or a click outside.
    Dismissed,
}

impl ExitCode {
    /// IntelliJ's exit code: 0 for OK, 1 for cancel and close, 2 for the first extra action.
    pub fn code(self) -> i32 {
        match self {
            ExitCode::Primary => OK_EXIT_CODE,
            ExitCode::Secondary => CANCEL_EXIT_CODE,
            ExitCode::Dismissed => CLOSE_EXIT_CODE,
            ExitCode::Tertiary => NEXT_USER_EXIT_CODE,
        }
    }

    /// Whether the dialog was confirmed (IntelliJ `isOK`).
    pub fn is_ok(self) -> bool {
        self.code() == OK_EXIT_CODE
    }
}

/// A modal dialog: a title, a message or custom content, and buttons at the bottom right.
/// The rest of the window is dimmed and blocked while it is shown.
///
/// Stateless: show it while your `open` flag is set and clear the flag when
/// [`DialogWrapper::show`] reports an action. <kbd>Enter</kbd> picks the primary button
/// (inside a multiline field it is a new line: <kbd>Ctrl</kbd>+<kbd>Enter</kbd>, or
/// <kbd>Cmd</kbd>+<kbd>Enter</kbd> on macOS, picks it), <kbd>Escape</kbd> dismisses. The
/// dialog is as tall as its content, up to [`DialogWrapper::max_height`], then scrolls.
///
/// Keys (IntelliJ spec section 7):
///
/// - <kbd>Esc</kbd> cancels ([`ExitCode::Dismissed`]), unless an open dropdown takes it
///   first.
/// - <kbd>Enter</kbd> presses the default (primary) button, unless a field consumes it, or
///   the focus is on another button: that one is pressed.
/// - <kbd>Left</kbd> and <kbd>Right</kbd> move the focus between the buttons.
/// - <kbd>Y</kbd> and <kbd>N</kbd> answer a [yes/no dialog](DialogWrapper::yes_no_keys),
///   while no field has the focus.
/// - The first focusable field of the content gets the focus when the dialog opens, or the
///   one named by [`DialogWrapper::preferred_focus`]; the buttons only when there is none.
/// - Buttons are 12 pt apart.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::openapi::ui::{DialogWrapper, ExitCode};
///
/// let mut open = true;
/// if open {
///     let action = DialogWrapper::new("Delete “main.rs”?")
///         .message("The file will be moved to the trash.")
///         .primary("Delete")
///         .danger(true)
///         .secondary("Cancel")
///         .show(ui.ctx());
///     if action == Some(ExitCode::Primary) { /* delete */ }
///     if action.is_some() { open = false; }
/// }
/// # });
/// ```
#[must_use = "show it with `dialog.show(ctx)`"]
pub struct DialogWrapper<'a> {
    title: String,
    message: Option<String>,
    icon: Option<Glyph>,
    primary: Option<String>,
    secondary: Option<String>,
    tertiary: Option<String>,
    danger: bool,
    primary_enabled: bool,
    yes_no_keys: bool,
    message_icon: Option<MessageIcon>,
    preferred_focus: Option<Id>,
    validation: Option<&'a DialogValidation>,
    width: f32,
    max_height: Option<f32>,
    content: Option<Content<'a>>,
}

impl<'a> DialogWrapper<'a> {
    /// A dialog titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: None,
            icon: None,
            primary: None,
            secondary: None,
            tertiary: None,
            danger: false,
            primary_enabled: true,
            yes_no_keys: false,
            message_icon: None,
            preferred_focus: None,
            validation: None,
            width: 400.0,
            max_height: None,
            content: None,
        }
    }

    /// A message below the title, wrapped.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// An icon before the title (e.g. [`Glyph::Warning`]).
    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Custom content below the message (fields, lists).
    pub fn content(mut self, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// The primary button (rightmost; <kbd>Enter</kbd>).
    pub fn primary(mut self, label: impl Into<String>) -> Self {
        self.primary = Some(label.into());
        self
    }

    /// The secondary button (left of the primary).
    pub fn secondary(mut self, label: impl Into<String>) -> Self {
        self.secondary = Some(label.into());
        self
    }

    /// A tertiary action, a secondary-style button at the left edge (e.g. "Reset").
    /// [`DialogWrapper::show`] reports [`ExitCode::Tertiary`] and the dialog stays yours to
    /// close or keep open.
    pub fn tertiary(mut self, label: impl Into<String>) -> Self {
        self.tertiary = Some(label.into());
        self
    }

    /// Makes the primary button destructive (red).
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    /// Disables the primary button (and <kbd>Enter</kbd>), e.g. until a form is valid.
    pub fn primary_enabled(mut self, enabled: bool) -> Self {
        self.primary_enabled = enabled;
        self
    }

    /// Makes <kbd>Y</kbd> press the primary button and <kbd>N</kbd> the secondary one, for
    /// a dialog that asks a yes/no question (IntelliJ's yes/no message dialogs). They are
    /// ignored while a field has the focus.
    pub fn yes_no_keys(mut self, yes_no_keys: bool) -> Self {
        self.yes_no_keys = yes_no_keys;
        self
    }

    /// A large icon before the message, as in IntelliJ's message dialogs.
    pub fn message_icon(mut self, icon: MessageIcon) -> Self {
        self.message_icon = Some(icon);
        self
    }

    /// The widget that gets the focus when the dialog opens (IntelliJ
    /// `getPreferredFocusedComponent`). Without it the first focusable widget of the
    /// content does.
    pub fn preferred_focus(mut self, id: Id) -> Self {
        self.preferred_focus = Some(id);
        self
    }

    /// Shows the problems of a [`DialogValidation`] above the buttons and disables the
    /// primary button (and <kbd>Enter</kbd>) while one of them blocks.
    pub fn validation(mut self, validation: &'a DialogValidation) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Width in points (default 400).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Tallest the dialog may grow, in points (default 80% of the window). The dialog is
    /// as tall as its content; beyond this the message and content scroll, the title and
    /// the buttons stay.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Shows the dialog this frame and reports what the user did, if anything.
    pub fn show(self, ctx: &egui::Context) -> Option<ExitCode> {
        let colors = Theme::colors(ctx);
        let id = egui::Id::new(("archipel-dialog", &self.title));
        let frame = Surface::Dialog.frame(ctx);
        let frame_height = frame.total_margin().sum().y;
        let mut action = None;
        let blocked = self.validation.is_some_and(DialogValidation::blocks_ok);
        let primary_enabled = self.primary_enabled && !blocked;
        // The dialog is new when it was not drawn in the last passes. The first passes of a
        // modal only measure it, so the initial focus is asked for until something has it.
        let opened_key = id.with("opened");
        let pass = ctx.cumulative_pass_nr();
        let (last_pass, mut passes_open) = ctx
            .data(|d| d.get_temp::<(u64, u32)>(opened_key))
            .unwrap_or((0, u32::MAX));
        if last_pass == 0 || pass.saturating_sub(last_pass) > 2 {
            passes_open = 0;
        }
        ctx.data_mut(|d| d.insert_temp(opened_key, (pass, passes_open.saturating_add(1))));
        if passes_open < 4 && ctx.memory(|m| m.focused().is_none()) {
            match self.preferred_focus {
                Some(field) => ctx.memory_mut(|m| m.request_focus(field)),
                // The first widget that takes the focus does: the content comes before
                // the buttons, so a field wins over them.
                None if self.content.is_some() => {
                    ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
                }
                None => {}
            }
        }
        let mut button_ids: Vec<Id> = Vec::new();
        let max_height = self
            .max_height
            .unwrap_or_else(|| ctx.content_rect().height() * 0.8);
        // Focus before the dialog runs: a field that still has it afterwards kept Enter.
        let focus_before = ctx.memory(|m| m.focused());
        let modal = egui::Modal::new(id)
            .frame(frame)
            .backdrop_color(
                egui::Color32::BLACK.gamma_multiply(if Theme::current(ctx).dark {
                    crate::tokens::opacity::SCRIM_DARK
                } else {
                    crate::tokens::opacity::SCRIM_LIGHT
                }),
            )
            .show(ctx, |ui| {
                ui.set_width(self.width);
                ui.spacing_mut().item_spacing.y = size::SECTION_SPACING;
                ui.horizontal(|ui| {
                    if let Some(icon) = self.icon {
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(size::ICON, size::ICON), Sense::hover());
                        icons::paint(ui.painter(), rect, icon, colors.icon);
                    }
                    let title = common::galley(
                        ui,
                        self.title.as_str(),
                        typography::semibold(ui.ctx(), typography::TITLE + 1.0),
                    );
                    let (rect, response) = ui.allocate_exact_size(title.size(), Sense::hover());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.title)
                    });
                    common::text(ui.painter(), rect.min, title, colors.text_strong);
                });
                // Title and buttons stay; what lies between scrolls past the max height.
                // What doesn't scroll: the frame, the title as drawn (measured), the button
                // row, and the four gaps between the title, body, space and buttons.
                let title_height = ui.min_rect().height();
                // A dialog without buttons (its body has its own) has no button row.
                let has_buttons =
                    self.primary.is_some() || self.secondary.is_some() || self.tertiary.is_some();
                let button_row = if has_buttons {
                    size::CONTROL_HEIGHT + 2.0 * size::SECTION_SPACING
                } else {
                    0.0
                };
                let chrome = frame_height + title_height + button_row + 2.0 * size::SECTION_SPACING;
                let body_max = (max_height - chrome).max(size::ROW_HEIGHT);
                // A solid scroll bar: it shows that there is more without hovering.
                ui.scope(|ui| {
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    ui.visuals_mut().extreme_bg_color = colors.border;
                    ui.visuals_mut().widgets.inactive.bg_fill = colors.text_placeholder;
                    ui.visuals_mut().widgets.hovered.bg_fill = colors.text_secondary;
                    ui.visuals_mut().widgets.active.bg_fill = colors.text_secondary;
                    let body = crate::parts::scroll_vertical()
                        .id_salt("archipel-dialog-body")
                        .max_height(body_max)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = size::SECTION_SPACING;
                            if let Some(message) = &self.message {
                                let label = egui::Label::new(
                                    egui::RichText::new(message)
                                        .font(typography::body(typography::BODY))
                                        .color(colors.text),
                                )
                                .wrap();
                                match self.message_icon {
                                    Some(kind) => {
                                        ui.horizontal_top(|ui| {
                                            ui.spacing_mut().item_spacing.x = size::SECTION_SPACING;
                                            let (rect, _) = ui.allocate_exact_size(
                                                vec2(size::MESSAGE_ICON, size::MESSAGE_ICON),
                                                Sense::hover(),
                                            );
                                            icons::paint(
                                                ui.painter(),
                                                rect,
                                                kind.glyph(),
                                                kind.color(&colors),
                                            );
                                            ui.add(label);
                                        });
                                    }
                                    None => {
                                        ui.add(label);
                                    }
                                }
                            }
                            if let Some(content) = self.content {
                                content(ui);
                            }
                        });
                    // The modal sizes itself from the last frame: when the body grew or
                    // shrank (a page switched, a banner appeared), draw once more so the
                    // dialog fits without waiting for the next input.
                    let key = id.with("body-height");
                    let height = body.content_size.y;
                    let before = ui.data(|d| d.get_temp::<f32>(key));
                    if before.is_some_and(|h| (h - height).abs() > 0.5) {
                        ui.ctx().request_repaint();
                    }
                    ui.data_mut(|d| d.insert_temp(key, height));
                });
                // The problems the validation found, above the buttons.
                if let Some(validation) = self.validation {
                    for info in validation.infos() {
                        let color = if info.warning {
                            colors.warning
                        } else {
                            colors.error
                        };
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&info.message)
                                    .font(typography::body(typography::BODY))
                                    .color(color),
                            )
                            .wrap(),
                        );
                    }
                }
                if !has_buttons {
                    return;
                }
                ui.add_space(size::SECTION_SPACING);
                // A fixed-height row: `with_layout` would take all the height left over.
                // Drawn left to right so Tab follows reading order: tertiary, secondary,
                // primary. The secondary and primary buttons sit at the right edge: their
                // widths are measured first, so the first frame is already right.
                let secondary = self.secondary.as_deref().map(Button::secondary);
                let primary = self.primary.as_deref().map(|label| {
                    let button = if self.danger {
                        Button::danger(label)
                    } else {
                        Button::primary(label)
                    };
                    button.enabled(primary_enabled)
                });
                let gap = size::DIALOG_BUTTON_GAP;
                let mut group = 0.0;
                for button in [&secondary, &primary].into_iter().flatten() {
                    group += button.width(ui);
                }
                if secondary.is_some() && primary.is_some() {
                    group += gap;
                }
                let row = vec2(ui.available_width(), size::CONTROL_HEIGHT);
                ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    if let Some(tertiary) = &self.tertiary {
                        let button = ui.add(Button::secondary(tertiary.as_str()));
                        button_ids.push(button.id);
                        if button.clicked() {
                            action = Some(ExitCode::Tertiary);
                        }
                    }
                    ui.add_space((ui.available_width() - group).max(0.0));
                    if let Some(secondary) = secondary {
                        let button = ui.add(secondary);
                        button_ids.push(button.id);
                        if button.clicked() {
                            action = Some(ExitCode::Secondary);
                        }
                    }
                    if let Some(primary) = primary {
                        let button = ui.add(primary);
                        button_ids.push(button.id);
                        if button.clicked() {
                            action = Some(ExitCode::Primary);
                        }
                    }
                });
            });
        // Buttons are not where the focus starts: if the search found none but them, it
        // stays out of the dialog's buttons and Enter works as the default button.
        if passes_open < 4
            && self.preferred_focus.is_none()
            && let Some(focused) = ctx.memory(|m| m.focused())
            && button_ids.contains(&focused)
        {
            ctx.memory_mut(|m| m.surrender_focus(focused));
        }
        // Left and Right move between the buttons. A focused button keeps these keys from
        // egui's own (spatial) focus search, so the order is always the reading order.
        if let Some(position) = button_ids
            .iter()
            .position(|id| ctx.memory(|m| m.has_focus(*id)))
        {
            let focused = button_ids[position];
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(
                    focused,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                )
            });
            let step = ctx.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowLeft) {
                    -1
                } else if i.consume_key(egui::Modifiers::NONE, Key::ArrowRight) {
                    1
                } else {
                    0
                }
            });
            if step != 0
                && let Some(next) = position
                    .checked_add_signed(step)
                    .and_then(|next| button_ids.get(next))
            {
                ctx.memory_mut(|m| m.request_focus(*next));
            }
        }
        if action.is_none() {
            // A multiline field keeps the focus when it takes Enter (a single-line field
            // gives it up), and Enter there is a new line: Ctrl/Cmd+Enter confirms.
            // egui has no "this field is multiline" query, so this relies on two of its
            // behaviours: a `TextEdit` reports IME output while it has focus, and a
            // single-line one surrenders focus on Enter. Anything else that keeps the focus
            // (no IME output) is not a text field: Enter still confirms. Tests pin this.
            let in_multiline = focus_before.is_some()
                && ctx.memory(|m| m.focused()) == focus_before
                && ctx.output(|o| o.ime.is_some());
            let enter = ctx.input(|i| {
                i.events.iter().any(|event| {
                    matches!(event, egui::Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        modifiers,
                        ..
                    } if !in_multiline || modifiers.command || modifiers.ctrl)
                })
            });
            // Y and N answer a yes/no dialog, but only when no field would take them as text.
            let typing = ctx.output(|o| o.ime.is_some());
            let answer = if self.yes_no_keys && !typing {
                ctx.input(|i| {
                    i.events.iter().find_map(|event| match event {
                        egui::Event::Key {
                            key: Key::Y,
                            pressed: true,
                            modifiers,
                            ..
                        } if modifiers.is_none() => Some(ExitCode::Primary),
                        egui::Event::Key {
                            key: Key::N,
                            pressed: true,
                            modifiers,
                            ..
                        } if modifiers.is_none() => Some(ExitCode::Secondary),
                        _ => None,
                    })
                })
            } else {
                None
            };
            if enter && blocked && self.primary.is_some() {
                // Confirming with a problem on screen: take the user to the field.
                if let Some(field) = self
                    .validation
                    .and_then(|v| v.infos().iter().find(|i| i.blocks_ok()))
                    .and_then(|info| info.field)
                {
                    ctx.memory_mut(|m| m.request_focus(field));
                }
            }
            let can_ok = primary_enabled && self.primary.is_some();
            let answered = match answer {
                Some(ExitCode::Primary) if can_ok => answer,
                Some(ExitCode::Secondary) if self.secondary.is_some() => answer,
                _ => None,
            };
            if enter && can_ok {
                action = Some(ExitCode::Primary);
            } else if answered.is_some() {
                action = answered;
            } else if modal.should_close() {
                action = Some(ExitCode::Dismissed);
            }
        }
        action
    }
}

/// The icon of a message dialog (IntelliJ `Messages.getInformationIcon` and friends),
/// 32 pt, before the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageIcon {
    /// Information.
    Information,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// A question (yes/no and OK/cancel dialogs).
    Question,
}

impl MessageIcon {
    fn glyph(self) -> Glyph {
        match self {
            MessageIcon::Information => Glyph::Info,
            MessageIcon::Warning => Glyph::Warning,
            MessageIcon::Error => Glyph::Error,
            MessageIcon::Question => Glyph::Help,
        }
    }

    fn color(self, colors: &crate::theme::Colors) -> egui::Color32 {
        match self {
            MessageIcon::Information | MessageIcon::Question => colors.link,
            MessageIcon::Warning => colors.warning,
            MessageIcon::Error => colors.error,
        }
    }
}
