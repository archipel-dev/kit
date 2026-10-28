//! [`ToolStripeControl`]: a [`model::ToolStripe`] as a column of three-cell buttons
//! (TUI-GUI-055..062). Each button is one row high with one blank row between buttons (the
//! GUI's 40 px pitch); the top group starts at the first row, the bottom group is bottom
//! aligned and listed from the bottom edge upward, a separator row (`─` in the middle cell)
//! follows the top group. When the column is too short the blank rows go first (pitch 1),
//! then the outermost buttons of the bottom group are clipped.
//!
//! An active (shown) button fills its three cells with the accent and draws its glyph in
//! the text-on-accent color; a shown window that does not have focus is the neutral gray; the
//! others draw the icon color with the hover and pressed fills. A badge is a `●` in the
//! button's right cell.
//!
//! A click toggles the window, a middle click or Shift+click hides it, a right click asks
//! for the stripe menu.

use crate::chrome::{self, Placed, Tokens};
use crate::components::{fill, put};
use crate::hit::Id;
use crate::input::{Event, MouseButton};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use archipel_kit_model::{Outline, OutlineEntry, StripeButton, StripeEvent, Tone, ToolStripe};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// Cells of a stripe.
pub const STRIPE_WIDTH: u16 = 3;

/// The interactive wrapper of a [`ToolStripe`].
#[derive(Debug, Clone)]
pub struct ToolStripeControl {
    /// The identity; buttons use child ids.
    pub id: Id,
    placed: Vec<Placed>,
    pressed: Option<Id>,
}

impl ToolStripeControl {
    /// A control.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            placed: Vec::new(),
            pressed: None,
        }
    }

    /// Where the button `key` is.
    pub fn rect_of(&self, key: &str) -> Option<Rect> {
        self.placed.iter().find(|p| p.key == key).map(|p| p.rect)
    }

    /// The outline of what was drawn, top group first.
    pub fn outline(&self, _buf: &Buffer) -> Outline {
        Outline {
            entries: self.placed.iter().map(|p| p.entry.clone()).collect(),
        }
    }

    /// The rows of the buttons: (button, y) with `pitch`.
    fn layout(props: &ToolStripe, area: Rect) -> (Vec<(&StripeButton, u16)>, Option<u16>) {
        let top = props.top.len() as u16;
        let bottom = props.bottom.len() as u16;
        let need = |pitch: u16| {
            let group = |n: u16| if n == 0 { 0 } else { (n - 1) * pitch + 1 };
            group(top) + u16::from(top > 0 && bottom > 0) + group(bottom)
        };
        let pitch = if need(2) <= area.height { 2 } else { 1 };
        let mut out = Vec::new();
        let mut y = area.y;
        for button in &props.top {
            if y < area.bottom() {
                out.push((button, y));
            }
            y += pitch;
        }
        let separator = (top > 0 && bottom > 0).then_some(y.saturating_sub(pitch) + 1);
        let mut y = area.bottom().saturating_sub(1);
        for button in &props.bottom {
            let top_limit = separator.map_or(area.y, |s| s + 1);
            if y >= top_limit && y < area.bottom() {
                out.push((button, y));
            }
            if y < pitch {
                break;
            }
            y -= pitch;
        }
        (out, separator.filter(|s| *s < area.bottom()))
    }

    /// Draws the stripe in `area` (three columns wide) over the `frame` surface.
    pub fn draw(&mut self, rt: &mut Runtime, props: &ToolStripe, area: Rect, buf: &mut Buffer) {
        self.placed.clear();
        let look = rt.look.clone();
        let c = &look.theme.colors;
        let notation = chrome::notation(rt);
        let t = Tokens::new(&look, c.frame);
        for y in area.top()..area.bottom() {
            fill(
                buf,
                area,
                area.x,
                y,
                area.width,
                t.style(t.of(&look, c.text)),
            );
        }
        let (buttons, separator) = Self::layout(props, area);
        if let Some(y) = separator {
            put(
                buf,
                area,
                area.x + area.width / 2,
                y,
                "─",
                t.style(t.of(&look, c.separator)),
            );
        }
        for (button, y) in buttons {
            let rect = Rect::new(area.x, y, area.width.min(STRIPE_WIDTH), 1);
            let id = chrome::child(self.id, &button.id);
            let hovered = rt.hovered() == Some(id);
            let pressed = self.pressed == Some(id) && hovered;
            let (bg, fg) = if button.selected && button.focused {
                (t.of(&look, c.accent), t.of(&look, c.text_on_accent))
            } else if button.selected {
                (
                    t.of(&look, c.selection_inactive),
                    t.of(&look, c.text_strong),
                )
            } else if pressed {
                (t.of(&look, c.pressed), t.icon(&look, Tone::Default))
            } else if hovered {
                (t.of(&look, c.hover), t.icon(&look, Tone::Default))
            } else {
                (t.bg, t.icon(&look, Tone::Default))
            };
            let style = Style::default().bg(bg).fg(fg);
            fill(buf, rect, rect.x, rect.y, rect.width, style);
            let (ch, color) = chrome::icon_cell(&look, &t, &button.icon, Tone::Default);
            let color = if button.selected { fg } else { color };
            put(
                buf,
                rect,
                rect.x + rect.width / 2,
                rect.y,
                &ch,
                style.fg(color),
            );
            if let Some(badge) = button.badge {
                chrome::badge(&look, &t, buf, rect, rect.right() - 1, rect.y, badge, bg);
            }
            if button.selected && button.focused {
                // Bold the glyph so an active button also reads without color.
                if let Some(cell) = buf.cell_mut((rect.x + rect.width / 2, rect.y)) {
                    cell.modifier |= Modifier::BOLD;
                }
            }
            rt.clickable(id, rect);
            rt.tooltip
                .offer(id, rect, chrome::tooltip(&button.tooltip, notation));
            let mut entry =
                OutlineEntry::new("stripe", button.id.clone(), button.tooltip.title.clone());
            entry.checked = Some(button.selected);
            entry.badge = button.badge.map(|b| format!("{b:?}").to_ascii_lowercase());
            entry.tooltip = Some(button.tooltip.text(notation));
            entry.shortcut = button.tooltip.shortcut.as_ref().map(|s| s.label(notation));
            self.placed.push(Placed {
                key: button.id.clone(),
                id,
                rect,
                entry,
            });
        }
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> Vec<StripeEvent> {
        let Event::Mouse(_) = event else {
            return Vec::new();
        };
        let Some(p) = rt.pointer().copied() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        match p.kind {
            PointerKind::Down(MouseButton::Left) => self.pressed = p.target,
            PointerKind::Up(MouseButton::Left) => {
                self.pressed = None;
                for placed in &self.placed {
                    if p.clicked(placed.id) {
                        out.push(if p.shift() {
                            StripeEvent::Hide {
                                id: placed.key.clone(),
                            }
                        } else {
                            StripeEvent::Toggle {
                                id: placed.key.clone(),
                            }
                        });
                    }
                }
            }
            PointerKind::Down(MouseButton::Middle) => {
                for placed in &self.placed {
                    if p.over(placed.id) {
                        out.push(StripeEvent::Hide {
                            id: placed.key.clone(),
                        });
                    }
                }
            }
            PointerKind::Down(MouseButton::Right) => {
                for placed in &self.placed {
                    if p.over(placed.id) {
                        out.push(StripeEvent::Context {
                            id: placed.key.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
        out
    }
}
