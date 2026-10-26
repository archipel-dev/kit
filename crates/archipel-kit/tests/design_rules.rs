//! Design rules every component follows, checked in the source so a new component cannot
//! drift from the Islands look (measured on Android Studio, see `docs/component-map.md`).
//!
//! A failure names the file, the line and the rule, and says what to use instead. The
//! rules and their reasons are in `docs/design-rules.md`.

use std::fs;
use std::path::{Path, PathBuf};

/// A rule: a forbidden pattern in component source, why, and what to use instead.
struct Rule {
    name: &'static str,
    /// Any of these substrings on a code line breaks the rule.
    patterns: &'static [&'static str],
    instead: &'static str,
    /// Files (relative to `src/`) where the pattern is the one sanctioned use, with why.
    allowed: &'static [(&'static str, &'static str)],
}

const RULES: &[Rule] = &[
    Rule {
        name: "colors come from the theme",
        patterns: &[
            "Color32::from_rgb",
            "Color32::from_rgba",
            "Color32::from_gray",
            "Color32::from_black_alpha",
            "Color32::from_white_alpha",
            "Color32::from_hex",
        ],
        instead: "a field of `Theme::colors(ctx)` (add one, measured in both themes, if none \
                  fits), or a `Tone`",
        allowed: &[
            ("theme.rs", "the themes define the colors"),
            (
                "theme/catalog.rs",
                "turns the packed color tables generated from IntelliJ's theme files into \
                 `Color32` (the generated files hold numbers, not `Color32` literals)",
            ),
            (
                "icons.rs",
                "the icon artwork's colors are data, like the glyphs",
            ),
        ],
    },
    Rule {
        name: "corner radii come from tokens",
        patterns: &[
            "CornerRadius::same(1",
            "CornerRadius::same(2",
            "CornerRadius::same(3",
            "CornerRadius::same(4",
            "CornerRadius::same(5",
            "CornerRadius::same(6",
            "CornerRadius::same(7",
            "CornerRadius::same(8",
            "CornerRadius::same(9",
        ],
        instead: "a `tokens::radius` constant",
        allowed: &[],
    },
    Rule {
        name: "controls are kit components, not stock egui widgets",
        patterns: &[
            "ui.button(",
            "ui.small_button(",
            "ui.checkbox(",
            "ui.radio(",
            "ui.radio_value(",
            "ui.selectable_label(",
            "ui.selectable_value(",
            "ui.toggle_value(",
            "ui.hyperlink(",
            "ui.text_edit_singleline(",
            "ui.text_edit_multiline(",
            "egui::Button::",
            "egui::Checkbox::",
            "egui::RadioButton::",
            "egui::SelectableLabel::",
            "egui::Slider::",
            "egui::DragValue::",
            "egui::Hyperlink::",
            "egui::ComboBox::",
        ],
        instead: "`Button`, `ActionButton`, `JBCheckBox`, `JBRadioButton`, `OnOffButton`, `JBTextField`, \
                  `ComboBox`, `ActionLink` … (they have the measured sizes, states and focus)",
        allowed: &[(
            "openapi/ui/combo_box.rs",
            "`ComboBox` wraps egui's combo box popup",
        )],
    },
    Rule {
        name: "tooltips are kit tooltips",
        patterns: &[
            ".on_hover_text(",
            ".on_hover_ui(",
            ".on_disabled_hover_text(",
        ],
        instead: "`HelpTooltip::new(..).show_for(&response)` or `.with(HelpTooltip::new(..))` (the \
                  tooltip surface, shortcut and placement of the kit)",
        allowed: &[],
    },
    Rule {
        name: "animation timing comes from tokens",
        patterns: &[
            "animate_bool_with_time(",
            "animate_value_with_time(",
            "request_repaint_after_secs(0.",
        ],
        instead: "a `tokens::motion` duration (measured on Android Studio), passed by name",
        allowed: &[],
    },
];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Code lines of `text`: comments and doc examples are not checked.
fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
}

