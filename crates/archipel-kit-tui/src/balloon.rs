//! Notification balloons: the kit's [`NotificationsManager`] state drawn as nvim-notify
//! floats in the top-right corner, with the GUI's timing (spec section 10.1).
//!
//! - At most two balloons show (newest last), the rest wait in the queue.
//! - A regular balloon hides after 10 s, a sticky one after 300 s. The timer pauses while the
//!   terminal window is not focused, stops while the pointer is over the balloon and starts
//!   over when it leaves.
//! - A balloon fades out for 200 ms before it goes.
//! - The close icon dismisses; an action link performs the action and expires the
//!   notification unless the action keeps it; a press outside hides the regular balloons;
//!   <kbd>Esc</kbd> closes a balloon that has focus (a click gives it focus).
//!
//! The timers live here because the kit's manager keeps them inside its egui `show`.

use crate::components::{fill_rect, put, text_width, wrap_text};
use crate::hit::Id;
use crate::input::{Event, KeyCode, MouseButton};
use crate::overlay::{Handled, Overlay};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use crate::tooltip::frame;
use archipel_kit::notification::{
    Notification, NotificationDisplayType, NotificationEvent, NotificationEventKind,
    NotificationId, NotificationsManager,
};
use archipel_kit::tokens::{motion, size};
use archipel_kit::ui::Status;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

/// Width of a balloon in cells.
pub const BALLOON_WIDTH: u16 = 46;

#[derive(Debug, Clone, Copy)]
struct Timer {
    remaining: f64,
    last_tick: f64,
    hovered: bool,
    fading_since: Option<f64>,
}

/// The balloon stack.
pub struct Balloons {
    /// The kit's notification state: entries, queue, history.
    pub manager: NotificationsManager,
    bodies: HashMap<NotificationId, String>,
    timers: HashMap<NotificationId, Timer>,
    rects: Vec<(NotificationId, Rect, Vec<Rect>, Rect)>,
    focused: Option<NotificationId>,
    pointer: Option<(u16, u16)>,
    events: Vec<NotificationEvent>,
}

impl Default for Balloons {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether `action` keeps its notification open. The kit has no getter for the flag, so it is
/// read from the `Debug` output; a test pins the format.
fn keeps_notification(action: &impl std::fmt::Debug) -> bool {
    format!("{action:?}").contains("keep: true")
}

impl Balloons {
    /// An empty stack.
    pub fn new() -> Self {
        Self {
            manager: NotificationsManager::new(),
            bodies: HashMap::new(),
            timers: HashMap::new(),
            rects: Vec::new(),
            focused: None,
            pointer: None,
            events: Vec::new(),
        }
    }

    /// Shows `notification` with an optional body text (the kit's `Notification` has no body
    /// getter, so the body is passed here too).
    pub fn notify(&mut self, notification: Notification, body: Option<&str>) -> NotificationId {
        let id = self.manager.notify(notification);
        if let Some(body) = body {
            self.bodies.insert(id, body.to_owned());
        }
        id
    }

    /// Takes the events since the last call.
    pub fn take_events(&mut self) -> Vec<NotificationEvent> {
        std::mem::take(&mut self.events)
    }

    fn delay(&self, id: NotificationId) -> f64 {
        match self
            .manager
            .get(id)
            .map(|n| n.notification_group().display_type())
        {
            Some(NotificationDisplayType::StickyBalloon) => {
                f64::from(motion::NOTIFICATION_STICKY_HIDE_DELAY)
            }
            _ => f64::from(motion::NOTIFICATION_HIDE_DELAY),
        }
    }

    fn sticky(&self, id: NotificationId) -> bool {
        self.delay(id) > f64::from(motion::NOTIFICATION_HIDE_DELAY)
    }

    fn dismiss(&mut self, id: NotificationId, kind: NotificationEventKind, now: f64) {
        if let Some(t) = self.timers.get_mut(&id) {
            if t.fading_since.is_none() {
                t.fading_since = Some(now);
                self.events.push(NotificationEvent { id, kind });
            }
        } else if self.manager.expire(id) {
            self.events.push(NotificationEvent { id, kind });
        }
    }

    fn remove(&mut self, id: NotificationId) {
        self.manager.expire(id);
        self.timers.remove(&id);
        self.bodies.remove(&id);
        if self.focused == Some(id) {
            self.focused = None;
        }
    }

