//! Size, spacing, radius and typography tokens shared by every component, so panels built
//! from different components line up with each other.

/// Spacing and sizes, in points.
pub mod size {
    /// Left/right inset of panel content.
    pub const CONTENT_INSET: f32 = 12.0;
    /// Horizontal inset of the selection/hover pill inside a row.
    pub const ROW_PILL_INSET: f32 = 4.0;
    /// List and tree row height.
    pub const ROW_HEIGHT: f32 = 24.0;
    /// Indent per tree level.
    pub const TREE_INDENT: f32 = 18.0;
    /// Width of the expander column; leaf rows reserve it too, so icons align.
    pub const CHEVRON_COLUMN: f32 = 16.0;
    /// Icon edge length.
    pub const ICON: f32 = 16.0;
    /// Gap between an icon and its text.
    pub const ICON_TEXT_GAP: f32 = 6.0;
    /// Gap between a primary and a secondary text in a row.
    pub const SECONDARY_GAP: f32 = 8.0;
    /// Panel title bar height.
    pub const HEADER_HEIGHT: f32 = 31.0;
    /// Square icon button edge.
    pub const ICON_BUTTON: f32 = 26.0;
    /// Gap between toolbar items.
    pub const TOOLBAR_GAP: f32 = 4.0;
    /// Height of buttons, text fields and combo boxes.
    pub const CONTROL_HEIGHT: f32 = 28.0;
    /// Height of small (compact) buttons.
    pub const CONTROL_HEIGHT_SMALL: f32 = 24.0;
    /// Horizontal padding inside buttons.
    pub const BUTTON_PADDING: f32 = 14.0;
    /// Vertical gap between stacked sections.
    pub const SECTION_SPACING: f32 = 8.0;
    /// Padding inside dialogs, popups and banners.
    pub const DIALOG_PADDING: f32 = 14.0;
    /// Gap between islands (the frame shows between them).
    pub const ISLAND_GAP: f32 = 4.0;
    /// Width of keyboard focus rings.
    pub const FOCUS_RING: f32 = 2.0;

    /// Distance from a row's left edge to the first column at `depth`.
    pub fn indent(depth: usize) -> f32 {
        CONTENT_INSET + depth as f32 * TREE_INDENT
    }
}

/// Corner radii, in points.
pub mod radius {
    /// Islands (panels).
    pub const ISLAND: u8 = 10;
    /// Popups, menus, dialogs and notifications.
    pub const POPUP: u8 = 8;
    /// Buttons and inputs.
    pub const CONTROL: u8 = 4;
    /// Banners and callouts.
    pub const BANNER: u8 = 6;
    /// Editor tab underline ends.
    pub const UNDERLINE: u8 = 2;
    /// Toolbar icon buttons.
    pub const ICON_BUTTON: u8 = 6;
    /// Row selection pills.
    pub const ROW: u8 = 4;
    /// Fully rounded (pills, counters, toggles).
    pub const PILL: u8 = 255;
}

/// Font sizes and families.
pub mod typography {
    use egui::{FontFamily, FontId};

    /// Default UI text size.
    pub const BODY: f32 = 13.0;
    /// Small captions and badges.
    pub const SMALL: f32 = 11.0;
    /// Section and dialog titles.
    pub const TITLE: f32 = 13.0;
    /// Large headings (empty states, welcome screens).
    pub const HEADING: f32 = 18.0;
    /// Code text size.
    pub const CODE: f32 = 13.0;

    /// Family name of the semibold UI font.
    pub const SEMIBOLD: &str = "semibold";
    /// Family name of the bold monospace font.
    pub const MONO_BOLD: &str = "mono-bold";

    /// Semibold UI font of `size`. Falls back to the regular font when the kit's fonts
    /// are not installed in `ctx` (see [`crate::install`]), instead of failing.
    pub fn semibold(ctx: &egui::Context, size: f32) -> FontId {
        named(ctx, SEMIBOLD, size, FontFamily::Proportional)
    }

    /// Bold monospace font of `size`, falling back to the regular monospace font.
    pub fn mono_bold(ctx: &egui::Context, size: f32) -> FontId {
        named(ctx, MONO_BOLD, size, FontFamily::Monospace)
    }

    fn named(ctx: &egui::Context, name: &str, size: f32, fallback: FontFamily) -> FontId {
        let family = FontFamily::Name(name.into());
        let installed = ctx.fonts(|fonts| fonts.definitions().families.contains_key(&family));
        FontId::new(size, if installed { family } else { fallback })
    }

    /// Regular UI font of `size`.
    pub fn body(size: f32) -> FontId {
        FontId::proportional(size)
    }
}
