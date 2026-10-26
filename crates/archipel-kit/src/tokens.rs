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
    /// [`ComboBox`](crate::openapi::ui::ComboBox) draws it too).
    pub const COMBO_CHEVRON_INSET: f32 = 16.0;
    /// Size the close glyph is drawn at in a clear button, so its cross is 8 pt.
    pub const CLEAR_GLYPH: f32 = 13.0;
    /// Gap between toolbar items.
    pub const TOOLBAR_GAP: f32 = 4.0;
    /// Smallest width of a push button (`Button.minimumSize` 72 x 28, IntelliJ spec section
    /// 13.2).
    pub const BUTTON_MIN_WIDTH: f32 = 72.0;
    /// How much a pressed push button is darkened (Swing's pressed button model, IntelliJ spec
    /// section 13.2; the amount is the kit's own choice, not specified).
    pub const BUTTON_PRESSED_DARKEN: f32 = 0.12;
    /// Smallest width of a combo box (`ComboBox.minimumSize` 49 x 28, IntelliJ spec section 4).
    pub const COMBO_MIN_WIDTH: f32 = 49.0;
    /// Space between a combo box's left edge and its text (`ComboBox.padding` left 9).
    pub const COMBO_PADDING_LEFT: f32 = 9.0;
    /// Rows a combo box's list shows before it scrolls (`getMaximumRowCount`, IntelliJ spec
    /// section 4).
    pub const COMBO_MAX_ROWS: usize = 10;
    /// Width of a spinner's arrow area at the right edge of its field.
    pub const SPINNER_ARROW_AREA: f32 = 20.0;
    /// Edge of a spinner arrow's glyph.
    pub const SPINNER_ARROW_GLYPH: f32 = 12.0;
    /// UI DSL `verticalSmallGap` (`Gap::Small`, IntelliJ UI DSL spacing).
    pub const DSL_GAP_SMALL: f32 = 8.0;
    /// UI DSL `verticalMediumGap` (`Gap::Medium`); also the measured space above a group title.
    pub const DSL_GAP_MEDIUM: f32 = 20.0;
    /// UI DSL `horizontalColumnsGap` (`RightGap::Columns`).
    pub const DSL_COLUMNS_GAP: f32 = 60.0;
    /// UI DSL `buttonGroupHeaderBottomGap`: space under the title of a buttons group.
    pub const DSL_BUTTONS_GROUP_HEADER_GAP: f32 = 2.0;
    /// Segments a segmented button shows before it becomes a combo box
    /// (`DEFAULT_MAX_BUTTONS_COUNT`; the value is not in the spec notes, IntelliJ's is 6).
    pub const SEGMENTED_MAX_BUTTONS: usize = 6;
    /// Smallest width of a text field (`TextField.minimumSize` 49 x 28, IntelliJ spec section 6).
    pub const TEXT_FIELD_MIN_WIDTH: f32 = 49.0;
    /// Width of a search field's leading icon area: a click inside it opens the history (IntelliJ
    /// spec section 6: the first 28 px).
    pub const SEARCH_ICON_AREA: f32 = 28.0;
    /// Entries a search field's history keeps (`SearchTextField` history length, IntelliJ spec
    /// section 6: 5).
    pub const SEARCH_HISTORY_LENGTH: usize = 5;
    /// Gap between the icons of a text field's extensions (`getIconGap`; the spec gives no
    /// default, the kit uses the checkbox label gap).
    pub const EXTENSION_GAP: f32 = 6.0;
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
    /// Padding inside a [`DropZone`](crate::ui::DropZone).
    pub const DROP_ZONE_PADDING: f32 = 8.0;
    /// Smallest height of a [`DropZone`](crate::ui::DropZone): room for its hint.
    pub const DROP_ZONE_MIN_HEIGHT: f32 = 40.0;
    /// Default width of a [`JBIntSpinner`](crate::ui::JBIntSpinner): `Spinner.minimumSize`
    /// 72 x 28 (IntelliJ spec section 13.4), room for five digits besides the arrows.
    pub const NUMBER_FIELD_WIDTH: f32 = 72.0;
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
    /// Width of a notification card: 360 (`BalloonLayoutConfiguration`, IntelliJ spec section
    /// 10.1; Android Studio's card measured 370 pt with its border).
    pub const NOTIFICATION_WIDTH: f32 = 360.0;
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
    /// Width of a classic `Splitter` divider (IntelliJ spec section 12: 7 px). The Islands
    /// layout uses the 4 pt gap ([`SPLIT_DIVIDER`]) instead.
    pub const SPLIT_CLASSIC_DIVIDER: f32 = 7.0;
    /// Width of a one pixel splitter's visible line (IntelliJ spec section 12).
    pub const SPLIT_ONE_PIXEL: f32 = 1.0;
    /// Width of a one pixel splitter's hit zone (`ide.splitter.mouseZone`, spec section 12).
    pub const SPLIT_MOUSE_ZONE: f32 = 6.0;
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
    // Overlays family (IntelliJ UX spec sections 7, 10, 11 and 13.8).
    /// Gap between the buttons of a dialog (`BASE_BUTTON_GAP`, IntelliJ spec section 7).
    pub const DIALOG_BUTTON_GAP: f32 = 12.0;
    /// Edge of the icon of a message dialog (IntelliJ spec section 7: 32 x 32).
    pub const MESSAGE_ICON: f32 = 32.0;
    /// Most notification balloons on screen at once (`ide.notification.visible.count`).
    pub const NOTIFICATION_VISIBLE_COUNT: usize = 2;
    /// Gap between stacked notification balloons. Not in the spec: the kit's own choice.
    pub const NOTIFICATION_STACK_GAP: f32 = 8.0;
    /// Distance of the balloon stack from the window's right and bottom edge (kit measured).
    pub const NOTIFICATION_MARGIN: f32 = 16.0;
    /// Share of the window's height a notification balloon may take before its text
    /// scrolls (IntelliJ spec section 10.1: 75%).
    pub const NOTIFICATION_MAX_HEIGHT_SHARE: f32 = 0.75;
    /// Space inside an editor notification panel above and below its line
    /// (`Editor.Notification.borderInsets`, IntelliJ spec section 10.2).
    pub const EDITOR_NOTIFICATION_PADDING_Y: f32 = 10.0;
    /// Space inside an editor notification panel at its sides (same key).
    pub const EDITOR_NOTIFICATION_PADDING_X: f32 = 12.0;
    /// Space between an editor notification panel's icon and text (spec section 10.2).
    pub const EDITOR_NOTIFICATION_ICON_GAP: f32 = 8.0;
    /// Space after an editor notification panel's text, before its links (spec section 10.2).
    pub const EDITOR_NOTIFICATION_TEXT_END: f32 = 20.0;
    /// Widest text of a help tooltip (`HelpTooltip.maxWidth`, IntelliJ spec section 11.1).
    pub const HELP_TOOLTIP_MAX_WIDTH: f32 = 250.0;
    /// Gap between the parts of a help tooltip: title, description, link.
    pub const HELP_TOOLTIP_GAP: f32 = 6.0;
    /// Edge of the external-link icon after a help tooltip's link.
    pub const HELP_TOOLTIP_LINK_ICON: f32 = 12.0;
    /// Widest text of a got-it tooltip (IntelliJ spec section 11.2).
    pub const GOT_IT_MAX_WIDTH: f32 = 280.0;
    /// Space inside a got-it tooltip, at its sides (`GotItTooltip.insets`, spec 11.2).
    pub const GOT_IT_INSET_X: f32 = 8.0;
    /// Space inside a got-it tooltip, above and below.
    pub const GOT_IT_INSET_Y: f32 = 7.0;
    /// Space between a got-it tooltip's text and its button row (`buttonTopInset`).
    pub const GOT_IT_BUTTON_TOP: f32 = 12.0;
    /// Space below a got-it tooltip's button row (`buttonBottomInset`).
    pub const GOT_IT_BUTTON_BOTTOM: f32 = 6.0;
    /// Space between a got-it tooltip's step number and its text.
    pub const GOT_IT_STEP_GAP: f32 = 6.0;
    /// Length of a got-it tooltip's arrow (pointer size 16 x 8, spec section 11.2).
    pub const GOT_IT_ARROW_LENGTH: f32 = 8.0;
    /// Base of a got-it tooltip's arrow.
    pub const GOT_IT_ARROW_WIDTH: f32 = 16.0;
    /// Distance from the corner to the arrow's middle on a `BottomLeft` tooltip (`getArrowShift`:
    /// 15 plus the arc).
    pub const GOT_IT_ARROW_SHIFT: f32 = 15.0;
    /// Height of a progress bar's track. Kept from the measurement of Android Studio: IntelliJ
    /// spec section 13.8 says the key `ProgressBar.height` is not known.
    pub const PROGRESS_BAR_HEIGHT: f32 = 4.0;
    /// Height a progress bar takes in a layout, around its track.
    pub const PROGRESS_BAR_ROW: f32 = 8.0;
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
    /// Narrowest a popup list is (kept from the measured menus; IntelliJ spec §5.3 names 170
    /// for hint popups).
    pub const MIN_WIDTH: f32 = 180.0;
    /// Width of the chevron zone at a row's right end: a press there opens the sub-step
    /// without choosing the row (IntelliJ spec §5.2).
    pub const CHEVRON_HIT: f32 = 24.0;
    /// Space between a row's mnemonic hint and its icon column (IntelliJ spec §5.3,
    /// `ActionsList` mnemonic gaps; not measured on Android Studio).
    pub const MNEMONIC_GAP: f32 = 8.0;
    /// Distance the pointer may move horizontally away from an open submenu before the
    /// "moving to the submenu" guard ends (IntelliJ spec §5.3, `ActionMenu`).
    pub const SUBMENU_AWAY: f32 = 16.0;
    /// Height of the speed search line at the top of a popup list that is being filtered
    /// (kit: one row; IntelliJ draws a small search field above the list).
    pub const SEARCH_HEIGHT: f32 = 24.0;
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
    /// Space between the island's left edge and the first tab's pill, at rest. It scrolls
    /// away with the tabs when they overflow.
    pub const STRIP_INSET: f32 = 6.0;
    /// Space between the end of the tabs' scrolling area and the first button after it
    /// ("Show hidden tabs" or "More actions").
    pub const STRIP_BUTTONS_GAP: f32 = 9.0;
    /// Space between the strip's buttons.
    pub const BUTTON_GAP: f32 = 4.0;
    /// Space between the strip's last button and the island's right edge.
    pub const BUTTONS_INSET: f32 = 4.0;
    /// Width of the fade where tabs are cut off at an edge of the scrolling area.
    pub const EDGE_FADE: f32 = 10.0;
    /// Thickness of the strip's scroll bar thumb at its top edge.
    pub const SCROLL_THUMB: f32 = 2.0;
    /// Thickness of the thumb under the pointer or while it is dragged.
    pub const SCROLL_THUMB_HOVER: f32 = 4.0;
    /// Space between the thumb and the strip's top edge.
    pub const SCROLL_MARGIN: f32 = 1.0;
    /// Space between the thumb's track and the island's sides (it stays inside the
    /// island's rounded corners).
    pub const SCROLL_INSET: f32 = 5.0;
    /// The widest a tab gets (`DEFAULT_MAX_TAB_WIDTH`, IntelliJ spec section 2.1); longer titles
    /// are elided.
    pub const MAX_WIDTH: f32 = 300.0;
    /// How far the pointer must move from the press before a tab starts to drag (7 px,
    /// `MouseDragHelper.DRAG_START_DEADZONE`, IntelliJ spec section 0.2).
    pub const DRAG_DEADZONE: f32 = 7.0;
    /// Space between a tab's action buttons (spec section 2.1: close, pin, ...).
    pub const ACTION_GAP: f32 = 2.0;
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
    /// How far the pointer must move from the press before a stripe button drags (7 px,
    /// IntelliJ spec section 8.2).
    pub const DRAG_DEADZONE: f32 = 7.0;
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
    /// Edge of a title pane button (minimize, maximize, close): 40 pt, `TitlePane.Button.preferredSize`
    /// in IntelliJ spec §14 (30 pt high when compact).
    pub const TITLE_BUTTON: f32 = 40.0;
    /// Widest a main toolbar drop-down button grows; longer text is elided
    /// (`MainToolbar.Dropdown.maxWidth`, IntelliJ spec §14).
    pub const DROPDOWN_MAX_WIDTH: f32 = 350.0;
    /// Width of a split button's chevron part: the 20 pt arrow icon's room plus its margins
    /// (`MainToolbar.SplitDropdown.rightPartMargin` 3, IntelliJ spec §14).
    pub const SPLIT_ARROW_WIDTH: f32 = 26.0;
    /// Space either side of the line between a split button's two parts
    /// (`MainToolbar.SplitDropdown.separatorMargin` (5,3), IntelliJ spec §14).
    pub const SPLIT_SEPARATOR_MARGIN: f32 = 3.0;
    /// Length of a split button's separator line: its height less the margin of 5 above and
    /// below.
    pub const SPLIT_SEPARATOR_LENGTH: f32 = 20.0;
}

