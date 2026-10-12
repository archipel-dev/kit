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
    /// Square icon button edge: actions in island headers and toolbars inside islands
    /// (22 pt in Android Studio's Islands UI).
    pub const ICON_BUTTON: f32 = 22.0;
    /// Space between an island header's last action and the island's right edge.
    pub const HEADER_ACTIONS_INSET: f32 = 10.0;
    /// Space between a tool window's title and its view switcher chevron.
    pub const TITLE_CHEVRON_GAP: f32 = 2.0;
    /// Length of the vertical line between groups of header actions.
    pub const HEADER_DIVIDER: f32 = 20.0;
    /// Square edge of a small icon button: actions inside rows and tabs.
    pub const ICON_BUTTON_SMALL: f32 = 20.0;
    /// A field's clear button: a round 16 pt target (measured on Android Studio's search
    /// fields).
    pub const CLEAR_BUTTON: f32 = 16.0;
    /// Width of a field's completion list.
    pub const COMPLETION_WIDTH: f32 = 280.0;
    /// Distance from a field's right edge to the center of its clear button.
    pub const CLEAR_BUTTON_INSET: f32 = 15.0;
    /// Distance from a combo box's right edge to the center of its chevron (where
    /// [`Dropdown`](crate::components::Dropdown) draws it too).
    pub const COMBO_CHEVRON_INSET: f32 = 16.0;
    /// Size the close glyph is drawn at in a clear button, so its cross is 8 pt.
    pub const CLEAR_GLYPH: f32 = 13.0;
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
    /// Settings forms (measured on Android Studio's Settings pages): gap between rows. With
    /// 24 pt checkbox rows this gives their 29 pt pitch.
    pub const FORM_ROW_GAP: f32 = 5.0;
    /// Settings forms: space above and below a row of fields (28 pt): a field sits 9 pt
    /// from the box of a checkbox above or below it, 9 pt below a group title's row.
    pub const FORM_FIELD_INSET: f32 = 4.0;
    /// Settings forms: gap between independent controls in one row, e.g. radio buttons.
    pub const FORM_CONTROL_GAP: f32 = 16.0;
    /// Settings forms: space to add between independent fields in a row of labelled
    /// fields, on top of [`FORM_LABEL_GAP`], making [`FORM_CONTROL_GAP`].
    pub const FORM_CONTROL_SPACE: f32 = FORM_CONTROL_GAP - FORM_LABEL_GAP;
    /// Settings forms: extra space between rows that are only loosely related (measured
    /// between `Zoom:` and `Use custom font:` in Settings | Appearance).
    pub const FORM_ROWS_GAP: f32 = 7.0;
    /// Settings forms: gap between a row's label (`Theme:`) and its control.
    pub const FORM_LABEL_GAP: f32 = 6.0;
    /// Settings forms: space above a group title, on top of [`FORM_ROW_GAP`].
    pub const FORM_GROUP_SPACING: f32 = 20.0;
    /// Settings forms: indent of a group's rows under its title.
    pub const FORM_GROUP_INDENT: f32 = 24.0;
    /// Default width of a [`NumberField`](crate::components::NumberField): room for six
    /// digits, as Android Studio's integer fields.
    pub const NUMBER_FIELD_WIDTH: f32 = 64.0;
    /// Gap between a group title and its line.
    pub const GROUP_LINE_GAP: f32 = 6.0;
    /// Gap between a checkbox or radio and its label.
    pub const CHECK_LABEL_GAP: f32 = 8.0;
    /// Space between a control's row and the first line of its comment below.
    pub const COMMENT_GAP: f32 = 3.0;
    /// A comment below a control takes this much space besides its lines (above and
    /// below them together).
    pub const COMMENT_SPACE: f32 = 7.0;
    /// Line height of comments (12 pt text).
    pub const COMMENT_LINE_HEIGHT: f32 = 16.0;
    /// Gap between a label and its inline comment (`Requires restart`).
    pub const INLINE_COMMENT_GAP: f32 = 12.0;
    /// Padding inside dialogs, popups and banners.
    pub const DIALOG_PADDING: f32 = 14.0;
    /// Gap between islands (the frame shows between them).
    pub const ISLAND_GAP: f32 = 4.0;
    /// Width of a submenu.
    pub const SUBMENU_WIDTH: f32 = 200.0;
    /// Space inside a status bar item at each end.
    pub const STATUS_ITEM_PADDING: f32 = 6.0;
    /// Height of a status bar item's hover pill (21 pt in Android Studio).
    pub const STATUS_ITEM_HEIGHT: f32 = 21.0;
    /// Space inside a tooltip at its sides (16 pt in Android Studio).
    pub const TOOLTIP_PADDING_X: f32 = 16.0;
    /// Space inside a tooltip above and below its text.
    pub const TOOLTIP_PADDING_Y: f32 = 9.0;
    /// Space between a tooltip's text and its shortcut.
    pub const TOOLTIP_SHORTCUT_GAP: f32 = 9.0;
    /// Height of a one-line banner (Android Studio's editor notifications: 39 pt).
    pub const BANNER_HEIGHT: f32 = 39.0;
    /// Space inside a banner before its icon.
    pub const BANNER_PADDING: f32 = 16.0;
    /// Space inside a banner after its last action.
    pub const BANNER_PADDING_END: f32 = 20.0;
    /// Space between a banner's icon and its text.
    pub const BANNER_ICON_GAP: f32 = 10.0;
    /// Space between a banner's action links.
    pub const BANNER_ACTION_GAP: f32 = 22.0;
    /// Width of a notification card (370 pt in Android Studio).
    pub const NOTIFICATION_WIDTH: f32 = 370.0;
    /// Space inside a notification card.
    pub const NOTIFICATION_PADDING: f32 = 12.0;
    /// Space inside a breadcrumb at each end.
    pub const BREADCRUMB_PADDING: f32 = 4.0;
    /// Space between a breadcrumb and the chevron after it.
    pub const BREADCRUMB_GAP: f32 = 2.0;
    /// Height of the status bar on the window frame below the islands.
    pub const STATUS_BAR_HEIGHT: f32 = 30.0;
    /// Space between the window's left edge and the status bar's first item.
    pub const STATUS_BAR_INSET: f32 = 8.0;
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
    /// Space between a tool window's title and its first content tab.
    pub const CONTENT_TITLE_GAP: f32 = 22.0;
    /// Space inside a content tab at each end.
    pub const CONTENT_PADDING: f32 = 10.0;
    /// Space between two content tabs.
    pub const CONTENT_GAP: f32 = 10.0;
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

