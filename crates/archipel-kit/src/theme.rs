//! Themes: every color a component paints, for one look (dark or light).
//!
//! A [`Theme`] is plain data. [`crate::install`] stores it in the [`egui::Context`] and every
//! component reads it back with [`Theme::current`], so two contexts (two windows, two tests
//! running in parallel) can use different themes at the same time.
//!
//! The built-in palettes [`Theme::dark`] and [`Theme::light`] are derived from public,
//! Apache-2.0 theme data (see `NOTICE.md`). Applications can start from one and override
//! single fields:
//!
//! ```
//! use archipel_kit::{Theme, egui::Color32};
//!
//! let mut theme = Theme::dark();
//! theme.colors.accent = Color32::from_rgb(0xE0, 0x88, 0x55);
//! ```

use egui::Color32;
use std::sync::Arc;

mod catalog;
// Written by `tools/themes` (see `docs/theming.md`); not reformatted.
#[rustfmt::skip]
mod generated;

pub use catalog::{EditorScheme, Effect, EffectKind, TextAttributes, ThemeInfo, ThemeSource};

const fn rgb(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

fn rgba(value: u32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (value >> 24) as u8,
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    )
}

/// A complete look: workbench and editor colors.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Display name, e.g. `"Archipel Dark"`.
    pub name: String,
    /// Whether this is a dark theme (selects egui's dark base visuals and native window chrome).
    pub dark: bool,
    /// Workbench colors.
    pub colors: Colors,
    /// Code editor colors.
    pub editor: EditorColors,
    /// How language file icons look (an app setting; see `docs/icons/languages.md`).
    pub language_icons: archipel_icons::LanguageStyle,
    /// Cursor over controls that perform an action (buttons, checkboxes, toggles,
    /// dropdowns, close buttons). Default: the pointing hand. Rows, tabs and menu items
    /// keep the arrow. See `docs/cursors.md`.
    pub action_cursor: egui::CursorIcon,
    /// The bundled IntelliJ tables this theme was built from (for [`Theme::ui_color`] and
    /// [`Theme::text_attributes`]); empty for a theme built by hand.
    pub source: ThemeSource,
}

/// Background and border of a banner or callout of one severity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    /// Fill color.
    pub background: Color32,
    /// 1 px outline color.
    pub border: Color32,
}

