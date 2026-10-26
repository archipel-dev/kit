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
//! | [`SimpleColoredComponent`] | Icon, label, secondary and trailing text of any row. |
//! | [`Slot`] | A place in a component that takes the caller's icon, text or widgets. |
//! | [`SimpleTextAttributes`], [`TextFragment`] | Styled fragments of text: regular, bold, grayed, link, error, search match. |
//! | [`nav`] | Arrow, Home/End and PgUp/PgDn movement through items. |
//! | [`ScrollBar`] | The scroll bar of collections, with the IDE's hover fade timing. |
//! | [`scroll_vertical`] | Scroll areas that behave like desktop ones. |
//! | [`focus_ring`], [`row_pill`] | The focus ring and the rounded row highlight. |
//! | [`project_glow`] | The project color's glow on the window frame. |
//!
//! Interaction that is not part of a component's look (tooltips, context menus, drag and
//! drop) is added with [`behaviour`](crate::behaviour). See `docs/building-components.md`
//! for a walk-through.

mod glow;
pub mod nav;
mod row;
mod scroll;
mod slot;
mod state;
mod surface;
mod text_attributes;
mod tone;

pub use glow::project_glow;
pub use row::{IconPainter, SimpleColoredComponent};
pub use scroll::ScrollBar;
pub use slot::Slot;
pub use state::{State, fill_gradient, focus_ring, paint_expander, row_pill};
pub use surface::{Surface, desktop, scroll_vertical};
pub use text_attributes::{SimpleTextAttributes, TextFragment};
pub(crate) use text_attributes::{layout_run, paint_ranges};
pub use tone::{Tone, ValueKind};