    /// Advances the timers to `now`.
    pub fn update(&mut self, now: f64, window_active: bool) {
        let visible = self.manager.visible();
        let pointer_over: Vec<NotificationId> = self
            .rects
            .iter()
            .filter(|(_, r, _, _)| self.hover_in(*r))
            .map(|(id, ..)| *id)
            .collect();
        for id in visible {
            let delay = self.delay(id);
            let hovered = pointer_over.contains(&id);
            let timer = self.timers.entry(id).or_insert(Timer {
                remaining: delay,
                last_tick: now,
                hovered: false,
                fading_since: None,
            });
            let elapsed = (now - timer.last_tick).max(0.0);
            timer.last_tick = now;
            if hovered {
                timer.hovered = true;
            } else if timer.hovered {
                timer.hovered = false;
                timer.remaining = delay;
            } else if window_active && timer.fading_since.is_none() {
                timer.remaining -= elapsed;
                if timer.remaining <= 0.0 {
                    timer.fading_since = Some(now);
                    self.events.push(NotificationEvent {
                        id,
                        kind: NotificationEventKind::Hidden,
                    });
                }
            }
        }
        let fade = f64::from(motion::NOTIFICATION_FADE_OUT);
        let done: Vec<NotificationId> = self
            .timers
            .iter()
            .filter(|(_, t)| t.fading_since.is_some_and(|s| now - s >= fade))
            .map(|(id, _)| *id)
            .collect();
        for id in done {
            self.remove(id);
        }
    }

    fn hover_in(&self, rect: Rect) -> bool {
        self.pointer
            .is_some_and(|(x, y)| rect.contains((x, y).into()))
    }

    /// The next time [`update`](Self::update) has something to do.
    pub fn deadline(&self) -> Option<f64> {
        self.timers
            .values()
            .filter(|t| !t.hovered)
            .map(|t| match t.fading_since {
                Some(s) => s + f64::from(motion::NOTIFICATION_FADE_OUT),
                None => t.last_tick + t.remaining,
            })
            .reduce(f64::min)
    }
}

fn severity(status: Status, p: &crate::NvChadPalette) -> (Color, crate::Glyph) {
    match status {
        Status::Info => (p.blue, crate::Glyph::Info),
        Status::Success => (p.green, crate::Glyph::Success),
        Status::Warning => (p.yellow, crate::Glyph::Warning),
        Status::Error => (p.red, crate::Glyph::Error),
        Status::Promo => (p.purple, crate::Glyph::Sparkle),
    }
}

impl<M> Overlay<M> for Balloons {
    fn modal(&self) -> bool {
        false
    }

    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<M> {
        match event {
            Event::Key(k) if k.code == KeyCode::Esc => {
                if let Some(id) = self.focused {
                    self.dismiss(id, NotificationEventKind::Dismissed, rt.now);
                    return Handled::consumed();
                }
                Handled::ignored()
            }
            Event::Mouse(_) => {
                let Some(p) = rt.pointer().copied() else {
                    return Handled::ignored();
                };
                self.pointer = Some((p.x, p.y));
                if p.kind != PointerKind::Down(MouseButton::Left) {
                    // Hover only; the app keeps the event.
                    return Handled::ignored();
                }
                let pos = (p.x, p.y).into();
                let hit = self
                    .rects
                    .iter()
                    .find(|(_, r, _, _)| r.contains(pos))
                    .cloned();
                let Some((id, _, actions, close)) = hit else {
                    // A press outside hides the regular balloons.
                    let regular: Vec<NotificationId> = self
                        .manager
                        .visible()
                        .into_iter()
                        .filter(|id| !self.sticky(*id))
                        .collect();
                    for id in regular {
                        self.dismiss(id, NotificationEventKind::Hidden, rt.now);
                    }
                    self.focused = None;
                    return Handled::ignored();
                };
                self.focused = Some(id);
                if close.contains(pos) {
                    self.dismiss(id, NotificationEventKind::Dismissed, rt.now);
                    return Handled::consumed();
                }
                for (i, rect) in actions.iter().enumerate() {
                    if rect.contains(pos) {
                        let keep = self
                            .manager
                            .get(id)
                            .and_then(|n| n.actions().get(i))
                            .is_some_and(keeps_notification);
                        self.events.push(NotificationEvent {
                            id,
                            kind: NotificationEventKind::Action(i),
                        });
                        if !keep {
                            self.dismiss(id, NotificationEventKind::Dismissed, rt.now);
                        }
                        return Handled::consumed();
                    }
                }
                Handled::consumed()
            }
            _ => Handled::ignored(),
        }
    }