/// Workbench colors: surfaces, text, interaction states and semantic accents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    // Surfaces.
    /// Window background behind the islands: main toolbar, status bar, stripes.
    pub frame: Color32,
    /// Island (panel, tool window, editor) background.
    pub island: Color32,
    /// Popup, menu and dialog background.
    pub popup: Color32,
    /// Popup, menu and dialog outline.
    pub popup_border: Color32,
    /// The mnemonic or number hint at the start of a popup list row (`Popup.mnemonicForeground`,
    /// `Gray.x78` dark / `Gray.x99` light; IntelliJ spec §5.3).
    pub popup_mnemonic: Color32,
    /// Text field and other input backgrounds.
    pub field: Color32,
    /// Notification balloon background.
    pub notification: Color32,
    /// Tooltip background.
    pub tooltip: Color32,

    // Text.
    /// Default text.
    pub text: Color32,
    /// Emphasized text: titles, selected rows.
    pub text_strong: Color32,
    /// Secondary labels: status bar widgets, row details.
    pub text_secondary: Color32,
    /// Hints, placeholders and captions.
    pub text_placeholder: Color32,
    /// Disabled text.
    pub text_disabled: Color32,
    /// Text drawn on the accent color (primary button label, selected toggle).
    pub text_on_accent: Color32,

    // Interaction.
    /// Hover fill of flat controls (toolbar buttons, rows, menu items).
    pub hover: Color32,
    /// Pressed fill of flat controls.
    pub pressed: Color32,
    /// Selected row in a focused list or tree.
    pub selection: Color32,
    /// Selected row in completion-style popups.
    pub selection_popup: Color32,
    /// Accent: primary buttons, focus, selected toggles.
    pub accent: Color32,
    /// Primary button fill when disabled.
    pub accent_disabled: Color32,
    /// Label of a disabled primary button: at least 3:1 against [`Colors::accent_disabled`].
    pub text_on_accent_disabled: Color32,
    /// Keyboard focus ring around inputs and buttons.
    pub focus_ring: Color32,
    /// Outline of buttons, inputs and checkboxes.
    pub border: Color32,
    /// Toolbar and menu separators.
    pub separator: Color32,
    /// Line after a group title in settings forms ([`TitledSeparator`](crate::ui::TitledSeparator)).
    pub group_separator: Color32,
    /// Selected editor tab fill.
    pub tab_selected: Color32,
    /// Selected editor tab underline.
    pub tab_selected_border: Color32,
    /// Title of an editor tab that is not selected (the selected one uses [`Colors::text`]).
    pub tab_text: Color32,
    /// Thumb of the editor tab strip's thin scroll bar (shown while the pointer is over
    /// the strip).
    pub tab_scroll_thumb: Color32,
    /// The same thumb under the pointer or while it is dragged.
    pub tab_scroll_thumb_hover: Color32,
    /// Fill of a switched-on toggle.
    pub toggle_on: Color32,
    /// Track of a switched-off toggle.
    pub toggle_off: Color32,
    /// Outline and knob of a switched-off toggle.
    pub toggle_off_knob: Color32,
    /// Outline of unchecked checkboxes and radio buttons.
    pub control_border: Color32,
    /// Border of a disabled button, field or checkbox.
    pub border_disabled: Color32,
    /// Fill of an unchecked checkbox or radio button.
    pub control: Color32,
    /// Fill of a checked checkbox or radio button, and their focus border.
    pub control_selected: Color32,
    /// Selection of a list, tree or table that does not have the keyboard focus.
    pub selection_inactive: Color32,
    /// Hover fill of list and tree rows (subtler than [`Colors::hover`]).
    pub row_hover: Color32,
    /// Background of the part of a row text that matches the speed search
    /// (`SimpleTextAttributes::STYLE_SEARCH_MATCH`; IntelliJ spec §3.4 `SearchMatch` Yellow7).
    pub search_match: Color32,
    /// Fill of the speed search popup (IntelliJ spec §3.4, §15.3 `SpeedSearch.background`).
    pub speed_search_background: Color32,
    /// 1 pt border of the speed search popup (IntelliJ spec §3.4 `SpeedSearch.borderColor`).
    pub speed_search_border: Color32,
    /// Veil over content that is loading ([`JBLoadingPanel`](crate::ui::JBLoadingPanel);
    /// IntelliJ spec §13.7: rgba(200,200,200,240) light).
    pub loading_veil: Color32,
    /// Fill of non-editable dropdowns.
    pub combo: Color32,
    /// Dropdown arrow.
    pub combo_arrow: Color32,
    /// The cross of a field's clear button.
    pub clear_icon: Color32,
    /// The cross of a field's clear button under the pointer.
    pub clear_icon_hover: Color32,
    /// The round background of a field's clear button under the pointer.
    pub clear_hover: Color32,
    /// Fill of the selected segment of a segmented control.
    pub segment_selected: Color32,
    /// Outline of the selected segment.
    pub segment_selected_border: Color32,
    /// Track of progress bars.
    pub progress_track: Color32,
    /// Counter badge fill.
    pub counter: Color32,
    /// Counter badge text.
    pub counter_text: Color32,
    /// Tag (chip) fill.
    pub tag: Color32,

    // Semantic.
    /// Errors: messages, invalid input outlines, error icons.
    pub error: Color32,
    /// Warnings.
    pub warning: Color32,
    /// Success.
    pub success: Color32,
    /// Hyperlinks.
    pub link: Color32,
    /// Info banner tint.
    pub banner_info: Tint,
    /// Warning banner tint.
    pub banner_warning: Tint,
    /// Error banner tint.
    pub banner_error: Tint,
    /// Success banner tint.
    pub banner_success: Tint,

    // Domain accents shared by developer tools.
    /// Run actions (play icon, run widget).
    pub run: Color32,
    /// Stop actions.
    pub stop: Color32,
    /// Modified files (VCS).
    pub vcs_modified: Color32,
    /// Added files (VCS).
    pub vcs_added: Color32,
    /// Untracked files (VCS).
    pub vcs_untracked: Color32,
    /// Ignored files and other muted text.
    pub vcs_ignored: Color32,

    // Icons.
    /// Default icon stroke.
    pub icon: Color32,
    /// Folder icon fill.
    pub folder_fill: Color32,

    // Data visualization.
    /// Eight distinguishable series colors for charts, each at least 3:1 against `island`.
    /// Use [`Colors::chart`] to index them.
    pub categorical: [Color32; 8],
    /// Sequential ramp from "little" to "much" (heatmaps, histograms). Luminance moves away
    /// from `island` as the value grows. Use [`Colors::ramp`] to sample it.
    pub sequential: [Color32; 5],

    // Got-it tooltips (IntelliJ spec section 11.2, Islands `got-it-bg` and `got-it-border`).
    /// Fill of a got-it tooltip: white on light, `#2E4D89` on dark.
    pub got_it: Color32,
    /// Outline of a got-it tooltip (`feedback-border`).
    pub got_it_border: Color32,
}

