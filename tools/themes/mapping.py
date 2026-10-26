"""THE mapping table: kit color fields -> IntelliJ keys. Written once, here.

Each entry is `(kit field, [source, ...])`. The first source that resolves wins; when none
does, the field keeps the kit's measured value (`Theme::dark()` for dark themes, `Theme::light()`
for light ones) and the generator records it as a fallback in `docs/theme-mapping.md`.

To map a new kit field, add a line to `COLORS` or `EDITOR`; nothing else needs to change. A
field that is not listed here is not touched: it keeps the measured value, so fields other
people add to `Colors` / `EditorColors` keep working before they get a mapping.

Source syntax
-------------
UI sources (resolved UIManager keys of the theme, see resolve_ui.py):
    ui:Key.name        a color key of the resolved theme, e.g. `ui:ToolWindow.background`
    scheme:NAME        a `<colors>` entry of the theme's default editor scheme, e.g.
                       `scheme:FILESTATUS_MODIFIED` (file status colors live in the scheme)
    self:field         an earlier-resolved field of the same struct
    mix:A|B|T          blend of two sources, `T` in 0..1 toward B (a number, or `num:Key` of
                       a numeric UI key, e.g. `num:EditorTabs.unselectedAlpha`)
Editor sources (resolved through the scheme's `parent_scheme` chain):
    attr:NAME.fg|bg|effect|stripe   a text attribute (foreground, background, effect color,
                                    error stripe color)
    color:NAME                      a `<colors>` entry (CARET_ROW_COLOR, ...)
    self:field, mix:A|B|T           as above
"""

