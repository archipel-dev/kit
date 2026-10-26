//! HelpTooltip: a small float with a title, the shortcut and an optional description, that
//! appears after the pointer rests on its owner (500 ms) or the owner has keyboard focus,
//! and hides 150 ms after the pointer leaves, on a press or a key, or by itself after 10 s
//! (30 s with a description). It looks like an LSP hover float in Neovim.

use crate::Look;
use crate::components::{fill_rect, put, text_width};
use crate::hit::Id;
use archipel_kit::tokens::motion;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// The content of a tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tooltip {
    /// The title: the action text.
    pub title: String,
    /// The shortcut text, shown grey at the right of the title.
    pub shortcut: Option<String>,
    /// The longer explanation.
    pub description: Option<String>,
}

impl Tooltip {
    /// A tooltip with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Adds the shortcut text.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Adds a description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn dismiss_after(&self) -> f64 {
        if self.description.is_some() {
            f64::from(motion::HELP_TOOLTIP_DISMISS_MULTI_LINE)
        } else {
            f64::from(motion::HELP_TOOLTIP_DISMISS_SINGLE_LINE)
        }
    }
}

#[derive(Debug, Clone)]
struct Offer {
    id: Id,
    rect: Rect,
    tip: Tooltip,
}

#[derive(Debug, Clone)]
struct Shown {
    id: Id,
    owner: Rect,
    tip: Tooltip,
    at: f64,
    rect: Rect,
}

/// The tooltip state machine. Owners [`offer`](Self::offer) their tooltip every frame while
/// they draw; the runtime calls [`update`](Self::update) with the hovered and focused widget.
#[derive(Debug, Clone, Default)]
pub struct TooltipState {
    offers: Vec<Offer>,
    target: Option<Id>,
    since: f64,
    left_at: Option<f64>,
    shown: Option<Shown>,
    dismissed: bool,
}

impl TooltipState {
    pub(crate) fn begin_frame(&mut self) {
        self.offers.clear();
    }

    /// Offers `tip` for the widget `id` drawn at `rect`.
    pub fn offer(&mut self, id: Id, rect: Rect, tip: Tooltip) {
        self.offers.push(Offer { id, rect, tip });
    }

    /// Hides the tooltip until the pointer enters another owner (a press or key).
    pub fn dismiss(&mut self) {
        self.shown = None;
        self.dismissed = true;
    }

    /// Whether a tooltip is showing.
    pub fn is_showing(&self) -> bool {
        self.shown.is_some()
    }

    /// The showing tooltip's title.
    pub fn showing_title(&self) -> Option<&str> {
        self.shown.as_ref().map(|s| s.tip.title.as_str())
    }

    /// The next time `update` has something to do, for the loop's deadline.
    pub fn deadline(&self) -> Option<f64> {
        if let Some(shown) = &self.shown {
            let hide = self
                .left_at
                .map(|t| t + f64::from(motion::HELP_TOOLTIP_HIDE_DELAY));
            let dismiss = shown.at + shown.tip.dismiss_after();
            return Some(hide.map_or(dismiss, |h| h.min(dismiss)));
        }
        let target = self.target?;
        if self.dismissed || !self.offers.iter().any(|o| o.id == target) {
            return None;
        }
        Some(self.since + f64::from(motion::HELP_TOOLTIP_SHOW_DELAY))
    }

    /// Advances the state machine. `hovered` is the widget under the pointer, `focused` the
    /// widget with keyboard focus (a focused owner shows its tooltip too).
    pub fn update(&mut self, now: f64, hovered: Option<Id>, focused: Option<Id>) {
        let offered = |id: Option<Id>| id.filter(|id| self.offers.iter().any(|o| o.id == *id));
        let candidate = offered(hovered).or_else(|| offered(focused));
        if candidate != self.target {
            self.target = candidate;
            self.since = now;
            self.dismissed = false;
            if self.shown.as_ref().is_some_and(|s| Some(s.id) != candidate) {
                // Pointer left the owner: hide after the hide delay unless another owner
                // takes over, which hides it at once.
                match candidate {
                    Some(_) => self.shown = None,
                    None => self.left_at = Some(now),
                }
            }
        }
        if candidate.is_some() {
            self.left_at = None;
        }
        if let Some(left) = self.left_at
            && now - left >= f64::from(motion::HELP_TOOLTIP_HIDE_DELAY)
        {
            self.shown = None;
            self.left_at = None;
        }
        if let Some(shown) = &self.shown
            && now - shown.at >= shown.tip.dismiss_after()
        {
            self.shown = None;
            self.dismissed = true;
        }
        if self.shown.is_none()
            && !self.dismissed
            && let Some(target) = self.target
            && now - self.since >= f64::from(motion::HELP_TOOLTIP_SHOW_DELAY)
            && let Some(offer) = self.offers.iter().find(|o| o.id == target)
        {
            self.shown = Some(Shown {
                id: target,
                owner: offer.rect,
                tip: offer.tip.clone(),
                at: now,
                rect: Rect::default(),
            });
        }
        // Keep the content current while it shows.
        if let Some(shown) = &mut self.shown
            && let Some(offer) = self.offers.iter().find(|o| o.id == shown.id)
        {
            shown.owner = offer.rect;
            shown.tip = offer.tip.clone();
        }
    }