/// Code editor colors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorColors {
    /// Editor background.
    pub background: Color32,
    /// Default code text.
    pub text: Color32,
    /// Caret.
    pub caret: Color32,
    /// Line containing the caret.
    pub caret_row: Color32,
    /// Gutter line numbers.
    pub line_number: Color32,
    /// Line number of the caret line.
    pub line_number_caret: Color32,
    /// Text selection.
    pub selection: Color32,
    /// Find bar and other panels inside the editor.
    pub background_panel: Color32,
    /// Search match highlight.
    pub search_result: Color32,
    /// Occurrences during rename.
    pub rename_occurrence: Color32,
    /// Matching brace highlight.
    pub matched_brace: Color32,
    /// Indent guides.
    pub indent_guide: Color32,
    /// Error squiggle.
    pub error_effect: Color32,
    /// Warning squiggle.
    pub warning_effect: Color32,
    /// Weak warning squiggle.
    pub weak_warning_effect: Color32,
    /// Breakpoint line tint.
    pub breakpoint_line: Color32,
    /// Suspended execution line tint.
    pub execution_line: Color32,
    /// Inline debugger values.
    pub inline_value: Color32,
    /// Error marks in the scrollbar stripe.
    pub error_stripe: Color32,
    /// Warning marks in the scrollbar stripe.
    pub warning_stripe: Color32,

    // Syntax: the colors of code and of typed values in data views (see `ValueKind`).
    /// String literals.
    pub string: Color32,
    /// Numbers.
    pub number: Color32,
    /// Keywords (also booleans in data views).
    pub keyword: Color32,
    /// Constants (also dates in data views).
    pub constant: Color32,
    /// Type names (also ids in data views).
    pub type_name: Color32,
    /// Annotations and metadata (also binary values in data views).
    pub annotation: Color32,
    /// Properties and keys.
    pub property: Color32,
    /// Punctuation: braces, brackets, commas, colons.
    pub punctuation: Color32,
    /// Comments (also null in data views).
    pub comment: Color32,
    /// Markup tags.
    pub tag: Color32,
    /// Markup attributes.
    pub attribute: Color32,
    /// Variables and identifiers with meaning.
    pub variable: Color32,
    /// Invalid or unterminated tokens.
    pub invalid: Color32,
    /// The current find match, stronger than `search_result`.
    pub search_result_current: Color32,

    // Editor component (IntelliJ editor spec §4.4, §5, §6, §8.1).
    /// Text selection while the editor does not have the focus (`SELECTION_BACKGROUND_INACTIVE`).
    pub selection_inactive: Color32,
    /// Text of a collapsed fold region (`FOLDED_TEXT_ATTRIBUTES`).
    pub folded_text: Color32,
    /// Background of a collapsed fold region.
    pub folded_background: Color32,
    /// The 2 px ring inside a collapsed fold region (`FOLDED_TEXT_BORDER_COLOR`).
    pub folded_border: Color32,
    /// The right margin guide (`RIGHT_MARGIN_COLOR`).
    pub right_margin: Color32,
    /// The indent guide of the caret's block (`SELECTED_INDENT_GUIDE`).
    pub indent_guide_selected: Color32,
    /// Dots and lines that show whitespace (`WHITESPACES`).
    pub whitespace: Color32,
    /// Fold chevrons in the gutter.
    pub fold_marker: Color32,
    /// Text-search marks in the error stripe (`ERROR_STRIPE_COLOR` of text search results).
    pub search_stripe: Color32,
    /// Gutter change marker: added lines.
    pub change_added: Color32,
    /// Gutter change marker: modified lines.
    pub change_modified: Color32,
    /// Gutter change marker: deleted lines.
    pub change_deleted: Color32,
}