#[test]
fn components_follow_the_design_rules() {
    let root = Path::new("src");
    let mut files = Vec::new();
    sources(root, &mut files);
    files.retain(|path| !path.starts_with(root.join("testing")));
    let mut broken = Vec::new();
    for path in &files {
        let relative = path
            .strip_prefix(root)
            .expect("under src")
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(path).expect("source");
        for rule in RULES {
            if rule.allowed.iter().any(|(file, _)| *file == relative) {
                continue;
            }
            for (number, line) in code_lines(&text) {
                // An animation call breaks its rule only with a literal duration.
                let literal_duration = |pattern: &str| {
                    !pattern.starts_with("animate_")
                        || line
                            .split(pattern)
                            .nth(1)
                            .is_some_and(|args| args.contains("0.") || args.contains("1."))
                };
                if let Some(pattern) = rule
                    .patterns
                    .iter()
                    .find(|pattern| line.contains(**pattern) && literal_duration(pattern))
                {
                    broken.push(format!(
                        "src/{relative}:{number}: `{pattern}` breaks \"{}\"; use {} instead",
                        rule.name, rule.instead
                    ));
                }
            }
        }
    }
    assert!(
        broken.is_empty(),
        "design rule violations (see docs/design-rules.md):\n{}",
        broken.join("\n")
    );
}

