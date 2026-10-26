//! Notifications.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`Notification`] | A notification card: icon, title, body and actions. |
//! | [`NotificationsManager`] ([`NotificationId`], [`NotificationEvent`]) | The balloon stack: timers, hover pause, queue, history. |
//! | [`NotificationGroup`] ([`NotificationDisplayType`], [`NotificationType`], [`NotificationAction`]) | How a kind of notification is shown. |

mod card;
mod manager;
mod model;

pub use card::Notification;
pub use manager::{NotificationEvent, NotificationEventKind, NotificationId, NotificationsManager};
pub use model::{NotificationAction, NotificationDisplayType, NotificationGroup, NotificationType};