impl Colors {
    /// Series color `index` of the categorical chart palette; wraps after 8 series.
    pub fn chart(&self, index: usize) -> Color32 {
        self.categorical[index % self.categorical.len()]
    }

    /// Samples the sequential ramp at `t` (clamped to 0..=1), blending between its stops.
    pub fn ramp(&self, t: f32) -> Color32 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        let scaled = t * (self.sequential.len() - 1) as f32;
        let low = (scaled.floor() as usize).min(self.sequential.len() - 2);
        self.sequential[low].lerp_to_gamma(self.sequential[low + 1], scaled - low as f32)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Archipel Dark: dark islands on a slightly lighter frame.
    pub fn dark() -> Self {
        Self {
            name: "Archipel Dark".into(),
            dark: true,
            colors: Colors {
                frame: rgb(0x26282C),
                island: rgb(0x191A1C),
                popup: rgb(0x26282C),
                popup_border: rgb(0x33353B),
                // IntelliJ spec §5.3: `Popup.mnemonicForeground` Gray.x78 in dark themes.
                popup_mnemonic: rgb(0x787878),
                field: rgb(0x191A1C),
                notification: rgb(0x33353B),
                tooltip: rgb(0x33353B),
                text: rgb(0xD1D3D9),
                text_strong: rgb(0xDFE1E5),
                text_secondary: rgb(0x9FA2A8),
                text_placeholder: rgb(0x73767C),
                text_disabled: rgb(0x4C4F56),
                text_on_accent: rgb(0xFFFFFF),
                hover: rgba(0xFFFFFF17),
                pressed: rgba(0xFFFFFF29),
                selection: rgb(0x2A4371),
                selection_popup: rgb(0x113A5C),
                accent: rgb(0x3871E1),
                accent_disabled: rgb(0x4E5157),
                text_on_accent_disabled: rgb(0xB0B3BA),
                focus_ring: rgb(0x3871E1),
                border: rgb(0x40434A),
                separator: rgba(0xFFFFFF21),
                group_separator: rgb(0x33353B),
                tab_selected: rgb(0x233558),
                tab_selected_border: rgb(0x2E4D89),
                tab_text: rgb(0xAFB1B6),
                tab_scroll_thumb: rgb(0x434344),
                tab_scroll_thumb_hover: rgb(0x535455),
                toggle_on: rgb(0x338555),
                toggle_off: rgb(0x191A1C),
                toggle_off_knob: rgb(0x5F6269),
                control_border: rgb(0x5A5D63),
                control: rgb(0x2B2D30),
                border_disabled: rgb(0x33353B),
                control_selected: rgb(0x3574F0),
                selection_inactive: rgb(0x33353B),
                row_hover: rgba(0xFFFFFF10),
                // Spec §3.4 lists only the light Yellow7 (#FEE6B1); dark uses the Islands
                // warning border (#694820, spec §15.2) so the dark text stays readable.
                search_match: rgb(0x694820),
                // Spec §15.3 has the light values; dark takes the tool window background and
                // the popup border of §15.2.
                speed_search_background: rgb(0x191A1C),
                speed_search_border: rgb(0x33353B),
                // Spec §13.7 gives the light veil only; dark is the island color at 240/255.
                loading_veil: rgba(0x191A1CF0),
                combo: rgb(0x26282C),
                combo_arrow: rgb(0x9A9DA1),
                clear_icon: rgb(0x676B71),
                clear_icon_hover: rgb(0x7F8389),
                clear_hover: rgb(0x353637),
                segment_selected: rgb(0x26282C),
                segment_selected_border: rgb(0x5F6269),
                progress_track: rgb(0x40434A),
                counter: rgb(0x73767C),
                counter_text: rgb(0x000000),
                tag: rgb(0x26282C),
                error: rgb(0xF57E84),
                warning: rgb(0xD59637),
                success: rgb(0x6DB083),
                link: rgb(0x71A1FE),
                banner_info: Tint {
                    background: rgb(0x233558),
                    border: rgb(0x2E4D89),
                },
                banner_warning: Tint {
                    background: rgb(0x44321D),
                    border: rgb(0x694820),
                },
                banner_error: Tint {
                    background: rgb(0x56272B),
                    border: rgb(0x80383E),
                },
                banner_success: Tint {
                    background: rgb(0x203B2A),
                    border: rgb(0x29583C),
                },
                run: rgb(0x4E9D6C),
                stop: rgb(0xC54E58),
                vcs_modified: rgb(0x71A1FE),
                vcs_added: rgb(0x73BD79),
                vcs_untracked: rgb(0xE37774),
                vcs_ignored: rgb(0x6F737A),
                icon: rgb(0xCED0D6),
                folder_fill: rgb(0x43454A),
                categorical: [
                    rgb(0x5B9CF6),
                    rgb(0xE8935A),
                    rgb(0x5FBF7F),
                    rgb(0xC77DDA),
                    rgb(0xF0D34E),
                    rgb(0x3FC1C9),
                    rgb(0xEF6F8B),
                    rgb(0xA3C44F),
                ],
                sequential: [
                    rgb(0x1E3558),
                    rgb(0x28569A),
                    rgb(0x3A7FD5),
                    rgb(0x78B0EE),
                    rgb(0xBFDDFF),
                ],
                // IntelliJ spec section 15.2: got-it-bg #2E4D89, feedback-border #33353B.
                got_it: rgb(0x2E4D89),
                got_it_border: rgb(0x33353B),
            },
            language_icons: archipel_icons::LanguageStyle::FileBand,
            action_cursor: egui::CursorIcon::PointingHand,
            source: ThemeSource::new("islands_dark", "islands_dark"),
            editor: EditorColors {
                background: rgb(0x191A1C),
                text: rgb(0xBCBEC4),
                caret: rgb(0xCED0D6),
                caret_row: rgb(0x1F2024),
                line_number: rgb(0x4B5059),
                line_number_caret: rgb(0xA1A3AB),
                selection: rgb(0x214283),
                background_panel: rgb(0x212326),
                search_result: rgb(0x114957),
                rename_occurrence: rgb(0x373B39),
                matched_brace: rgb(0x43454A),
                indent_guide: rgb(0x323438),
                error_effect: rgb(0xFA6675),
                warning_effect: rgb(0xF2C55C),
                weak_warning_effect: rgb(0x857042),
                breakpoint_line: rgb(0x40252B),
                execution_line: rgb(0x2A5091),
                inline_value: rgb(0x666D75),
                error_stripe: rgb(0xD64D5B),
                warning_stripe: rgb(0xC29E4A),
                string: rgb(0x6AAB73),
                number: rgb(0x2AACB8),
                keyword: rgb(0xCF8E6D),
                constant: rgb(0x9E86C8),
                type_name: rgb(0x16BAAC),
                annotation: rgb(0xB3AE60),
                property: rgb(0xC77DBB),
                punctuation: rgb(0x9FA2A8),
                comment: rgb(0x7A7E85),
                tag: rgb(0xE8BA36),
                attribute: rgb(0xB3AE60),
                variable: rgb(0x56A8F5),
                invalid: rgb(0xFA6675),
                search_result_current: rgb(0x1F7C94),
                selection_inactive: rgb(0x4C4F56),
                folded_text: rgb(0x868991),
                folded_background: rgb(0x393B40),
                folded_border: rgb(0x2B2D30),
                right_margin: rgb(0x323438),
                indent_guide_selected: rgb(0x4E5157),
                whitespace: rgb(0x6F737A),
                fold_marker: rgb(0x6F737A),
                search_stripe: rgb(0x72D6D6),
                change_added: rgb(0x549159),
                change_modified: rgb(0x375FAD),
                change_deleted: rgb(0x868A91),
            },
        }
    }

