//! [`Dialog`]: IntelliJ's `DialogWrapper` as a modal float, and [`Messages`].
//!
//! - <kbd>Enter</kbd> presses the default (primary) button, or the focused button when a
//!   button has focus; <kbd>Esc</kbd> cancels (unless the content uses it first, as a
//!   search field that clears does); <kbd>Left</kbd> and <kbd>Right</kbd> move between the
//!   buttons when one has focus; <kbd>Tab</kbd> cycles through the content and the buttons;
//!   <kbd>Y</kbd> and <kbd>N</kbd> answer a yes/no question; <kbd>Alt</kbd> plus a button's
//!   mnemonic presses it; a press outside dismisses.
//! - The primary button is disabled while a validation error blocks it. Validation runs 300 ms
//!   after the content last changed (its [`revision`](DialogContent::revision)); errors are
//!   red lines above the buttons, warnings yellow.
//! - Buttons: the tertiary action at the left, secondary and primary at the right (primary
//!   last), 2 cells apart.

use crate::components::{Button, fill, fill_rect, put, text_width, title_block, wrap_text};
use crate::controls::{ButtonControl, ButtonState};
use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crate::overlay::{Handled, Overlay};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use archipel_kit::openapi::ui::{ExitCode, MessageIcon, ValidationInfo};
use archipel_kit::tokens::motion;
use archipel_kit::ui::ButtonVariant;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// The custom content of a dialog.
pub trait DialogContent {
    /// The rows the content needs at `width`.
    fn height(&self, width: u16) -> u16;

    /// Draws the content in `area`; register widgets with `rt` (they join the dialog's
    /// focus order).
    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer);

    /// Applies `event`; returns whether it was used (then the dialog does not act on it).
    fn event(&mut self, rt: &mut Runtime, event: &Event) -> bool;

    /// A number that changes whenever the input changes, to trigger the validation.
    fn revision(&self) -> u64 {
        0
    }

    /// Checks the input.
    fn validate(&self) -> Vec<ValidationInfo> {
        Vec::new()
    }

    /// The widget that gets focus when the dialog opens.
    fn preferred_focus(&self) -> Option<Id> {
        None
    }
}

type ExitFn<M> = Box<dyn FnMut(ExitCode) -> Vec<M>>;

/// A modal dialog.
pub struct Dialog<M> {
    title: String,
    message: Option<String>,
    icon: Option<MessageIcon>,
    primary: Option<String>,
    secondary: Option<String>,
    tertiary: Option<String>,
    danger: bool,
    yes_no_keys: bool,
    width: u16,
    content: Option<Box<dyn DialogContent>>,
    on_exit: ExitFn<M>,
    states: [ButtonState; 3],
    infos: Vec<ValidationInfo>,
    validated_revision: Option<u64>,
    changed_at: Option<f64>,
    asked_focus: bool,
    rect: Rect,
    exit: Option<ExitCode>,
    id: Id,
}

