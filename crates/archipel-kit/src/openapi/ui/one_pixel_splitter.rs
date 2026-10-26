//! The splitter variants of the IntelliJ Platform: a one pixel line, and one that remembers
//! its proportion.

use super::splitter::{DividerStyle, Splitter};
use crate::host::PropertiesComponent;
use std::hash::Hash;

/// A [`Splitter`] whose divider is a 1 pt line in the separator color with a 6 pt wide hit
/// zone (IntelliJ `OnePixelSplitter`, spec section 12). Everything else is the splitter's:
/// drag, double click resets to the default share (0.5 unless
/// [`default_ratio`](Splitter::default_ratio) says otherwise), keyboard,
/// [Ctrl-click to flip the orientation](Splitter::allow_switch_orientation_by_mouse_click).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::openapi::ui::OnePixelSplitter;
///
/// let mut ratio = 0.3;
/// OnePixelSplitter::horizontal("editor", &mut ratio)
///     .show(ui, |a| { a.label("Left"); }, |b| { b.label("Right"); });
/// # });
/// ```
pub struct OnePixelSplitter;

impl OnePixelSplitter {
    /// Side by side, `ratio` is the left pane's share.
    pub fn horizontal<'a>(id_salt: impl Hash, ratio: &'a mut f32) -> Splitter<'a> {
        Splitter::horizontal(id_salt, ratio).divider_style(DividerStyle::OnePixel)
    }

    /// Stacked, `ratio` is the top pane's share.
    pub fn vertical<'a>(id_salt: impl Hash, ratio: &'a mut f32) -> Splitter<'a> {
        Splitter::vertical(id_salt, ratio).divider_style(DividerStyle::OnePixel)
    }
}

/// A [`OnePixelSplitter`] that persists its proportion under a key (IntelliJ `JBSplitter`):
/// the share is loaded from `properties` the first time the split is shown, and stored
/// whenever the user changes it. The host saves the properties.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::host::InMemoryProperties;
/// use archipel_kit::openapi::ui::JBSplitter;
///
/// let mut properties = InMemoryProperties::new();
/// let mut ratio = 0.5;
/// JBSplitter::horizontal("main", &mut ratio, "my.plugin.splitter.proportion", &mut properties)
///     .show(ui, |a| { a.label("Left"); }, |b| { b.label("Right"); });
/// # });
/// ```
pub struct JBSplitter;

impl JBSplitter {
    /// Side by side, `ratio` is the left pane's share.
    pub fn horizontal<'a>(
        id_salt: impl Hash,
        ratio: &'a mut f32,
        key: impl Into<String>,
        properties: &'a mut dyn PropertiesComponent,
    ) -> Splitter<'a> {
        OnePixelSplitter::horizontal(id_salt, ratio).persist(key, properties)
    }

    /// Stacked, `ratio` is the top pane's share.
    pub fn vertical<'a>(
        id_salt: impl Hash,
        ratio: &'a mut f32,
        key: impl Into<String>,
        properties: &'a mut dyn PropertiesComponent,
    ) -> Splitter<'a> {
        OnePixelSplitter::vertical(id_salt, ratio).persist(key, properties)
    }
}