/// Action toolbars and action buttons, in points (IntelliJ spec §1.4, §1.5).
pub mod action_toolbar {
    /// Width of the drop-down arrow an action button with a popup group paints after its
    /// icon (IntelliJ draws a small arrow; the size is not specified, so this is the kit's).
    pub const ARROW_WIDTH: f32 = 10.0;
    /// Edge of that arrow.
    pub const ARROW_ICON: f32 = 12.0;
    /// Inset of a plain (non-decorated) toolbar's border: `empty(2)`.
    pub const INSET: f32 = 2.0;
    /// Vertical inset of a decorated toolbar (`ToolBar.verticalToolbarInsets` (5,7)).
    pub const DECORATED_INSET_Y: f32 = 5.0;
    /// Horizontal inset of a decorated toolbar.
    pub const DECORATED_INSET_X: f32 = 7.0;
    /// Space either side of a separator line.
    pub const SEPARATOR_GAP: f32 = 4.0;
    /// Length of a separator line in a toolbar.
    pub const SEPARATOR_LENGTH: f32 = 20.0;
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
    /// How long the editor tab strip's scroll bar stays after the pointer leaves the strip
    /// (or after the last wheel step). It appears at once.
    pub const TAB_SCROLL_HIDE_DELAY: f32 = 0.2;
    /// The fade-out of the tab strip's scroll bar once the hide delay has passed.
    pub const TAB_SCROLL_FADE_OUT: f32 = 0.1;
    // Overlays family (IntelliJ UX spec sections 7, 10, 11 and 13.8).
    /// Pause before a dialog validates after the last input change (DialogWrapper's
    /// validation delay, IntelliJ spec section 7: 300 ms).
    pub const DIALOG_VALIDATION_DELAY: f32 = 0.3;
    /// A notification balloon fades out after this long on screen (IntelliJ spec section
    /// 10.1: 10 000 ms).
    pub const NOTIFICATION_HIDE_DELAY: f32 = 10.0;
    /// A sticky notification balloon stays this long (IntelliJ spec section 10.1: 300 000 ms).
    pub const NOTIFICATION_STICKY_HIDE_DELAY: f32 = 300.0;
    /// The fade-out of a notification balloon (IntelliJ spec section 10.1: 200 ms cycle).
    pub const NOTIFICATION_FADE_OUT: f32 = 0.2;
    /// A help tooltip shows this long after the pointer rests on its owner (`ide.tooltip.
    /// initialReshowDelay`, IntelliJ spec section 11.1: 500 ms).
    pub const HELP_TOOLTIP_SHOW_DELAY: f32 = 0.5;
    /// A help tooltip hides this long after the pointer leaves (`ide.tooltip.initialDelay.
    /// highlighter`: 150 ms).
    pub const HELP_TOOLTIP_HIDE_DELAY: f32 = 0.15;
    /// A one-line help tooltip hides by itself after this long (`ide.helptooltip.regular.
    /// dismissDelay`: 10 000 ms).
    pub const HELP_TOOLTIP_DISMISS_SINGLE_LINE: f32 = 10.0;
    /// A help tooltip with a description or link hides by itself after this long
    /// (`ide.helptooltip.full.dismissDelay`: 30 000 ms).
    pub const HELP_TOOLTIP_DISMISS_MULTI_LINE: f32 = 30.0;
    /// A got-it tooltip with a timeout closes after this long (IntelliJ spec section 11.2:
    /// 5000 ms).
    pub const GOT_IT_TIMEOUT: f32 = 5.0;
    /// One frame of the async process icon (`AnimatedIcon.Default`: 125 ms).
    pub const ASYNC_PROCESS_FRAME: f32 = 0.125;
    /// Frames in one turn of the async process icon.
    pub const ASYNC_PROCESS_FRAMES: u32 = 8;
    /// One pass of an indeterminate progress bar's segment. Kept from the measurement of
    /// Android Studio (IntelliJ spec section 13.8 gives no period).
    pub const PROGRESS_INDETERMINATE_PERIOD: f32 = 1.0 / 0.6;