impl<M: 'static> Dialog<M> {
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
            yes_no_keys: false,
            width: 56,
            content: None,
            on_exit: Box::new(|_| Vec::new()),
            states: [ButtonState::default(); 3],
            infos: Vec::new(),
            validated_revision: None,
            changed_at: None,
            asked_focus: false,
            rect: Rect::default(),
            exit: None,
            id: Id::new("dialog"),
        }
    }

    /// The message above the content.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// The icon before the message.
    #[must_use]
    pub fn message_icon(mut self, icon: MessageIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The label of the primary (default) button.
    #[must_use]
    pub fn primary(mut self, label: impl Into<String>) -> Self {
        self.primary = Some(label.into());
        self
    }

    /// The label of the secondary (cancel) button.
    #[must_use]
    pub fn secondary(mut self, label: impl Into<String>) -> Self {
        self.secondary = Some(label.into());
        self
    }

    /// The label of the tertiary button at the left.
    #[must_use]
    pub fn tertiary(mut self, label: impl Into<String>) -> Self {
        self.tertiary = Some(label.into());
        self
    }

    /// Makes the primary button a destructive one.
    #[must_use]
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    /// Answers with Y and N.
    #[must_use]
    pub fn yes_no_keys(mut self, on: bool) -> Self {
        self.yes_no_keys = on;
        self
    }

    /// The dialog width in cells (default 56).
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Custom content below the message.
    #[must_use]
    pub fn content(mut self, content: impl DialogContent + 'static) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// What to tell the app when the dialog ends.
    #[must_use]
    pub fn on_exit(mut self, f: impl FnMut(ExitCode) -> Vec<M> + 'static) -> Self {
        self.on_exit = Box::new(f);
        self
    }

    /// The identity of the dialog's widgets.
    #[must_use]
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    fn button_id(&self, slot: usize) -> Id {
        self.id.with(0xB0 + slot as u64)
    }

    fn blocked(&self) -> bool {
        self.infos.iter().any(ValidationInfo::blocks_ok)
    }

    fn labels(&self) -> [Option<&str>; 3] {
        // primary, secondary, tertiary
        [
            self.primary.as_deref(),
            self.secondary.as_deref(),
            self.tertiary.as_deref(),
        ]
    }

    fn exit_code(slot: usize) -> ExitCode {
        match slot {
            0 => ExitCode::Primary,
            1 => ExitCode::Secondary,
            _ => ExitCode::Tertiary,
        }
    }

    fn finish(&mut self, code: ExitCode) -> Handled<M> {
        if code == ExitCode::Primary && self.blocked() {
            return Handled::consumed();
        }
        self.exit = Some(code);
        let messages = (self.on_exit)(code);
        Handled::closed().with_all(messages)
    }

    fn mnemonic(label: &str) -> Option<char> {
        label.chars().find(|c| c.is_alphabetic())
    }

    fn run_validation(&mut self) {
        if let Some(content) = &self.content {
            self.infos = content.validate();
        }
    }

    /// The ExitCode the dialog ended with.
    pub fn exit_code_taken(&self) -> Option<ExitCode> {
        self.exit
    }
}

impl<M: 'static> Overlay<M> for Dialog<M> {
    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<M> {
        // The content first.
        if let Some(content) = &mut self.content {
            let used = content.event(rt, event);
            let revision = content.revision();
            if self.validated_revision.is_some() && self.validated_revision != Some(revision) {
                // The input changed: validate 300 ms after the last change.
                self.validated_revision = Some(revision);
                self.changed_at = Some(rt.now);
            }
            if used {
                return Handled::consumed();
            }
        }
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let focused_button = (0..3).find(|s| rt.focused(self.button_id(*s)));
                match key.code {
                    KeyCode::Esc => return self.finish(ExitCode::Dismissed),
                    KeyCode::Enter if key.modifiers.is_empty() => {
                        let slot = focused_button.unwrap_or(0);
                        if self.labels()[slot].is_some() {
                            return self.finish(Self::exit_code(slot));
                        }
                    }
                    KeyCode::Left | KeyCode::Right if focused_button.is_some() => {
                        let order: Vec<usize> = [2usize, 1, 0]
                            .into_iter()
                            .filter(|s| self.labels()[*s].is_some())
                            .collect();
                        let at = order
                            .iter()
                            .position(|s| Some(*s) == focused_button)
                            .unwrap_or(0);
                        let next = if key.code == KeyCode::Left {
                            at.saturating_sub(1)
                        } else {
                            (at + 1).min(order.len() - 1)
                        };
                        rt.focus.focus(self.button_id(order[next]));
                        return Handled::consumed();
                    }
                    KeyCode::Tab => {
                        rt.focus.next();
                        return Handled::consumed();
                    }
                    KeyCode::BackTab => {
                        rt.focus.previous();
                        return Handled::consumed();
                    }
                    KeyCode::Char(c) if self.yes_no_keys && key.modifiers.is_empty() => {
                        match c.to_ascii_lowercase() {
                            'y' => return self.finish(ExitCode::Primary),
                            'n' => return self.finish(ExitCode::Secondary),
                            _ => {}
                        }
                    }
                    KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) => {
                        for slot in 0..3 {
                            if self.labels()[slot]
                                .and_then(Self::mnemonic)
                                .is_some_and(|m| m.eq_ignore_ascii_case(&c))
                            {
                                return self.finish(Self::exit_code(slot));
                            }
                        }
                    }
                    _ => {}
                }
                Handled::consumed()
            }
            Event::Mouse(_) => {
                let Some(p) = rt.pointer().copied() else {
                    return Handled::consumed();
                };
                for slot in 0..3 {
                    let id = self.button_id(slot);
                    let label = self.labels()[slot].unwrap_or("").to_owned();
                    let control = ButtonControl::new(id, &label);
                    if !label.is_empty() {
                        let enabled = !(slot == 0 && self.blocked());
                        let control = control.enabled(enabled);
                        if control.handle(rt, &mut self.states[slot], event) {
                            return self.finish(Self::exit_code(slot));
                        }
                    }
                }
                // A press outside dismisses.
                if p.kind == PointerKind::Down(crate::input::MouseButton::Left)
                    && !self.rect.contains((p.x, p.y).into())
                {
                    return self.finish(ExitCode::Dismissed);
                }
                Handled::consumed()
            }
            Event::Paste(_) => Handled::consumed(),
            _ => Handled::ignored(),
        }
    }

    fn tick(&mut self, rt: &mut Runtime) -> Vec<M> {
        let Some(content) = &self.content else {
            return Vec::new();
        };
        let revision = content.revision();
        if self.validated_revision.is_none() {
            self.validated_revision = Some(revision);
            self.run_validation();
        } else if self.validated_revision != Some(revision) {
            self.validated_revision = Some(revision);
            self.changed_at = Some(rt.now);
        }
        if let Some(at) = self.changed_at
            && rt.now - at >= f64::from(motion::DIALOG_VALIDATION_DELAY)
        {
            self.changed_at = None;
            self.run_validation();
        }
        Vec::new()
    }

    fn deadline(&self) -> Option<f64> {
        self.changed_at
            .map(|t| t + f64::from(motion::DIALOG_VALIDATION_DELAY))
    }

    fn finished(&self) -> bool {
        self.exit.is_some()
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let p = rt.look.palette;
        let width = self
            .width
            .min(area.width.saturating_sub(2))
            .max(area.width.min(20));
        let inner_w = width.saturating_sub(4);
        let icon_w = if self.icon.is_some() { 3 } else { 0 };
        let message_lines = self
            .message
            .as_deref()
            .map(|m| wrap_text(m, inner_w.saturating_sub(icon_w).max(1)))
            .unwrap_or_default();
        let content_h = self.content.as_ref().map_or(0, |c| c.height(inner_w));
        let info_rows = self.infos.len().min(3) as u16;
        let mut height = 2 + message_lines.len() as u16 + content_h + info_rows + 3;
        if !message_lines.is_empty() && content_h > 0 {
            height += 1;
        }
        height = height.min(area.height.max(1));
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 3;
        let rect = Rect::new(x, y, width, height).intersection(area);
        self.rect = rect;
        let bg = p.darker_black;
        fill_rect(buf, rect, Style::default().bg(bg).fg(p.white));
        rt.hits.add(rect, self.id);
        let title_color = match self.icon {
            Some(MessageIcon::Error) => p.red,
            Some(MessageIcon::Warning) => p.yellow,
            _ => p.nord_blue,
        };
        title_block(
            buf,
            rect,
            rect.x + 1,
            rect.y,
            &self.title,
            p.on(title_color),
            title_color,
        );
        let mut cy = rect.y + 2;
        if !message_lines.is_empty() {
            if let Some(icon) = self.icon {
                let (glyph, color) = match icon {
                    MessageIcon::Information => (crate::Glyph::Info, p.blue),
                    MessageIcon::Warning => (crate::Glyph::Warning, p.yellow),
                    MessageIcon::Error => (crate::Glyph::Error, p.red),
                    MessageIcon::Question => (crate::Glyph::Help, p.blue),
                };
                put(
                    buf,
                    rect,
                    rect.x + 2,
                    cy,
                    &rt.look.glyph(glyph),
                    Style::default().fg(color).bg(bg),
                );
            }
            for line in &message_lines {
                put(
                    buf,
                    rect,
                    rect.x + 2 + icon_w,
                    cy,
                    line,
                    Style::default().fg(p.white).bg(bg),
                );
                cy += 1;
            }
            if content_h > 0 {
                cy += 1;
            }
        }
        if let Some(content) = &mut self.content {
            let content_rect = Rect::new(rect.x + 2, cy, inner_w, content_h).intersection(rect);
            if !self.asked_focus {
                self.asked_focus = true;
                if let Some(id) = content.preferred_focus() {
                    rt.focus.focus(id);
                }
            }
            content.draw(rt, content_rect, buf);
            cy += content_h;
        }
        if !self.asked_focus {
            self.asked_focus = true;
            rt.focus.focus(self.button_id(0));
        }
        for info in self.infos.iter().take(3) {
            let color = if info.warning { p.yellow } else { p.red };
            let glyph = if info.warning {
                crate::Glyph::Warning
            } else {
                crate::Glyph::Error
            };
            let s = Style::default().fg(color).bg(bg);
            let ex = put(buf, rect, rect.x + 2, cy, &rt.look.glyph(glyph), s);
            put(buf, rect, ex + 1, cy, &info.message, s);
            cy += 1;
        }
        // Buttons on the last rows: tertiary at the left, secondary and primary at the right.
        let by = rect.bottom().saturating_sub(2);
        fill(buf, rect, rect.x, by, rect.width, Style::default().bg(bg));
        let on: Color = bg;
        // Positions first (primary at the far right), then draw left to right so that the
        // focus order is the visual order.
        let mut placed: Vec<(usize, u16)> = Vec::new();
        let mut right = rect.right().saturating_sub(2);
        for slot in [0usize, 1] {
            let Some(label) = self.labels()[slot] else {
                continue;
            };
            let w = Button::new(&rt.look, label, ButtonVariant::Primary).width();
            right = right.saturating_sub(w);
            placed.push((slot, right));
            right = right.saturating_sub(2);
        }
        if self.tertiary.is_some() {
            placed.push((2, rect.x + 2));
        }
        placed.sort_by_key(|(_, x)| *x);
        for (slot, bx) in placed {
            let Some(label) = self.labels()[slot] else {
                continue;
            };
            let variant = match (slot, self.danger) {
                (0, true) => ButtonVariant::Danger,
                (0, false) => ButtonVariant::Primary,
                (2, _) => ButtonVariant::Ghost,
                _ => ButtonVariant::Secondary,
            };
            let enabled = !(slot == 0 && self.blocked());
            let mut control = ButtonControl::new(self.button_id(slot), label)
                .variant(variant)
                .enabled(enabled)
                .on(on);
            if let Some(k) = Self::mnemonic(label) {
                control = control.key(k);
            }
            control.show(rt, &self.states[slot], bx, by, buf);
        }
        let _ = text_width;
    }
}