# fmt: off
COLORS = [
    # Surfaces. `frame` is the window background between islands (Islands: main window,
    # main toolbar, status bar and stripes share it); classic themes have one panel color.
    ("frame", ["ui:MainWindow.background", "ui:StatusBar.background", "ui:ToolWindow.Stripe.background", "ui:MainToolbar.background", "ui:Panel.background"]),
    ("island", ["ui:ToolWindow.background", "ui:Tree.background", "ui:Panel.background"]),
    ("popup", ["ui:Popup.background", "ui:PopupMenu.background", "ui:Menu.background", "ui:Panel.background"]),
    ("popup_border", ["ui:Popup.borderColor", "ui:Popup.Border.color", "ui:Menu.borderColor", "ui:Borders.color"]),
    ("field", ["ui:TextField.background", "ui:TextArea.background"]),
    ("notification", ["ui:Notification.background", "ui:ToolTip.background", "self:popup"]),
    ("tooltip", ["ui:ToolTip.background", "ui:Tooltip.background", "self:popup"]),

    # Text.
    ("text", ["ui:Label.foreground", "ui:Panel.foreground", "ui:TextField.foreground"]),
    # Titles and selected rows: the default text color (List.selectionForeground is white on
    # the classic light themes' dark blue selection, so it cannot serve for titles).
    ("text_strong", ["ui:Label.foreground", "self:text"]),
    ("text_secondary", ["ui:StatusBar.Widget.foreground", "ui:Label.infoForeground", "ui:Component.infoForeground"]),
    ("text_placeholder", ["ui:Component.infoForeground", "ui:Label.infoForeground", "self:text_secondary"]),
    ("text_disabled", ["ui:Label.disabledForeground", "ui:Label.disabledText"]),
    ("text_on_accent", ["ui:Button.default.foreground"]),

    # Interaction.
    ("hover", ["ui:ActionButton.hoverBackground", "ui:StatusBar.Widget.hoverBackground"]),
    ("pressed", ["ui:ActionButton.pressedBackground"]),
    ("selection", ["ui:List.selectionBackground", "ui:Tree.selectionBackground"]),
    ("selection_popup", ["ui:CompletionPopup.selectionBackground", "ui:Popup.Selection.backgroundInactive", "self:selection"]),
    ("accent", ["ui:Button.default.startBackground", "ui:Button.default.endBackground"]),
    ("accent_disabled", ["ui:Button.default.disabledBackground"]),
    ("text_on_accent_disabled", ["ui:Button.default.disabledForeground"]),
    ("focus_ring", ["ui:Component.focusColor", "ui:Link.focusedBorderColor", "self:accent"]),
    ("border", ["ui:Component.borderColor", "ui:Button.startBorderColor", "ui:Button.borderColor"]),
    ("separator", ["ui:MainToolbar.separatorColor", "ui:ToolBar.separatorColor", "ui:Separator.separatorColor", "ui:Separator.foreground"]),
    ("group_separator", ["ui:Menu.separatorColor", "ui:Popup.separatorColor", "ui:Separator.separatorColor", "ui:Borders.color"]),
    ("tab_selected", ["ui:EditorTabs.underlinedTabBackground", "ui:EditorTabs.selectedBackground", "ui:DefaultTabs.underlinedTabBackground", "ui:TabbedPane.focusColor"]),
    ("tab_selected_border", ["ui:EditorTabs.underlinedBorderColor", "ui:EditorTabs.underlineColor", "ui:DefaultTabs.underlinedTabBorderColor", "self:accent"]),
    ("tab_text", ["ui:EditorTabs.foreground", "mix:self:text|self:island|num:EditorTabs.unselectedAlpha", "mix:self:text|self:island|0.25"]),
    ("tab_scroll_thumb", []),
    ("tab_scroll_thumb_hover", []),
    ("control_border", ["ui:ToggleButton.borderColor", "ui:Component.borderColor"]),
    ("toggle_on", ["ui:ToggleButton.onBackground", "ui:ProgressBar.passedColor"]),
    ("toggle_off", ["ui:ToggleButton.offBackground", "self:island"]),
    ("toggle_off_knob", ["ui:ToggleButton.buttonColor", "ui:ToggleButton.borderColor", "self:control_border"]),
    ("border_disabled", ["ui:CheckBox.disabledBorderColor", "ui:Button.disabledBorderColor", "ui:Component.disabledBorderColor"]),
    ("control", []),
    ("control_selected", ["ui:Button.default.startBackground", "ui:Button.default.endBackground"]),
    ("selection_inactive", ["ui:List.selectionInactiveBackground", "ui:Tree.selectionInactiveBackground", "ui:List.selectionBackgroundInactive"]),
    ("row_hover", ["ui:List.hoverBackground", "ui:Tree.hoverBackground", "ui:Table.hoverBackground", "self:hover"]),
    ("combo", ["ui:ComboBox.ArrowButton.nonEditableBackground", "ui:ComboBox.nonEditableBackground", "ui:ComboBox.background"]),
    ("combo_arrow", ["ui:ComboBox.ArrowButton.iconColor"]),
    ("clear_icon", []),
    ("clear_icon_hover", []),
    ("clear_hover", []),
    ("segment_selected", ["ui:SegmentedButton.selectedButtonColor"]),
    ("segment_selected_border", ["ui:SegmentedButton.selectedStartBorderColor", "ui:SegmentedButton.selectedEndBorderColor"]),
    ("progress_track", ["ui:ProgressBar.trackColor"]),
    ("counter", ["ui:Counter.background"]),
    ("counter_text", ["ui:Counter.foreground"]),
    ("tag", ["ui:Tag.background", "ui:List.Tag.background"]),

    # Semantic.
    ("error", ["ui:Label.errorForeground", "ui:SearchField.errorForeground", "ui:SpeedSearch.errorForeground", "ui:Component.errorFocusColor"]),
    ("warning", ["ui:Label.warningForeground", "ui:Component.warningFocusColor"]),
    ("success", ["ui:Label.successForeground", "ui:ProgressBar.passedColor"]),
    ("link", ["ui:Link.activeForeground", "ui:Link.hoverForeground", "ui:Link.foreground"]),
    ("banner_info.background", ["ui:Banner.infoBackground"]),
    ("banner_info.border", ["ui:Banner.infoBorderColor"]),
    ("banner_warning.background", ["ui:Banner.warningBackground"]),
    ("banner_warning.border", ["ui:Banner.warningBorderColor"]),
    ("banner_error.background", ["ui:Banner.errorBackground"]),
    ("banner_error.border", ["ui:Banner.errorBorderColor"]),
    ("banner_success.background", ["ui:Banner.successBackground"]),
    ("banner_success.border", ["ui:Banner.successBorderColor"]),

    # Domain accents. File status colors belong to the editor color scheme.
    ("run", ["ui:RunWidget.runIconColor", "ui:Badge.greenBackground"]),
    ("stop", ["ui:RunWidget.stopBackground", "ui:ProgressBar.failedColor"]),
    ("vcs_modified", ["scheme:FILESTATUS_MODIFIED"]),
    ("vcs_added", ["scheme:FILESTATUS_ADDED"]),
    ("vcs_untracked", ["scheme:FILESTATUS_UNKNOWN"]),
    ("vcs_ignored", ["scheme:FILESTATUS_DELETED"]),

    # Icons. `categorical` and `sequential` (chart palettes) are kit-defined, not IntelliJ's.
    ("icon", []),  # Component.iconColor is a 50 % alpha gray: not a stroke color
    ("folder_fill", []),
]