    /// Draws the tooltip above everything, below or above its owner, inside `screen`.
    pub fn draw(&mut self, look: &Look, screen: Rect, buf: &mut Buffer) {
        let Some(shown) = &mut self.shown else {
            return;
        };
        let p = &look.palette;
        let tip = &shown.tip;
        let desc_width = 40u16;
        let desc: Vec<String> = tip
            .description
            .as_deref()
            .map(|d| crate::components::wrap_text(d, desc_width))
            .unwrap_or_default();
        let head =
            text_width(&tip.title) + tip.shortcut.as_deref().map_or(0, |s| text_width(s) + 2);
        let inner_w = desc
            .iter()
            .map(|l| text_width(l))
            .max()
            .unwrap_or(0)
            .max(head);
        let width = (inner_w + 4).min(screen.width);
        let height = (2 + 1 + desc.len() as u16).min(screen.height);
        if width < 3 || height < 3 {
            return;
        }
        let below = shown.owner.bottom();
        let y = if below + height <= screen.bottom() {
            below
        } else {
            shown.owner.y.saturating_sub(height)
        };
        let x = shown
            .owner
            .x
            .min(screen.right().saturating_sub(width))
            .max(screen.x);
        let rect = Rect::new(x, y.max(screen.y), width, height);
        shown.rect = rect;
        let bg = p.one_bg;
        fill_rect(buf, rect, Style::default().bg(bg).fg(p.white));
        frame(buf, rect, Style::default().fg(p.grey).bg(bg));
        let title = Style::default()
            .fg(p.white)
            .bg(bg)
            .add_modifier(Modifier::BOLD);
        let inner = Rect::new(
            rect.x + 2,
            rect.y + 1,
            rect.width.saturating_sub(4),
            rect.height - 2,
        );
        let end = put(buf, inner, inner.x, inner.y, &tip.title, title);
        if let Some(shortcut) = &tip.shortcut {
            put(
                buf,
                inner,
                end + 2,
                inner.y,
                shortcut,
                Style::default().fg(p.light_grey).bg(bg),
            );
        }
        for (i, line) in desc.iter().enumerate() {
            put(
                buf,
                inner,
                inner.x,
                inner.y + 1 + i as u16,
                line,
                Style::default().fg(p.light_grey).bg(bg),
            );
        }
    }

    /// The rectangle of the showing tooltip, as of the last draw.
    pub fn rect(&self) -> Option<Rect> {
        self.shown.as_ref().map(|s| s.rect)
    }
}

/// Draws a thin rounded border on the edge cells of `rect`.
pub(crate) fn frame(buf: &mut Buffer, rect: Rect, style: Style) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    let area = rect.intersection(buf.area);
    let set = |buf: &mut Buffer, x: u16, y: u16, s: &str| {
        if area.contains((x, y).into())
            && let Some(cell) = buf.cell_mut((x, y))
        {
            cell.set_symbol(s).set_style(style);
        }
    };
    let (l, r, t, b) = (rect.x, rect.right() - 1, rect.y, rect.bottom() - 1);
    for x in l + 1..r {
        set(buf, x, t, "─");
        set(buf, x, b, "─");
    }
    for y in t + 1..b {
        set(buf, l, y, "│");
        set(buf, r, y, "│");
    }
    set(buf, l, t, "╭");
    set(buf, r, t, "╮");
    set(buf, l, b, "╰");
    set(buf, r, b, "╯");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered(state: &mut TooltipState, id: Id) {
        state.begin_frame();
        state.offer(
            id,
            Rect::new(5, 2, 4, 1),
            Tooltip::new("Run").shortcut("Ctrl+R"),
        );
    }

    #[test]
    fn shows_after_the_delay_and_hides_after_leaving() {
        let id = Id::new("run");
        let mut s = TooltipState::default();
        offered(&mut s, id);
        s.update(0.0, Some(id), None);
        s.update(0.4, Some(id), None);
        assert!(!s.is_showing());
        s.update(0.5, Some(id), None);
        assert!(s.is_showing());
        s.update(1.0, None, None);
        assert!(s.is_showing(), "hides only after 150 ms");
        s.update(1.16, None, None);
        assert!(!s.is_showing());
    }

    #[test]
    fn a_press_hides_it_until_another_owner() {
        let id = Id::new("run");
        let mut s = TooltipState::default();
        offered(&mut s, id);
        s.update(0.0, Some(id), None);
        s.update(0.6, Some(id), None);
        assert!(s.is_showing());
        s.dismiss();
        s.update(5.0, Some(id), None);
        assert!(!s.is_showing());
    }

    #[test]
    fn keyboard_focus_shows_it_too() {
        let id = Id::new("run");
        let mut s = TooltipState::default();
        offered(&mut s, id);
        s.update(0.0, None, Some(id));
        s.update(0.5, None, Some(id));
        assert!(s.is_showing());
    }

    #[test]
    fn hides_by_itself_after_ten_seconds() {
        let id = Id::new("run");
        let mut s = TooltipState::default();
        offered(&mut s, id);
        s.update(0.0, Some(id), None);
        s.update(0.5, Some(id), None);
        s.update(10.4, Some(id), None);
        assert!(s.is_showing());
        s.update(10.5, Some(id), None);
        assert!(!s.is_showing());
    }
}