    /// Archipel Light: white islands on a light gray frame.
    pub fn light() -> Self {
        Self {
            name: "Archipel Light".into(),
            dark: false,
            colors: Colors {
                frame: rgb(0xE9EAEE),
                island: rgb(0xFFFFFF),
                popup: rgb(0xFFFFFF),
                popup_border: rgb(0xD1D3D9),
                // IntelliJ spec §5.3: `Popup.mnemonicForeground` Gray.x99 in light themes.
                popup_mnemonic: rgb(0x999999),
                field: rgb(0xFFFFFF),
                notification: rgb(0xFFFFFF),
                tooltip: rgb(0xFFFFFF),
                text: rgb(0x000000),
                text_strong: rgb(0x000000),
                text_secondary: rgb(0x5F6269),
                text_placeholder: rgb(0x73767C),
                text_disabled: rgb(0x9FA2A8),
                text_on_accent: rgb(0xFFFFFF),
                hover: rgba(0x00000012),
                pressed: rgba(0x00000020),
                selection: rgb(0xD0DFFE),
                selection_popup: rgb(0xC5DFFC),
                accent: rgb(0x3871E1),
                accent_disabled: rgb(0xDDDFE4),
                text_on_accent_disabled: rgb(0x60636A),
                focus_ring: rgb(0x3871E1),
                border: rgb(0xD1D3D9),
                separator: rgba(0x00000020),
                group_separator: rgb(0xE9EAEE),
                tab_selected: rgb(0xE3EBFE),
                tab_selected_border: rgb(0xA7C5FF),
                tab_text: rgb(0x515151),
                tab_scroll_thumb: rgb(0xABABAB),
                tab_scroll_thumb_hover: rgb(0x7F7F7F),
                toggle_on: rgb(0x4E9D6C),
                toggle_off: rgb(0xF7F8F9),
                toggle_off_knob: rgb(0xC3C5CB),
                control_border: rgb(0xA8ADBD),
                control: rgb(0xFFFFFF),
                border_disabled: rgb(0xDDDFE4),
                control_selected: rgb(0x3574F0),
                selection_inactive: rgb(0xE9EAEE),
                row_hover: rgba(0x00000008),
                search_match: rgb(0xFEE6B1),
                speed_search_background: rgb(0xFFFFFF),
                speed_search_border: rgb(0xEBECF0),
                loading_veil: rgba(0xC8C8C8F0),
                combo: rgb(0xFFFFFF),
                combo_arrow: rgb(0x6E6E6E),
                clear_icon: rgb(0xB0B4C3),
                clear_icon_hover: rgb(0x8A8E9C),
                clear_hover: rgb(0xEBEBED),
                segment_selected: rgb(0xFFFFFF),
                segment_selected_border: rgb(0xB5B7BD),
                progress_track: rgb(0xDDDFE4),
                counter: rgb(0x8B8E94),
                counter_text: rgb(0xFFFFFF),
                tag: rgb(0xEBECF0),
                error: rgb(0xC54E58),
                warning: rgb(0xA56906),
                success: rgb(0x338555),
                link: rgb(0x2F5EB9),
                banner_info: Tint {
                    background: rgb(0xF7F8FF),
                    border: rgb(0xBDD3FF),
                },
                banner_warning: Tint {
                    background: rgb(0xFFF6E9),
                    border: rgb(0xF4CD9A),
                },
                banner_error: Tint {
                    background: rgb(0xFFF6F5),
                    border: rgb(0xFFC4C5),
                },
                banner_success: Tint {
                    background: rgb(0xF5FAF3),
                    border: rgb(0xBBDBC2),
                },
                run: rgb(0x369650),
                stop: rgb(0xE4656E),
                vcs_modified: rgb(0x2F5EB9),
                vcs_added: rgb(0x067D17),
                vcs_untracked: rgb(0xB23247),
                vcs_ignored: rgb(0x818594),
                icon: rgb(0x73767C),
                folder_fill: rgb(0xEBECF0),
                categorical: [
                    rgb(0x2F6FDE),
                    rgb(0xD9661F),
                    rgb(0x1E9A4A),
                    rgb(0x9A4FC0),
                    rgb(0x9C8200),
                    rgb(0x0E8F9B),
                    rgb(0xD6457A),
                    rgb(0x5C8A0A),
                ],
                sequential: [
                    rgb(0xDCE9FB),
                    rgb(0xA3C6F2),
                    rgb(0x5C97E6),
                    rgb(0x2A63C2),
                    rgb(0x0F3A82),
                ],
                // IntelliJ spec section 15.2: got-it-bg #FFFFFF, feedback-border #D1D3D9.
                got_it: rgb(0xFFFFFF),
                got_it_border: rgb(0xD1D3D9),
            },
            language_icons: archipel_icons::LanguageStyle::FileBand,
            action_cursor: egui::CursorIcon::PointingHand,
            source: ThemeSource::new("islands_light", "new_ui_light"),
            editor: EditorColors {
                background: rgb(0xFFFFFF),
                text: rgb(0x080808),
                caret: rgb(0x000000),
                caret_row: rgb(0xF5F8FE),
                line_number: rgb(0xAEB3C2),
                line_number_caret: rgb(0x767A8A),
                selection: rgb(0xA6D2FF),
                background_panel: rgb(0xF7F8F9),
                search_result: rgb(0xFCD47E),
                rename_occurrence: rgb(0xEDEBFC),
                matched_brace: rgb(0x93D9D9),
                indent_guide: rgb(0xEBECF0),
                error_effect: rgb(0xFF0000),
                warning_effect: rgb(0xF2BF57),
                weak_warning_effect: rgb(0xCCCCCC),
                breakpoint_line: rgb(0xFAEAE6),
                execution_line: rgb(0x2154A6),
                inline_value: rgb(0x868686),
                error_stripe: rgb(0xCF5B56),
                warning_stripe: rgb(0xF2BF57),
                string: rgb(0x067D17),
                number: rgb(0x1750EB),
                keyword: rgb(0x0033B3),
                constant: rgb(0x7A3E9D),
                type_name: rgb(0x00627A),
                annotation: rgb(0x8A6D00),
                property: rgb(0x871094),
                punctuation: rgb(0x5C5F6B),
                comment: rgb(0x7C7F8A),
                tag: rgb(0x0033B3),
                attribute: rgb(0x174AD4),
                variable: rgb(0x00627A),
                invalid: rgb(0xF50000),
                search_result_current: rgb(0xF2A33A),
                selection_inactive: rgb(0xB5B7BD),
                folded_text: rgb(0x414D41),
                folded_background: rgb(0xE9F5E6),
                folded_border: rgb(0xADD9C7),
                right_margin: rgb(0xEBECF0),
                indent_guide_selected: rgb(0xAEB3C2),
                whitespace: rgb(0xADADAD),
                fold_marker: rgb(0xA8ADBD),
                search_stripe: rgb(0xC47233),
                change_added: rgb(0x7FC784),
                change_modified: rgb(0x88ADF7),
                change_deleted: rgb(0x767A8A),
            },
        }
    }