/// Exported names that are not interactive widgets: data, responses, enums, behaviours, or
/// display-only parts. Everything else must pass `testing::conformance`.
const NOT_INTERACTIVE: &[(&str, &str)] = &[
    ("Avatar", "display only"),
    (
        "InlineBanner",
        "composition of links and an icon button, audited in tests/feedback.rs",
    ),
    ("BreadcrumbsComponentResponse", "response"),
    ("Bucket", "chart data"),
    ("ButtonVariant", "enum"),
    ("CellEdit", "table edit state"),
    ("ChartColor", "enum"),
    ("EditorComponentResponse", "response"),
    ("CollectionResponse", "response"),
    (
        "Completer",
        "trait: suggestions for JBTextField, tested in tests/completion.rs",
    ),
    ("Completion", "data"),
    ("Completions", "data"),
    ("Counter", "display only"),
    (
        "DialogWrapper",
        "composition of buttons and fields, audited in tests/overlays.rs",
    ),
    ("ExitCode", "enum"),
    (
        "DropZone",
        "pointer drop target around content, no focus or keys of its own; tested in tests/drag.rs",
    ),
    ("DropZoneResponse", "response"),
    (
        "StatusText",
        "composition with one button or links, audited in tests/feedback.rs and tests/status_and_loading.rs",
    ),
    ("CollapsiblePanelResponse", "response"),
    (
        "JBLoadingPanel",
        "veil over other components: audited in tests/status_and_loading.rs",
    ),
    ("ListSelectionModel", "selection state of collections"),
    ("SelectionMode", "enum"),
    ("SelectionTarget", "where a collection keeps its selection"),
    ("PUNCTUATION_MARKS", "constant"),
    (
        "SpeedSearch",
        "option of the collections, tested in tests/speed_search.rs",
    ),
    ("SpeedSearchState", "caller-owned state"),
    ("SpeedSearchComparator", "text matcher"),
    ("StatusTextResponse", "response"),
    ("FieldLabel", "display only: names the field after it"),
    ("Integer", "trait: the types a JBIntSpinner edits"),
    (
        "Form",
        "layout of other components, measured in tests/form.rs",
    ),
    ("FormUi", "layout handle of Form"),
    ("Histogram", "chart, audited in tests/chart.rs"),
    ("HistogramResponse", "response"),
    ("Kbd", "display only"),
    ("KeyValueRow", "row data"),
    ("KeyValueTableResponse", "response"),
    ("ListSeparator", "display only: a line or a group title"),
    (
        "JBPopup",
        "popup container, not a widget; exercised by tests/menus.rs and tests/template_field.rs",
    ),
    ("MessageResponse", "response"),
    ("Orientation", "enum"),
    (
        "SimpleToolWindowPanel",
        "composition of icon buttons, audited in tests/island_header.rs and data.rs",
    ),
    ("ProgressBar", "display only"),
    ("PropertyGrid", "display only"),
    ("CellRendererResponse", "response"),
    ("SeparatorComponent", "display only"),
    ("Status", "enum"),
    ("SortOrder", "enum"),
    ("Sparkline", "chart, audited in tests/chart.rs"),
    ("AsyncProcessIcon", "display only"),
    ("SplitterResponse", "response"),
    ("JBTabsResponse", "response"),
    ("Tag", "display only"),
    ("JBTextFieldOutput", "response"),
    (
        "Notification",
        "composition of links and an icon button, audited in tests/feedback.rs",
    ),
    (
        "ToolWindowToolbar",
        "lays out `SquareStripeButton`s, which conform; audited in tests/workbench.rs",
    ),
    ("HelpTooltip", "behaviour"),
    ("HelpTooltipExt", "trait"),
    ("ValueCell", "display only"),
    ("IdeFrameDecorator", "enum"),
    ("MainToolbar", "toolbar frame, audited in tests/window.rs"),
    (
        "ToolWindowPane",
        "layout of conforming parts, audited in tests/workbench.rs",
    ),
    ("ToolWindowPlace", "enum"),
    ("ToolWindowAnchor", "enum"),
    ("ToolWindowPaneState", "layout state"),
    // Overlays family (dialogs, notifications, tooltips, banners).
    (
        "EditorNotificationPanel",
        "composition of links and an icon button, audited in tests/notifications.rs",
    ),
    ("ProgressState", "enum"),
    ("OK_EXIT_CODE", "constant"),
    ("CANCEL_EXIT_CODE", "constant"),
    ("CLOSE_EXIT_CODE", "constant"),
    ("NEXT_USER_EXIT_CODE", "constant"),
    ("MessageIcon", "enum"),
    (
        "Messages",
        "builders of DialogWrapper, tested in tests/dialog_wrapper.rs",
    ),
    (
        "DialogValidation",
        "state of a dialog's validation, tested in tests/dialog_wrapper.rs",
    ),
    ("ValidationInfo", "data"),
    ("GotItDismissal", "enum"),
    ("GotItPosition", "enum"),
    ("GotItResponse", "response"),
    (
        "GotItTooltip",
        "overlay bubble with a button and links; keys, focus and timing tested in tests/tooltips.rs",
    ),
    ("HelpTooltipResponse", "response"),
    ("NotificationEvent", "response"),
    ("NotificationEventKind", "enum"),
    ("NotificationId", "data"),
    (
        "NotificationsManager",
        "balloon stack of cards that are audited in tests/notifications.rs",
    ),
    ("NotificationAction", "data"),
    ("NotificationDisplayType", "enum"),
    ("NotificationGroup", "data"),
    ("NotificationType", "enum"),
    ("DividerStyle", "enum"),
    ("LackOfSpaceStrategy", "enum"),
    ("StripeResponse", "response"),
    ("Content", "tool window content data"),
    ("ContentManager", "tool window content model"),
    ("ToolWindow", "tool window description and contents"),
    ("ToolWindowEvent", "enum"),
    (
        "StatusBar",
        "container of `StatusBarWidget`s, which conform; audited in tests/status_bar_widgets.rs",
    ),
    ("StatusBarAnchor", "enum"),
    ("StatusBarResponse", "response"),
    ("StatusBarWidgetItem", "widget description"),
    ("TextPresentation", "widget description"),
    ("IconPresentation", "widget description"),
    ("MultipleTextValuesPresentation", "widget description"),
    ("WidgetPresentation", "enum"),
    ("ActionButtonResponse", "response"),
    ("ButtonLook", "enum"),
    (
        "ActionPopupMenu",
        "popup container of ActionMenuItem rows, not a widget; keys, pointer and timings in tests/action_popup_menu.rs",
    ),
    ("ActionPopupMenuResponse", "response"),
    (
        "ActionToolbar",
        "layout of ActionButtons, which conform; behavior and audit in tests/action_toolbar.rs",
    ),
    ("ActionToolbarResponse", "response"),
    ("ToolbarOrientation", "enum"),
    ("ToolbarSplitResponse", "response"),
    ("PopupChain", "popup state owned by the caller"),
    (
        "BaseListPopupStep",
        "data of a ListPopup, tested in tests/list_popup.rs",
    ),
    (
        "ListPopup",
        "popup container of ActionMenuItem rows, not a widget; keys, pointer and timings in tests/list_popup.rs",
    ),
    ("ListPopupResponse", "response"),
    ("PopupStep", "trait: the rows of a ListPopup"),
    ("PopupStepResult", "enum"),
];

