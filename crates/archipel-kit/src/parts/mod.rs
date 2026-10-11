//! The building blocks the kit's components are made of, public so new components are
//! built from them instead of from scratch. A component built on these looks, reacts and
//! reads like the rest of the kit by construction; `testing::conformance` and
//! `testing::audit` check the rest.
//!
//! | Part | Use it for |
//! |---|---|
//! | [`Tone`] | Text and icon colors by meaning (error, muted, link …), never raw colors. |
//! | [`Surface`] | Islands, popups, tooltips, dialogs, field boxes. |
//! | [`State`] | Hover, press, focus, selection and disabled → fills and text colors. |
//! | [`RowContent`] | Icon, label, secondary and trailing text of any row. |
//! | [`Slot`] | A place in a component that takes the caller's icon, text or widgets. |
//! | [`Popover`] | Popups attached to a widget that close on Escape and outside clicks. |
//! | [`nav`] | Arrow, Home/End and PgUp/PgDn movement through items. |
//! | [`scroll_vertical`] | Scroll areas that behave like desktop ones. |
//! | [`focus_ring`], [`row_pill`] | The focus ring and the rounded row highlight. |
//!
//! Interaction that is not part of a component's look (tooltips, context menus, drag and
//! drop) is added with [`behaviour`](crate::behaviour). See `docs/building-components.md`
//! for a walk-through.

pub mod nav;
mod popover;
mod row;
mod slot;
mod state;
mod surface;
mod tone;

pub use popover::Popover;
pub use row::RowContent;
pub use slot::Slot;
pub use state::{State, focus_ring, row_pill};
pub use surface::{Surface, desktop, scroll_vertical};
pub use tone::Tone;
