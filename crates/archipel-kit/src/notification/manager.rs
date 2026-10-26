//! The balloon stack: which notifications are on screen, for how long, and where.

use super::{Notification, NotificationDisplayType};
use crate::tokens::{motion, size};
use egui::{Rect, vec2};

/// Names one notification of a [`NotificationsManager`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NotificationId(u64);

/// What happened to a notification during [`NotificationsManager::show`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationEvent {
    /// The notification.
    pub id: NotificationId,
    /// What happened to it.
    pub kind: NotificationEventKind,
}

/// See [`NotificationEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotificationEventKind {
    /// The user performed the action at this index. The notification expired, unless the
    /// action [keeps it](super::NotificationAction::keep_notification).
    Action(usize),
    /// The user closed the balloon with its close button or <kbd>Esc</kbd>.
    Dismissed,
    /// The balloon went away by itself: its time ran out, or the user clicked outside of
    /// it. The notification stays in the [history](NotificationsManager::history).
    Hidden,
}

/// One notification and its timer.
#[derive(Debug, Clone)]
struct Entry {
    id: NotificationId,
    notification: Notification,
    /// Seconds of display left; counts down only while the balloon is not hovered and the
    /// window is active. `None` until the balloon first shows.
    remaining: Option<f64>,
    /// When the balloon was last advanced, to count the time that passed.
    last_tick: f64,
    /// The pointer was over the balloon last frame.
    hovered: bool,
    /// Where the balloon was drawn last frame.
    rect: Option<Rect>,
    /// A button or link of the balloon had the keyboard focus last frame. (egui drops the
    /// focus when <kbd>Esc</kbd> arrives, before the balloon is drawn.)
    focused: bool,
    /// The pass in which the balloon first showed, so the click that created it does not
    /// hide it.
    first_pass: Option<u64>,
    /// When the fade-out began.
    fading_since: Option<f64>,
}

impl Entry {
    fn delay(&self) -> f64 {
        match self.notification.notification_group().display_type() {
            NotificationDisplayType::StickyBalloon => {
                f64::from(motion::NOTIFICATION_STICKY_HIDE_DELAY)
            }
            _ => f64::from(motion::NOTIFICATION_HIDE_DELAY),
        }
    }

    fn sticky(&self) -> bool {
        self.notification.notification_group().display_type()
            == NotificationDisplayType::StickyBalloon
    }
}

/// The notifications of the window and the balloons they show: IntelliJ's
/// `Notifications.Bus` and `BalloonLayout` in one caller-owned value.
///
/// Notifications are added with [`NotificationsManager::notify`]; every frame
/// [`NotificationsManager::show`] draws the balloons and reports what the user did. The
/// rules are IntelliJ's:
///
/// - at most [`size::NOTIFICATION_VISIBLE_COUNT`] (2) balloons are visible, the oldest at
///   the top and the newest at the bottom of a stack in the bottom-right corner; the others
///   wait and start their timer when they get a place;
/// - a balloon hides after 10 s, a [sticky](super::NotificationGroup::sticky_balloon_group)
///   one after 300 s. The timer stops while the pointer is over the balloon (and starts over
///   when it leaves) or the window is not active;
/// - the close button or <kbd>Esc</kbd> (with focus in the balloon) closes it;
///   a click outside hides regular balloons ([`NotificationsManager::hide_on_click_outside`]);
/// - performing an action expires the notification unless the action keeps it;
/// - groups of display type [`None`](NotificationDisplayType::None) are never shown.
///
/// Hidden and expired notifications go to the [history](NotificationsManager::history).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::notification::{Notification, NotificationsManager};
/// use archipel_kit::ui::Status;
///
/// // Keep the manager in your application state.
/// let mut notifications = NotificationsManager::new();
/// notifications.notify(Notification::new(Status::Success, "Build finished"));
/// for event in notifications.show(ui.ctx()) {
///     // react to `event.kind`
/// }
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct NotificationsManager {
    entries: Vec<Entry>,
    history: Vec<Notification>,
    next_id: u64,
    hide_on_click_outside: bool,
    bottom_inset: f32,
}