    /// How long a tab's freshly shown close button ignores clicks (`TimedDeadzone.DEFAULT`
    /// 150 ms, IntelliJ spec section 2.1), so a pointer landing on the tab does not close it.
    pub const TAB_CLOSE_DEADZONE: f32 = 0.15;

    /// How long a spinner's arrow must be held before it repeats (Swing's spinner timer; IntelliJ
    /// spec section 13.4 says only that a press repeats, so the delay is marked S).
    pub const SPINNER_REPEAT_DELAY: f32 = 0.3;
    /// Time between the repeated steps of a held spinner arrow (S, like the delay).
    pub const SPINNER_REPEAT_INTERVAL: f32 = 0.06;

    /// How long a combo box remembers the letters typed on it, to select by a longer prefix
    /// (Swing's key selection manager, `timeFactor` 1000 ms; IntelliJ spec section 4, marked S).
    pub const COMBO_TYPE_AHEAD_RESET: f32 = 1.0;

    /// How long the pointer rests on a popup row that opens a submenu before it opens
    /// (IntelliJ spec §5.2, a 250 ms timer in `ListPopupImpl`).
    pub const POPUP_SUBMENU_DELAY: f32 = 0.25;
    /// How long a popup with auto-selection waits, idle, before it chooses the selected row
    /// (`ide.popup.auto.delay`, IntelliJ spec §0.3).
    pub const POPUP_AUTO_SELECTION_DELAY: f32 = 0.5;
    /// How long after a popup closed it refuses to open again, so the click that closed it
    /// by pressing its owner button does not reopen it on release
    /// (`ide.popup.hide.show.threshold`, IntelliJ spec §0.3).
    pub const POPUP_REOPEN_GUARD: f32 = 0.2;

