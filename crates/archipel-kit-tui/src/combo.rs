//! [`ComboBoxControl`]: a field with a drop-down list of options.
//!
//! Keys (IntelliJ `ComboBox`, spec section 4): <kbd>Up</kbd> and <kbd>Down</kbd> on a closed
//! combo change the selection; <kbd>Alt+Down</kbd>, <kbd>F4</kbd>, <kbd>Enter</kbd> and
//! <kbd>Space</kbd> open the list; a click anywhere opens it. The list is a [`ListPopup`]
//! (a completion menu below the field, at least as wide as the field) that the caller shows
//! as an overlay; when it ends, the caller stores the chosen index with
//! [`ComboBoxState::choose`].

use crate::components::{fill, put, text_width};
use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crate::popup::ListPopup;
use crate::runtime::Runtime;
use archipel_kit::tokens::motion;
use archipel_kit::ui::popup::BaseListPopupStep;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// The state of a combo box: the selected option and when its list last closed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComboBoxState {
    /// The selected option.
    pub selected: usize,
    closed_at: Option<f64>,
}

impl ComboBoxState {
    /// A combo with option `selected`.
    pub fn new(selected: usize) -> Self {
        Self {
            selected,
            closed_at: None,
        }
    }

    /// Stores the choice from the list popup and records that the list closed at `now`.
    pub fn choose(&mut self, index: usize, now: f64) {
        self.selected = index;
        self.closed_at = Some(now);
    }

    /// Records that the list closed without a choice.
    pub fn list_closed(&mut self, now: f64) {
        self.closed_at = Some(now);
    }
}

/// What a combo box did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComboEvent {
    /// Nothing for the caller.
    None,
    /// The selection changed with the arrow keys.
    Changed,
    /// The list should open: show [`ComboBoxControl::popup`].
    OpenList,
}

/// A combo box over a list of option labels.
#[derive(Debug, Clone)]
pub struct ComboBoxControl<'a> {
    /// The identity.
    pub id: Id,
    /// The option labels.
    pub options: &'a [String],
    /// Disabled combos ignore input.
    pub enabled: bool,
    /// The width of the field in cells.
    pub width: u16,
}

impl<'a> ComboBoxControl<'a> {
    /// A combo of `options`, 24 cells wide.
    pub fn new(id: Id, options: &'a [String]) -> Self {
        Self {
            id,
            options,
            enabled: true,
            width: 24,
        }
    }

    /// Sets the width.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Enables or disables it.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the field at (`x`, `y`) and registers it; returns its rectangle (the anchor of
    /// the list).
    pub fn show(
        &self,
        rt: &mut Runtime,
        state: &ComboBoxState,
        x: u16,
        y: u16,
        buf: &mut Buffer,
    ) -> Rect {
        let rect = Rect::new(x, y, self.width, 1);
        let p = rt.look.palette;
        let focused = rt.focused(self.id) && self.enabled;
        let hover = rt.hovered() == Some(self.id) && self.enabled;
        let bg = if hover { p.one_bg } else { p.black2 };
        fill(
            buf,
            rect,
            rect.x,
            rect.y,
            rect.width,
            Style::default().bg(bg),
        );
        if focused {
            put(
                buf,
                rect,
                rect.x,
                rect.y,
                "▌",
                Style::default().fg(p.nord_blue).bg(bg),
            );
        }
        let fg = if self.enabled { p.white } else { p.grey_fg };
        let label = self.options.get(state.selected).map_or("", String::as_str);
        let room = self.width.saturating_sub(5);
        let shown = crate::components::clip(label, room);
        put(
            buf,
            rect,
            rect.x + 2,
            rect.y,
            &shown,
            Style::default().fg(fg).bg(bg).add_modifier(if focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        );
        let arrow = rt.look.glyph(crate::Glyph::ChevronDown);
        put(
            buf,
            rect,
            rect.right().saturating_sub(3),
            rect.y,
            &arrow,
            Style::default()
                .fg(if self.enabled {
                    p.light_grey
                } else {
                    p.grey_fg
                })
                .bg(bg),
        );
        if self.enabled {
            rt.widget(self.id, rect);
        }
        let _ = text_width;
        rect
    }

    /// Applies `event`.
    pub fn handle(&self, rt: &mut Runtime, state: &mut ComboBoxState, event: &Event) -> ComboEvent {
        if !self.enabled || self.options.is_empty() {
            return ComboEvent::None;
        }
        let guard_open = |state: &ComboBoxState, now: f64| {
            state
                .closed_at
                .is_none_or(|t| now - t >= f64::from(motion::POPUP_REOPEN_GUARD))
        };
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release && rt.focused(self.id) => {
                let alt = key.modifiers.contains(KeyModifiers::ALT);
                match key.code {
                    KeyCode::Down if alt => ComboEvent::OpenList,
                    KeyCode::F(4) => ComboEvent::OpenList,
                    KeyCode::Enter | KeyCode::Char(' ') if key.modifiers.is_empty() => {
                        ComboEvent::OpenList
                    }
                    KeyCode::Up if !alt => {
                        if state.selected > 0 {
                            state.selected -= 1;
                            ComboEvent::Changed
                        } else {
                            ComboEvent::None
                        }
                    }
                    KeyCode::Down if !alt => {
                        if state.selected + 1 < self.options.len() {
                            state.selected += 1;
                            ComboEvent::Changed
                        } else {
                            ComboEvent::None
                        }
                    }
                    KeyCode::Home => {
                        state.selected = 0;
                        ComboEvent::Changed
                    }
                    KeyCode::End => {
                        state.selected = self.options.len() - 1;
                        ComboEvent::Changed
                    }
                    _ => ComboEvent::None,
                }
            }
            Event::Mouse(_) => {
                if let Some(p) = rt.pointer().copied()
                    && p.clicked(self.id)
                    && guard_open(state, rt.now)
                {
                    return ComboEvent::OpenList;
                }
                ComboEvent::None
            }
            _ => ComboEvent::None,
        }
    }

    /// The list popup for the current selection, below `anchor` (the field's rectangle from
    /// [`show`](Self::show)). Its chosen value is the option index.
    pub fn popup(&self, state: &ComboBoxState, anchor: Rect) -> ListPopup<usize> {
        let labels: Vec<String> = self.options.to_vec();
        let step =
            BaseListPopupStep::with_text("", (0..labels.len()).collect(), move |i: &usize| {
                labels.get(*i).cloned().unwrap_or_default()
            })
            .default_index(state.selected.min(self.options.len().saturating_sub(1)))
            .speed_search(true);
        ListPopup::menu_below(step, anchor)
    }
}