/// The modules whose `pub use` lines export the kit's widgets, relative to `src/`. The
/// `editor` module only counts its code view and editor re-exports (its highlighters are
/// data, not widgets), and `action_system` only its widgets (the rest is the action model).
const WIDGET_MODULES: &[(&str, &str)] = &[
    ("ui/mod.rs", "pub use "),
    ("ui/tabs/mod.rs", "pub use "),
    ("ui/popup/mod.rs", "pub use "),
    ("openapi/ui/mod.rs", "pub use "),
    ("ide/mod.rs", "pub use "),
    ("action_system/mod.rs", "pub use action_"),
    ("wm/mod.rs", "pub use "),
    ("notification/mod.rs", "pub use "),
    ("editor/mod.rs", "pub use editor_component"),
];

/// The uppercase names of the `pub use` statements of `module` that start with `prefix`.
fn exported_names(module: &str, prefix: &str) -> Vec<String> {
    let text = fs::read_to_string(Path::new("src").join(module)).expect("module source");
    let mut names = Vec::new();
    let mut statement = String::new();
    for line in text.lines() {
        if line.starts_with(prefix) || !statement.is_empty() {
            statement.push_str(line);
            if line.trim_end().ends_with(';') {
                let list = statement
                    .split_once('{')
                    .map_or(statement.rsplit("::").next().unwrap_or(""), |(_, rest)| {
                        rest
                    });
                names.extend(
                    list.trim_end_matches(';')
                        .trim_end_matches('}')
                        .split(',')
                        .map(|name| name.trim().to_owned())
                        .filter(|name| name.chars().next().is_some_and(char::is_uppercase)),
                );
                statement.clear();
            }
        }
    }
    names
}

#[test]
fn every_interactive_component_passes_conformance() {
    let names: Vec<String> = WIDGET_MODULES
        .iter()
        .flat_map(|(module, prefix)| exported_names(module, prefix))
        .collect();
    let conformance = fs::read_to_string("tests/conformance.rs").expect("conformance tests");
    let missing: Vec<_> = names
        .iter()
        .filter(|name| {
            !NOT_INTERACTIVE.iter().any(|(listed, _)| listed == name)
                && !conformance.contains(&format!("{name}::"))
                && !conformance.contains(&format!("{name} "))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "interactive components without a conformance test in tests/conformance.rs: \
         {missing:?}. Add one (enabled and disabled, with the keys it handles), or, if it is \
         not interactive, list it in NOT_INTERACTIVE with the reason."
    );
    let stale: Vec<_> = NOT_INTERACTIVE
        .iter()
        .filter(|(listed, _)| !names.iter().any(|name| name == listed))
        .collect();
    assert!(
        stale.is_empty(),
        "NOT_INTERACTIVE lists removed names: {stale:?}"
    );
}