    /// The editor caret blink period: shown for this long, then hidden for this long, counted
    /// from the last caret activity (IntelliJ editor spec §0, §1.9).
    pub const CARET_BLINK: f32 = 0.5;
    /// The longest animated jump of the editor viewport to the caret (spec §4.1).
    pub const EDITOR_SCROLL_JUMP: f32 = 0.1;
    /// How long the pointer rests on text before the editor reports a hover (spec §8.1,
    /// `TOOLTIPS_DELAY_MS`).
    pub const EDITOR_TOOLTIP_DELAY: f32 = 0.5;
    /// How long the pointer rests on a fold placeholder before the preview is reported
    /// (spec §6, `FoldingPopupManager.TOOLTIP_DELAY_MS`).
    pub const FOLD_PREVIEW_DELAY: f32 = 0.3;
    /// Fade-in of the gutter's fold chevrons when the pointer enters the gutter (spec §5).
    pub const FOLD_CHEVRON_FADE_IN: f32 = 0.13;
    /// Delay before the fold chevrons start to fade out after the pointer left the gutter.
    pub const FOLD_CHEVRON_FADE_OUT_DELAY: f32 = 0.14;
    /// Fade-out of the fold chevrons.
    pub const FOLD_CHEVRON_FADE_OUT: f32 = 0.15;
    /// Tick of the drag autoscroll timer (spec §2.3, `editor.scrolling.animation.interval.ms`).
    pub const EDITOR_AUTOSCROLL_TICK: f32 = 0.1;
    /// Two presses closer than this in time and space count as a double (triple) click.
    pub const MULTI_CLICK_INTERVAL: f32 = 0.5;
}