    /// Series color `index` of the chart palette (wraps after 8 series).
    pub fn chart(&self, index: usize) -> Color32 {
        self.colors.chart(index)
    }

    /// The theme installed in `ctx`, or [`Theme::dark`] if [`crate::install`] was never called.
    pub fn current(ctx: &egui::Context) -> Arc<Theme> {
        ctx.data(|data| data.get_temp::<Arc<Theme>>(theme_id()))
            .unwrap_or_else(|| {
                static FALLBACK: std::sync::LazyLock<Arc<Theme>> =
                    std::sync::LazyLock::new(|| Arc::new(Theme::dark()));
                FALLBACK.clone()
            })
    }

    /// Workbench colors of the theme installed in `ctx`.
    pub fn colors(ctx: &egui::Context) -> Colors {
        Self::current(ctx).colors
    }

    /// Makes this the active theme of `ctx` and rebuilds egui's visuals from it. Fonts and
    /// spacing are untouched; call [`crate::install`] once at startup for those.
    pub fn apply(self, ctx: &egui::Context) {
        let theme = Arc::new(self);
        crate::style::apply_visuals(ctx, &theme);
        ctx.data_mut(|data| data.insert_temp(theme_id(), theme));
    }
}

fn theme_id() -> egui::Id {
    egui::Id::new("archipel_kit::theme")
}

