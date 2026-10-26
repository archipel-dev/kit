//! IDE-level helpers.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`HelpTooltip`] ([`HelpTooltipExt`], [`HelpTooltipResponse`]) | The kit's tooltip: a label, a shortcut, a description and a link. |
//! | [`GotItTooltip`] ([`GotItPosition`], [`GotItDismissal`], [`GotItResponse`]) | A one-shot onboarding bubble pointing at a control. |

mod got_it_tooltip;
mod help_tooltip;

pub use got_it_tooltip::{GotItDismissal, GotItPosition, GotItResponse, GotItTooltip};
pub use help_tooltip::{HelpTooltip, HelpTooltipExt, HelpTooltipResponse};