/// Opacities, from 0 to 1.
pub mod opacity {
    /// Opacity of a row's [`tint`](crate::ui::ColoredListCellRenderer::tint) over the island.
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
    /// Opacity of a row left in place while it is dragged (a
    /// [`DragSource`](crate::behaviour::DragSource) on a [`ColoredListCellRenderer`](crate::ui::ColoredListCellRenderer)).
    pub const DRAG_SOURCE: f32 = 0.4;
    /// Opacity of the card with a dragged row's icon and label that follows the pointer.
    pub const DRAG_IMAGE: f32 = 0.85;
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
    /// Focus ring of a status bar widget (arc 10, IntelliJ spec section 9).
    pub const STATUS_FOCUS: u8 = 10;
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

/// Scroll bar fade, from the IntelliJ spec section 13.1: after the pointer enters the bar's
/// strip the thumb waits 150 ms, then brightens over 125 ms; after it leaves it waits 300 ms,
/// then dims over 125 ms. The bar's width and colors stay the measured ones of [`scroll`].
pub mod scroll_bar {
    /// Seconds the pointer must be over the bar before the thumb starts to brighten.
    pub const ENTER_DELAY: f32 = 0.150;
    /// Seconds after the pointer left the bar before the thumb starts to dim.
    pub const LEAVE_DELAY: f32 = 0.300;
    /// Seconds the thumb takes to brighten or dim.
    pub const FADE: f32 = 0.125;
    /// Shortest thumb, in points, so a very long list still has something to grab (the
    /// measured minimum of the kit's scroll bars; the spec has no value).
    pub const MIN_THUMB: f32 = 12.0;
}

/// The speed search popup (IntelliJ spec section 3.4).
pub mod speed_search {
    /// Height of the popup field (kit choice; the spec gives no value).
    pub const HEIGHT: f32 = super::size::CONTROL_HEIGHT_SMALL;
    /// Space between the popup's left edge and its search icon.
    pub const ICON_INSET: f32 = 8.0;
    /// Gap between the search icon and the text (spec: 10).
    pub const ICON_GAP: f32 = 10.0;
    /// Space after the text.
    pub const PADDING_END: f32 = 8.0;
    /// Corner radius of the background behind a matched fragment of text.
    pub const MATCH_RADIUS: u8 = 2;
    /// The text field is at least as wide as this placeholder hint.
    pub const PLACEHOLDER: &str = "Search";
}

/// Loading panels ([`JBLoadingPanel`](crate::ui::JBLoadingPanel)), IntelliJ spec section 13.7.
pub mod loading {
    /// Seconds the veil takes to fade out when loading ends (kit choice; the spec only
    /// says "fade-out animator").
    pub const FADE_OUT: f32 = 0.2;
    /// Gap between the spinner and the text beneath it.
    pub const TEXT_GAP: f32 = 8.0;
}

/// The empty text of collections (IntelliJ `StatusText`).
pub mod empty_text {
    /// Shown by an empty list, tree or table that was given no text of its own
    /// (`StatusText.getDefaultEmptyText`).
    pub const DEFAULT: &str = "Nothing to show";
    /// Space between two lines of a [`StatusText`](crate::ui::StatusText) block.
    pub const LINE_GAP: f32 = 2.0;
}

/// Editor metrics (the editor component, IntelliJ editor spec §0, §1.9, §4.1, §5, §6).
pub mod editor {
    /// Width of the insert-mode caret bar (`editor.caret.width`).
    pub const CARET_WIDTH: f32 = 2.0;
    /// Corner radius of the caret bar in the Islands theme (`min(width / 2, 3)`).
    pub const CARET_RADIUS: u8 = 1;
    /// The selection's corner radius is the line height divided by this (Islands selection
    /// painter, spec §8.1).
    pub const SELECTION_RADIUS_DIVISOR: f32 = 6.0;
    /// Corner radius of the highlights behind text (search results, occurrences, braces).
    pub const HIGHLIGHT_RADIUS: u8 = 2;
    /// Points of the arc of one rounded selection corner.
    pub const SELECTION_ARC_STEPS: usize = 5;
    /// Space before the gutter's line numbers (`emptyAnnotationAreaWidth`).
    pub const GUTTER_ANNOTATION: f32 = 4.0;
    /// Space before and after the line numbers (`gapAfterVcsMarkersWidth`,
    /// `gapAfterLineNumbersWidth`).
    pub const GUTTER_GAP: f32 = 4.0;
    /// Width of the strip of version control change markers (`FREE_PAINTERS_LEFT_AREA_WIDTH`).
    pub const CHANGE_MARKER_AREA: f32 = 8.0;
    /// Width of the colored change marker inside that strip.
    pub const CHANGE_MARKER: f32 = 3.0;
    /// Width of one lane of gutter icons (`START_ICON_AREA_WIDTH`).
    pub const GUTTER_LANE: f32 = 17.0;
    /// Edge of a gutter icon.
    pub const GUTTER_ICON: f32 = 16.0;
    /// Space after the icon lanes (`gapAfterIconsWidth`).
    pub const GUTTER_AFTER_ICONS: f32 = 4.0;
    /// Edge of the fold chevron icon.
    pub const FOLD_ICON: f32 = 9.0;
    /// Width of the fold column: the chevron and its click target (the kit's measured 16).
    pub const FOLD_COLUMN: f32 = 16.0;
    /// Thickness of the ring inside a fold placeholder (`FOLDED_TEXT_BORDER_COLOR`).
    pub const FOLD_BORDER: f32 = 2.0;
    /// The line numbers' font is the editor font reduced by this
    /// (`editor.gutter.linenumber.font.size.delta`).
    pub const NUMBER_FONT_DELTA: f32 = -1.0;
    /// Width of the error stripe column next to the scroll bar (`SCROLLBAR_WIDTH`).
    pub const STRIPE_WIDTH: f32 = 14.0;
    /// Least height of one stripe mark (`errorStripeMarkMinHeight`).
    pub const STRIPE_MARK_MIN: f32 = 2.0;
    /// Pixels a press in the gutter must move before it starts a line selection
    /// (`editor.drag.sensitivity`).
    pub const DRAG_SENSITIVITY: f32 = 5.0;
    /// Autoscroll: the distance beyond the viewport edge in pixels that adds one line per step
    /// (spec §2.3: a step every `20 / d + 1` ticks, `1 + d / 20` lines each).
    pub const AUTOSCROLL_DISTANCE: f32 = 20.0;
    /// Diameter of the dot that shows a space when whitespace is shown.
    pub const WHITESPACE_DOT: f32 = 1.0;
    /// Height of the wave of an error squiggle.
    pub const SQUIGGLE_HEIGHT: f32 = 2.0;
    /// Half the period of the wave of an error squiggle.
    pub const SQUIGGLE_HALF_PERIOD: f32 = 2.0;
    /// Selection occurrences are highlighted only while there are at most this many
    /// (`editor.highlight.selected.text.max.occurrences.threshold`).
    pub const MAX_SELECTION_OCCURRENCES: usize = 50;
    /// Height of one row of the find bar: the field (`CONTROL_HEIGHT`) and 4 above and below
    /// (spec §7: the field inset of the new UI is 7,10,7,8; the kit's field already carries
    /// its own focus inset, so the row adds 4).
    pub const FIND_ROW_HEIGHT: f32 = 36.0;
    /// Space left of the find bar's first control (spec §7 field inset, left 10).
    pub const FIND_INSET_LEFT: f32 = 10.0;
    /// Space right of the find bar's last control (spec §7 field inset, right 8).
    pub const FIND_INSET_RIGHT: f32 = 8.0;
    /// Width reserved for the status text ("1234/10000+", "Bad pattern").
    pub const FIND_STATUS_WIDTH: f32 = 84.0;
    /// The status text shrinks to this in a narrow bar.
    pub const FIND_STATUS_MIN_WIDTH: f32 = 44.0;
    /// Width of the controls around the status text and the field in the find bar's first
    /// row: what is left of the row beyond it goes to the status text, up to its width.
    pub const FIND_NARROW_RESERVE: f32 = 330.0;
    /// The query field grows with its lines (Ctrl+Shift+Enter) up to this many.
    pub const FIND_MAX_QUERY_ROWS: usize = 4;
    /// Width of the history popup's rows when the field is narrower.
    pub const FIND_HISTORY_MIN_WIDTH: f32 = 180.0;
}