impl Default for NotificationsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Notifications kept in the history.
const HISTORY_LIMIT: usize = 200;

impl NotificationsManager {
    /// An empty manager.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            history: Vec::new(),
            next_id: 0,
            hide_on_click_outside: true,
            bottom_inset: 0.0,
        }
    }

    /// Whether a click outside hides regular balloons (default `true`; sticky balloons
    /// ignore outside clicks either way).
    pub fn hide_on_click_outside(mut self, hide: bool) -> Self {
        self.hide_on_click_outside = hide;
        self
    }

    /// Extra space between the stack and the window's bottom edge, e.g. the status bar's
    /// height.
    pub fn bottom_inset(mut self, inset: f32) -> Self {
        self.bottom_inset = inset;
        self
    }

    /// Adds a notification. It shows now if there is room, else when a balloon goes away.
    pub fn notify(&mut self, notification: Notification) -> NotificationId {
        let id = NotificationId(self.next_id);
        self.next_id += 1;
        if notification.notification_group().display_type() == NotificationDisplayType::None {
            self.log(notification);
        } else {
            self.entries.push(Entry {
                id,
                notification,
                remaining: None,
                last_tick: 0.0,
                hovered: false,
                rect: None,
                focused: false,
                first_pass: None,
                fading_since: None,
            });
        }
        id
    }

    /// Removes a notification, shown or waiting, without a trace in the history
    /// (IntelliJ `Notification.expire`). Returns whether it was there.
    pub fn expire(&mut self, id: NotificationId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != before
    }

    /// The notification, if it is shown or waiting.
    pub fn get(&self, id: NotificationId) -> Option<&Notification> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| &entry.notification)
    }

    /// The notifications on screen, oldest first.
    pub fn visible(&self) -> Vec<NotificationId> {
        self.entries
            .iter()
            .take(size::NOTIFICATION_VISIBLE_COUNT)
            .map(|entry| entry.id)
            .collect()
    }

    /// The notifications waiting for a place, oldest first.
    pub fn queued(&self) -> Vec<NotificationId> {
        self.entries
            .iter()
            .skip(size::NOTIFICATION_VISIBLE_COUNT)
            .map(|entry| entry.id)
            .collect()
    }

    /// Hidden notifications and those of log-only groups, oldest first (the last 200).
    pub fn history(&self) -> &[Notification] {
        &self.history
    }

    /// Forgets the history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    fn log(&mut self, notification: Notification) {
        self.history.push(notification);
        if self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
    }

    /// Advances the timers and draws the balloons in the bottom-right corner of the window.
    /// Call it once per frame. Returns what the user did.
    pub fn show(&mut self, ctx: &egui::Context) -> Vec<NotificationEvent> {
        let mut events = Vec::new();
        if self.entries.is_empty() {
            return events;
        }
        let now = ctx.input(|i| i.time);
        let pass = ctx.cumulative_pass_nr();
        let active = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let pressed_at = ctx.input(|i| {
            i.pointer
                .any_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        let fade = f64::from(motion::NOTIFICATION_FADE_OUT);
        // A press that landed on no balloon at all.
        let outside_press = pressed_at.is_some_and(|p| {
            !self
                .entries
                .iter()
                .any(|entry| entry.rect.is_some_and(|rect| rect.contains(p)))
        });

        // Timers of the visible balloons.
        let visible = self.entries.len().min(size::NOTIFICATION_VISIBLE_COUNT);
        for entry in self.entries.iter_mut().take(visible) {
            let delay = entry.delay();
            let first = entry.remaining.is_none();
            let mut remaining = entry.remaining.unwrap_or(delay);
            if first {
                entry.last_tick = now;
                entry.first_pass = Some(pass);
            }
            let elapsed = (now - entry.last_tick).max(0.0);
            entry.last_tick = now;
            let hovered = pointer.is_some_and(|p| entry.rect.is_some_and(|r| r.contains(p)));
            if hovered {
                entry.hovered = true;
            } else if entry.hovered {
                // The pointer left: the full time starts over.
                entry.hovered = false;
                remaining = delay;
            } else if active {
                remaining -= elapsed;
            }
            entry.remaining = Some(remaining);
            // A click outside every balloon hides the regular ones, but not the one that
            // just appeared.
            let outside_click = self.hide_on_click_outside
                && !entry.sticky()
                && entry.first_pass.is_some_and(|first| first < pass)
                && outside_press;
            if entry.fading_since.is_none() && (remaining <= 0.0 || outside_click) {
                entry.fading_since = Some(now);
            }
        }

        // Draw the visible balloons, oldest first so the newest ends at the bottom.
        let max_height = ctx.content_rect().height() * size::NOTIFICATION_MAX_HEIGHT_SHARE;
        let mut drawn: Vec<(NotificationId, Rect, crate::ui::MessageResponse)> = Vec::new();
        egui::Area::new(egui::Id::new("archipel-notification-balloons"))
            .order(egui::Order::Foreground)
            // The stack itself is not a target; its cards and their buttons are.
            .sense(egui::Sense::hover())
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                vec2(
                    -size::NOTIFICATION_MARGIN,
                    -size::NOTIFICATION_MARGIN - self.bottom_inset,
                ),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = size::NOTIFICATION_STACK_GAP;
                for entry in self.entries.iter().take(visible) {
                    let opacity = entry.fading_since.map_or(1.0, |since| {
                        (1.0 - (now - since) / fade).clamp(0.0, 1.0) as f32
                    });
                    let inner = ui.push_id(entry.id, |ui| {
                        ui.multiply_opacity(opacity);
                        entry
                            .notification
                            .clone()
                            .show_limited(ui, Some((max_height - 120.0).max(size::ROW_HEIGHT)))
                    });
                    drawn.push((entry.id, inner.response.rect, inner.inner));
                }
            });

        // Apply what happened, newest first so indices stay valid.
        let mut remove = Vec::new();
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        for (id, rect, response) in drawn {
            let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
                continue;
            };
            entry.rect = Some(rect);
            let had_focus = std::mem::replace(&mut entry.focused, response.focused);
            let escape_closes = escape && (response.focused || had_focus);
            if let Some(index) = response.action {
                events.push(NotificationEvent {
                    id,
                    kind: NotificationEventKind::Action(index),
                });
                let keep = entry
                    .notification
                    .actions()
                    .get(index)
                    .is_some_and(|action| action.keep);
                if !keep {
                    remove.push((id, false));
                }
            } else if response.dismissed || escape_closes {
                if escape_closes {
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                }
                events.push(NotificationEvent {
                    id,
                    kind: NotificationEventKind::Dismissed,
                });
                remove.push((id, true));
            } else if entry.fading_since.is_some_and(|since| now - since >= fade) {
                events.push(NotificationEvent {
                    id,
                    kind: NotificationEventKind::Hidden,
                });
                remove.push((id, true));
            }
        }
        for (id, log) in remove {
            if let Some(position) = self.entries.iter().position(|entry| entry.id == id) {
                let entry = self.entries.remove(position);
                if log {
                    self.log(entry.notification);
                }
            }
        }

        // Wake up when the first timer runs out; fades repaint every frame.
        let mut wake = f64::INFINITY;
        for entry in self.entries.iter().take(size::NOTIFICATION_VISIBLE_COUNT) {
            if entry.fading_since.is_some() {
                wake = 0.0;
            } else if !entry.hovered && active {
                wake = wake.min(entry.remaining.unwrap_or(0.0).max(0.0));
            }
        }
        if wake == 0.0 {
            ctx.request_repaint();
        } else if wake.is_finite() {
            ctx.request_repaint_after_secs(wake as f32);
        }
        events
    }
}