EDITOR = [
    ("background", ["attr:TEXT.bg", "color:CONSOLE_BACKGROUND_KEY"]),
    ("text", ["attr:TEXT.fg", "attr:DEFAULT_IDENTIFIER.fg"]),
    ("caret", ["color:CARET_COLOR"]),
    ("caret_row", ["color:CARET_ROW_COLOR"]),
    ("line_number", ["color:LINE_NUMBERS_COLOR"]),
    ("line_number_caret", ["color:LINE_NUMBER_ON_CARET_ROW_COLOR", "color:LINE_NUMBERS_COLOR"]),
    ("selection", ["color:SELECTION_BACKGROUND"]),
    ("background_panel", ["color:DOCUMENTATION_COLOR", "color:LOOKUP_COLOR"]),
    ("search_result", ["attr:TEXT_SEARCH_RESULT_ATTRIBUTES.bg", "attr:SEARCH_RESULT_ATTRIBUTES.bg"]),
    ("rename_occurrence", ["attr:IDENTIFIER_UNDER_CARET_ATTRIBUTES.bg", "attr:WRITE_IDENTIFIER_UNDER_CARET_ATTRIBUTES.bg"]),
    ("matched_brace", ["attr:MATCHED_BRACE_ATTRIBUTES.bg"]),
    ("indent_guide", ["color:INDENT_GUIDE"]),
    ("error_effect", ["attr:ERRORS_ATTRIBUTES.effect", "attr:ERRORS_ATTRIBUTES.stripe"]),
    ("warning_effect", ["attr:WARNING_ATTRIBUTES.effect", "attr:WARNING_ATTRIBUTES.stripe"]),
    ("weak_warning_effect", ["attr:INFO_ATTRIBUTES.effect"]),  # INFO_ATTRIBUTES is the weak warning key
    ("breakpoint_line", ["attr:BREAKPOINT_ATTRIBUTES.bg"]),
    ("execution_line", ["attr:EXECUTIONPOINT_ATTRIBUTES.bg"]),
    ("inline_value", ["attr:DEBUGGER_INLINED_VALUES.fg"]),
    ("error_stripe", ["attr:ERRORS_ATTRIBUTES.stripe", "attr:ERRORS_ATTRIBUTES.effect"]),
    ("warning_stripe", ["attr:WARNING_ATTRIBUTES.stripe", "attr:WARNING_ATTRIBUTES.effect"]),
    ("string", ["attr:DEFAULT_STRING.fg"]),
    ("number", ["attr:DEFAULT_NUMBER.fg"]),
    ("keyword", ["attr:DEFAULT_KEYWORD.fg"]),
    ("constant", ["attr:DEFAULT_CONSTANT.fg"]),
    ("type_name", ["attr:TYPE_PARAMETER_NAME_ATTRIBUTES.fg", "attr:DEFAULT_CLASS_REFERENCE.fg", "attr:DEFAULT_CLASS_NAME.fg"]),
    ("annotation", ["attr:DEFAULT_METADATA.fg"]),
    ("property", ["attr:DEFAULT_INSTANCE_FIELD.fg", "attr:DEFAULT_STATIC_FIELD.fg"]),
    ("punctuation", ["attr:DEFAULT_COMMA.fg", "attr:DEFAULT_BRACES.fg", "attr:DEFAULT_OPERATION_SIGN.fg", "mix:self:text|self:background|0.25"]),
    ("comment", ["attr:DEFAULT_LINE_COMMENT.fg", "attr:DEFAULT_BLOCK_COMMENT.fg"]),
    ("tag", ["attr:XML_TAG_NAME.fg", "attr:HTML_TAG_NAME.fg", "attr:DEFAULT_TAG.fg", "self:keyword"]),
    ("attribute", ["attr:XML_ATTRIBUTE_NAME.fg", "attr:HTML_ATTRIBUTE_NAME.fg", "attr:DEFAULT_ATTRIBUTE.fg", "self:annotation"]),
    ("variable", ["attr:DEFAULT_FUNCTION_DECLARATION.fg", "attr:DEFAULT_INSTANCE_METHOD.fg", "attr:DEFAULT_STATIC_METHOD.fg", "attr:DEFAULT_LOCAL_VARIABLE.fg"]),
    ("invalid", ["attr:BAD_CHARACTER.fg", "attr:WRONG_REFERENCES_ATTRIBUTES.fg", "attr:ERRORS_ATTRIBUTES.effect"]),
    ("search_result_current", []),  # no IntelliJ key: the find UI draws the current match
]
# fmt: on

# Fields of `Colors` / `EditorColors` the generator never maps (kit-defined data).
UNMAPPED_COLORS = {"categorical", "sequential"}

# Geometry keys recorded for every theme next to the colors (README, docs/theming.md).
# The table keeps every numeric UI key; these are the ones the docs call out.
GEOMETRY_KEYS = [
    "Island.arc", "Island.borderWidth", "Island.borderArcLength", "Island.inactiveAlpha",
    "Island.toolWindowAlpha", "Button.arc", "Component.arc", "Banner.arc", "Component.focusWidth",
    "Tree.rowHeight", "List.rowHeight", "TabbedPane.tabHeight", "TabbedPane.tabSelectionArc",
    "EditorTabs.underlineHeight", "EditorTabs.underlineArc", "ToolWindow.Header.height",
    "StatusBar.topBorderWidth", "Button.minimumSize", "TextField.minimumSize", "ComboBox.minimumSize",
    "StripeToolbar.Button.size", "CheckBox.iconSize",
]