/// Stable avatar color for a name (project and user badges).
pub fn avatar_color(name: &str) -> Color32 {
    const AVATAR_COLORS: [Color32; 9] = [
        rgb(0xE08855),
        rgb(0xB08B14),
        rgb(0xA1A359),
        rgb(0x3B92B8),
        rgb(0x3574F0),
        rgb(0xC84D8F),
        rgb(0x955AE0),
        rgb(0x24A394),
        rgb(0x5FAD65),
    ];
    let hash = name.bytes().fold(0u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u32)
    });
    AVATAR_COLORS[hash as usize % AVATAR_COLORS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_keeps_alpha() {
        assert_eq!(rgba(0xFFFFFF17).a(), 0x17);
        assert_eq!(rgb(0x26282C), Color32::from_rgb(0x26, 0x28, 0x2C));
    }

    #[test]
    fn current_falls_back_to_dark_and_follows_apply() {
        let ctx = egui::Context::default();
        assert!(Theme::current(&ctx).dark);
        Theme::light().apply(&ctx);
        assert!(!Theme::current(&ctx).dark);
        assert_eq!(Theme::colors(&ctx).island, Theme::light().colors.island);
    }

    #[test]
    fn themes_are_independent_per_context() {
        let (a, b) = (egui::Context::default(), egui::Context::default());
        Theme::light().apply(&a);
        Theme::dark().apply(&b);
        assert_ne!(Theme::colors(&a).island, Theme::colors(&b).island);
    }

    #[test]
    fn text_contrasts_with_islands() {
        for theme in [Theme::dark(), Theme::light()] {
            let ratio = contrast(theme.colors.text, theme.colors.island);
            assert!(ratio >= 7.0, "{}: text contrast {ratio:.1}", theme.name);
            let secondary = contrast(theme.colors.text_secondary, theme.colors.island);
            assert!(secondary >= 4.5, "{}: secondary {secondary:.1}", theme.name);
            let on_accent = contrast(theme.colors.text_on_accent, theme.colors.accent);
            assert!(on_accent >= 4.5, "{}: on accent {on_accent:.1}", theme.name);
            let disabled = contrast(
                theme.colors.text_on_accent_disabled,
                theme.colors.accent_disabled,
            );
            assert!(
                disabled >= 3.0,
                "{}: disabled primary {disabled:.1}",
                theme.name
            );
        }
    }

    #[test]
    fn chart_palette_is_readable_and_distinct() {
        for theme in [Theme::dark(), Theme::light()] {
            let island = theme.colors.island;
            for (i, color) in theme.colors.categorical.iter().enumerate() {
                let ratio = contrast(*color, island);
                assert!(
                    ratio >= 3.0,
                    "{}: series {i} contrast {ratio:.2}",
                    theme.name
                );
            }
            let all = &theme.colors.categorical;
            for i in 0..all.len() {
                for j in i + 1..all.len() {
                    let d = |a: u8, b: u8| (a as f32 - b as f32).powi(2);
                    let dist = (d(all[i].r(), all[j].r())
                        + d(all[i].g(), all[j].g())
                        + d(all[i].b(), all[j].b()))
                    .sqrt();
                    assert!(
                        dist >= 60.0,
                        "{}: series {i} and {j} too close ({dist:.0})",
                        theme.name
                    );
                }
            }
            assert_eq!(theme.chart(8), theme.chart(0));
        }
    }

    #[test]
    fn sequential_ramp_grows_away_from_the_island() {
        for theme in [Theme::dark(), Theme::light()] {
            let c = &theme.colors;
            let gaps: Vec<f32> = c
                .sequential
                .iter()
                .map(|s| contrast(*s, c.island))
                .collect();
            assert!(
                gaps.windows(2).all(|w| w[1] > w[0]),
                "{}: ramp must separate steadily from the island: {gaps:?}",
                theme.name
            );
            assert!(gaps[4] >= 7.0, "{}: top of ramp {:.1}", theme.name, gaps[4]);
            assert_eq!(c.ramp(0.0), c.sequential[0]);
            assert_eq!(c.ramp(1.0), c.sequential[4]);
            assert_eq!(c.ramp(f32::NAN), c.sequential[0]);
        }
    }

    #[test]
    fn avatar_color_is_stable() {
        assert_eq!(avatar_color("archipel"), avatar_color("archipel"));
    }

    /// Value colors are readable on the editor background (they match the code editor).
    #[test]
    fn syntax_colors_are_readable_in_the_editor() {
        for theme in [Theme::dark(), Theme::light()] {
            let e = &theme.editor;
            for (name, color) in [
                ("string", e.string),
                ("number", e.number),
                ("keyword", e.keyword),
                ("constant", e.constant),
                ("type", e.type_name),
                ("annotation", e.annotation),
                ("property", e.property),
                ("punctuation", e.punctuation),
                ("comment", e.comment),
            ] {
                let ratio = contrast(color, e.background);
                assert!(ratio >= 3.0, "{}: {name} contrast {ratio:.2}", theme.name);
            }
        }
    }

    /// WCAG 2 contrast ratio.
    fn contrast(a: Color32, b: Color32) -> f32 {
        fn luminance(c: Color32) -> f32 {
            let channel = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
        }
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }
}