    fn tick(&mut self, rt: &mut Runtime) -> Vec<M> {
        self.update(rt.now, rt.window_focused);
        Vec::new()
    }

    fn deadline(&self) -> Option<f64> {
        Balloons::deadline(self)
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        self.pointer = rt.pointer_pos();
        self.rects.clear();
        let p = rt.look.palette;
        let width = BALLOON_WIDTH.min(area.width.saturating_sub(2));
        if width < 12 {
            return;
        }
        let x = area.right().saturating_sub(width + 1);
        let mut y = area.y + 1;
        let visible = self.manager.visible();
        for id in visible.iter().take(size::NOTIFICATION_VISIBLE_COUNT) {
            let Some(notification) = self.manager.get(*id) else {
                continue;
            };
            let (color, glyph) = severity(notification.status(), &p);
            let body_lines = self
                .bodies
                .get(id)
                .map(|b| wrap_text(b, width - 4))
                .unwrap_or_default();
            let has_actions = !notification.actions().is_empty();
            let height = (2 + 1 + body_lines.len() as u16 + u16::from(has_actions))
                .min(area.height.saturating_sub(y));
            if height < 3 {
                break;
            }
            let rect = Rect::new(x, y, width, height);
            let fading = self.timers.get(id).and_then(|t| t.fading_since).is_some();
            let bg = p.one_bg;
            let fg = if fading { p.grey_fg } else { p.white };
            fill_rect(buf, rect, Style::default().bg(bg).fg(fg));
            frame(
                buf,
                rect,
                Style::default()
                    .fg(if fading { p.grey } else { color })
                    .bg(bg),
            );
            let title_style = Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD);
            let ix = put(
                buf,
                rect,
                rect.x + 2,
                rect.y + 1,
                &rt.look.glyph(glyph),
                Style::default().fg(color).bg(bg),
            );
            put(
                buf,
                rect,
                ix + 1,
                rect.y + 1,
                notification.title(),
                title_style,
            );
            let close = Rect::new(rect.right().saturating_sub(4), rect.y + 1, 2, 1);
            let close_hover = rt.hovered() == Some(Id::new("balloon-close").with(id_key(*id)));
            put(
                buf,
                rect,
                close.x,
                close.y,
                &rt.look.glyph(crate::Glyph::Close),
                Style::default()
                    .fg(if close_hover { p.red } else { p.light_grey })
                    .bg(bg),
            );
            rt.clickable(Id::new("balloon-close").with(id_key(*id)), close);
            for (i, line) in body_lines.iter().enumerate() {
                put(
                    buf,
                    rect,
                    rect.x + 2,
                    rect.y + 2 + i as u16,
                    line,
                    Style::default().fg(p.light_grey).bg(bg),
                );
            }
            let mut action_rects = Vec::new();
            if has_actions {
                let ay = rect.y + 2 + body_lines.len() as u16;
                let mut ax = rect.x + 2;
                for action in notification.actions() {
                    let w = text_width(action.text());
                    let r = Rect::new(ax, ay, w, 1);
                    put(
                        buf,
                        rect,
                        ax,
                        ay,
                        action.text(),
                        Style::default()
                            .fg(p.blue)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    );
                    action_rects.push(r);
                    ax += w + 2;
                }
            }
            rt.hits.add(rect, Id::new("balloon").with(id_key(*id)));
            self.rects.push((*id, rect, action_rects, close));
            y += height;
        }
        let queued = self.manager.queued().len();
        if queued > 0 && y < area.bottom() {
            let text = format!(" +{queued} more ");
            let w = text_width(&text);
            put(
                buf,
                area,
                area.right().saturating_sub(w + 1),
                y,
                &text,
                Style::default().fg(p.light_grey).bg(p.one_bg),
            );
        }
    }
}

fn id_key(id: NotificationId) -> u64 {
    // The id has no accessor; its Debug output is `NotificationId(n)`.
    format!("{id:?}")
        .trim_start_matches("NotificationId(")
        .trim_end_matches(')')
        .parse()
        .unwrap_or(0)
}
