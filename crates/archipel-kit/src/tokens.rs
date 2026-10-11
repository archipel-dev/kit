//! Size, spacing, radius and typography tokens shared by every component, so panels built
//! from different components line up with each other.

/// Spacing and sizes, in points.
pub mod size {
    /// Minimum central work area before docked tool windows are temporarily hidden.
    pub const WORKBENCH_CENTER_MIN: f32 = 200.0;
    /// Left/right inset of panel content.
    pub const CONTENT_INSET: f32 = 12.0;
    /// Horizontal inset of the selection/hover pill inside a row.
    pub const ROW_PILL_INSET: f32 = 4.0;
    /// List and tree row height.
    pub const ROW_HEIGHT: f32 = 24.0;
    /// Indent per tree level (19 pt in Android Studio's Islands UI).
    pub const TREE_INDENT: f32 = 19.0;
    /// Inset of a list or tree row's selection pill from the island's sides; the expander
    /// column starts at the pill's edge.
    pub const ROW_INSET: f32 = 12.0;
    /// Space between a tree row's expander column and its icon.
    pub const CHEVRON_GAP: f32 = 5.0;
    /// Width of the expander column; leaf rows reserve it too, so icons align.
    pub const CHEVRON_COLUMN: f32 = 16.0;
    /// Icon edge length.
    pub const ICON: f32 = 16.0;
    /// Gap between an icon and its text (rows, menus, tabs: 4 pt in Android Studio).
    pub const ICON_TEXT_GAP: f32 = 4.0;
    /// Gap between a primary and a secondary text in a row.
    pub const SECONDARY_GAP: f32 = 8.0;
    /// Height of an island's header: a tool window's title bar or an editor's tab bar
    /// (40 pt in Android Studio's Islands UI).
    pub const HEADER_HEIGHT: f32 = 40.0;
    /// Shared main toolbar / integrated window title height.
    pub const WINDOW_TOOLBAR_HEIGHT: f32 = 40.0;
    /// macOS native traffic-light region, including the gap before app content.
    pub const NATIVE_WINDOW_CONTROLS: f32 = 80.0;
    /// Square icon button edge.
    pub const ICON_BUTTON: f32 = 26.0;
    /// Square edge of a small icon button: actions inside rows and tabs.
    pub const ICON_BUTTON_SMALL: f32 = 20.0;
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
    /// Width of a submenu.
    pub const SUBMENU_WIDTH: f32 = 200.0;
    /// Space inside a status bar item at each end.
    pub const STATUS_ITEM_PADDING: f32 = 6.0;
    /// Gap between a popup and what it is attached to (its own padding is
    /// [`popup::PADDING`](super::popup::PADDING)).
    pub const POPUP_PADDING: f32 = 4.0;
    /// Line height of code views and editors.
    pub const CODE_LINE_HEIGHT: f32 = 20.0;
    /// Padding inside code views and editors, and between gutter and text.
    pub const CODE_PADDING: f32 = 8.0;
    /// Pitch of the dotted underline of unresolved template variables.
    pub const DOTTED_PITCH: f32 = 3.0;
    /// Edge of one dot of the dotted underline.
    pub const DOTTED_DOT: f32 = 1.0;
    /// Distance from the bottom of the line to the underline's dots.
    pub const DOTTED_INSET: f32 = 0.5;
    /// Space between a row's leading slot and its icon.
    pub const ROW_LEADING_GAP: f32 = 4.0;
    /// Space kept between a row's label and its trailing slot.
    pub const ROW_TRAILING_GAP: f32 = 12.0;
    /// Least width worth showing of a row's elided secondary text.
    pub const ROW_MIN_SECONDARY: f32 = 32.0;
    /// Width of keyboard focus rings.
    pub const FOCUS_RING: f32 = 2.0;
    /// Number of gaps between the four parts of a key/value row: checkbox, fields, delete.
    pub const KEY_VALUE_ROW_GAPS: f32 = 3.0;
    /// Share of a key/value table's field width that goes to the key column; the value
    /// column gets the rest.
    pub const KEY_COLUMN_SHARE: f32 = 0.4;
    /// Thickness of a split pane's divider: the gap between its panes, like an island gap.
    pub const SPLIT_DIVIDER: f32 = ISLAND_GAP;
    /// Extra pointer reach on each side of a split pane's divider, so it is easy to grab.
    pub const SPLIT_GRAB: f32 = 2.0;
    /// Distance an arrow key moves a split pane's divider.
    pub const SPLIT_KEY_STEP: f32 = 16.0;
    /// How far past its minimum size a split pane is dragged before it collapses.
    pub const SPLIT_COLLAPSE: f32 = 40.0;
    /// Default minimum size of each side of a split pane.
    pub const SPLIT_MIN_PANE: f32 = 120.0;
    /// Default height of a vertical histogram.
    pub const CHART_HEIGHT: f32 = 120.0;
    /// Height of a sparkline: it sits inline with text.
    pub const SPARKLINE_HEIGHT: f32 = 20.0;
    /// Default width of a sparkline.
    pub const SPARKLINE_WIDTH: f32 = 96.0;