/// The main window toolbar's controls, in points. Measured on Android Studio's Islands UI.
pub mod toolbar {
    /// Height of toolbar buttons and the edge of toolbar icon buttons.
    pub const BUTTON: f32 = 30.0;
    /// Edge of a toolbar icon.
    pub const ICON: f32 = 20.0;
    /// Space before a toolbar button's text.
    pub const PADDING: f32 = 10.0;
    /// Space before a toolbar button's leading icon, and after its chevron.
    pub const ICON_PADDING: f32 = 8.0;
    /// Space between a leading icon and the text.
    pub const ICON_GAP: f32 = 6.0;
    /// Space between the text and the dropdown chevron.
    pub const CHEVRON_GAP: f32 = 2.0;
    /// Space between toolbar controls.
    pub const GAP: f32 = 10.0;
    /// Space between the window's edges and the toolbar's first and last controls: the
    /// main menu button sits right above the stripe buttons.
    pub const WINDOW_PADDING: f32 = 6.0;
}

/// The project color's glow on the window frame, in points from the window's top-left
/// corner. Measured on Android Studio's Islands UI.
pub mod glow {
    /// Horizontal position of the glow's center (behind the project widget).
    pub const CENTER_X: f32 = 140.0;
    /// Vertical position of the glow's center (in the toolbar).
    pub const CENTER_Y: f32 = 20.0;
    /// Horizontal radius: it fades out about 550 pt to the right.
    pub const RADIUS_X: f32 = 520.0;
    /// Vertical radius: it reaches over the top of the stripes.
    pub const RADIUS_Y: f32 = 150.0;
    /// Opacity of the project color at the center (dark themes).
    pub const ALPHA: f32 = 0.26;
    /// Opacity of the glow at the center in light themes.
    pub const LIGHT_ALPHA: f32 = 0.36;
    /// How far light themes mix the project color toward white.
    pub const LIGHT_PASTEL: f32 = 0.6;
}

/// Scroll bars, measured on Android Studio's Islands UI.
pub mod scroll {
    /// Thickness of a scroll bar's thumb.
    pub const THUMB: f32 = 7.0;
    /// Space between the thumb and the edge of what scrolls.
    pub const MARGIN: f32 = 2.0;
    /// Opacity of the text color the thumb is drawn in, at rest (`#3C3D3E` on a dark island).
    pub const THUMB_OPACITY: f32 = 0.19;
    /// Opacity of the thumb under the pointer (`#565657` on a dark island).
    pub const THUMB_HOVER_OPACITY: f32 = 0.33;
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
    /// Opacity of the black shadow under popups and windows in dark themes.
    pub const SHADOW_DARK: f32 = 90.0 / 255.0;
    /// Opacity of the black shadow under popups and windows in light themes.
    pub const SHADOW_LIGHT: f32 = 40.0 / 255.0;
    /// Opacity of the black scrim behind a modal dialog in dark themes.
    pub const SCRIM_DARK: f32 = 120.0 / 255.0;
    /// Opacity of the black scrim behind a modal dialog in light themes.
    pub const SCRIM_LIGHT: f32 = 60.0 / 255.0;
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
    /// Main toolbar and stripe buttons (30 pt).
    pub const ICON_BUTTON: u8 = 6;
    /// Icon buttons inside islands (22 pt).
    pub const ICON_BUTTON_SMALL: u8 = 4;
    /// Editor tab pills.
    pub const TAB: u8 = 6;
    /// Checkboxes (3 pt in Android Studio).
    pub const CHECKBOX: u8 = 3;
    /// Histogram bars and sparkline bands.
    pub const CHART_BAR: u8 = 2;
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
    /// Comments under and next to settings controls.
    pub const COMMENT: f32 = 12.0;
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