/// Ready-made message dialogs (IntelliJ `Messages`).
#[derive(Debug, Clone, Copy)]
pub struct Messages;

impl Messages {
    /// An information dialog with an OK button.
    pub fn info_dialog<M: 'static>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Dialog<M> {
        Dialog::new(title)
            .message(message)
            .message_icon(MessageIcon::Information)
            .primary("OK")
    }

    /// A warning dialog with an OK button.
    pub fn warning_dialog<M: 'static>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Dialog<M> {
        Dialog::new(title)
            .message(message)
            .message_icon(MessageIcon::Warning)
            .primary("OK")
    }

    /// An error dialog with an OK button.
    pub fn error_dialog<M: 'static>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Dialog<M> {
        Dialog::new(title)
            .message(message)
            .message_icon(MessageIcon::Error)
            .primary("OK")
    }

    /// An OK/Cancel question.
    pub fn ok_cancel_dialog<M: 'static>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Dialog<M> {
        Dialog::new(title)
            .message(message)
            .message_icon(MessageIcon::Question)
            .primary("OK")
            .secondary("Cancel")
    }

    /// A Yes/No question answered with Y and N too.
    pub fn yes_no_dialog<M: 'static>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Dialog<M> {
        Dialog::new(title)
            .message(message)
            .message_icon(MessageIcon::Question)
            .primary("Yes")
            .secondary("No")
            .yes_no_keys(true)
    }
}