    /// Distance from a row's left edge to the first column at `depth`.
    pub fn indent(depth: usize) -> f32 {
        CONTENT_INSET + depth as f32 * TREE_INDENT
    }
}

/// Charts (histograms, sparklines), in points.
pub mod chart {
    /// Thickness of a horizontal histogram bar.
    pub const BAR_HEIGHT: f32 = 12.0;
    /// Inset of a sparkline's plot from its edges, leaving room for markers.
    pub const SPARKLINE_PAD: f32 = 3.0;
    /// Radius of sparkline markers (last value, extremes, hover).
    pub const DOT_RADIUS: f32 = 2.5;
    /// Extra radius of the island-colored ring around a marker.
    pub const DOT_RING: f32 = 1.0;
    /// Stroke width of a sparkline.
    pub const LINE_WIDTH: f32 = 1.5;
    /// Stroke width of hairlines: axis baselines.
    pub const HAIRLINE: f32 = 1.0;
}

/// Popups and menus, in points. Measured on Android Studio's Islands UI.
pub mod popup {
    /// Space above the first and below the last item of a popup.
    pub const PADDING: f32 = 8.0;
    /// Inset of an item's highlight pill (and of separators) from the popup's sides.
    pub const ITEM_INSET: f32 = 8.0;
    /// Space inside the pill before the icon column and after the shortcut or chevron.
    pub const ITEM_PADDING: f32 = 8.0;
    /// Height of a section header row ("Open Projects").
    pub const HEADER_HEIGHT: f32 = 24.0;
    /// Height of a separator row; the line is on its fifth pixel row.
    pub const SEPARATOR_HEIGHT: f32 = 9.0;
    /// Icon edge of a two-line item (an item with a description).
    pub const LARGE_ICON: f32 = 20.0;
    /// Height of the description line under a two-line item's label.
    pub const DESCRIPTION_HEIGHT: f32 = 24.0;
}

/// Editor tabs, in points. Measured on Android Studio's Islands UI.
pub mod tab {
    /// Height of a tab's pill, centered in the header.
    pub const HEIGHT: f32 = 28.0;
    /// Space between two tabs' pills.
    pub const GAP: f32 = 6.0;
    /// Space from the pill's left edge to the icon (or the title).
    pub const PADDING_LEFT: f32 = 8.0;
    /// Space from the close button to the pill's right edge.
    pub const PADDING_RIGHT: f32 = 4.0;
    /// Space between the title and the close button.
    pub const CLOSE_GAP: f32 = 4.0;
    /// Edge of the close button's box.
    pub const CLOSE: f32 = 16.0;
    /// Edge the close cross is drawn in: smaller than an icon, as in Android Studio.
    pub const CLOSE_GLYPH: f32 = 10.0;
    /// Radius of the round hover behind the close button.
    pub const CLOSE_HOVER_RADIUS: f32 = 9.0;
}

/// Tool window stripes (the icon bars beside the islands), in points. Measured on Android
/// Studio's Islands UI.
pub mod stripe {
    /// Width of a stripe, between the window edge and the island gap.
    pub const WIDTH: f32 = 38.0;
    /// Edge of a stripe button.
    pub const BUTTON: f32 = 30.0;
    /// Edge of a stripe button's icon.
    pub const ICON: f32 = 20.0;
    /// Distance between the tops of two stripe buttons.
    pub const PITCH: f32 = 40.0;
    /// Space above the first button (below the window toolbar).
    pub const PADDING: f32 = 3.0;
}

/// Motion: delays and durations, in seconds. Measured on Android Studio's Islands UI.
pub mod motion {
    /// How long an island's header actions stay after the pointer leaves the island (and
    /// focus is elsewhere), so passing over a gap does not flicker them. They appear at once.
    pub const ISLAND_ACTIONS_HIDE_DELAY: f32 = 0.2;
    /// The fade-out of an island's header actions once the hide delay has passed.
    pub const ISLAND_ACTIONS_FADE_OUT: f32 = 0.07;
}

/// Opacities, from 0 to 1.
pub mod opacity {
    /// Opacity of a row's [`tint`](crate::components::ListRow::tint) over the island.
    pub const ROW_TINT: f32 = 0.22;
    /// Multiplier of the colors of disabled code and highlighted text.
    pub const DISABLED_CODE: f32 = 0.55;
    /// Multiplier of a color used as a tinted fill behind text of the same color.
    pub const TINT_FILL: f32 = 0.18;
    /// Opacity of a sparkline's min/max band.
    pub const CHART_BAND: f32 = 0.12;
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
    /// Editor tab pills.
    pub const TAB: u8 = 6;
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
