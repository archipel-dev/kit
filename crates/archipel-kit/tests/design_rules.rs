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
        instead: "`Button`, `IconButton`, `Checkbox`, `Radio`, `Toggle`, `TextField`, \
                  `Dropdown`, `Link` … (they have the measured sizes, states and focus)",
        allowed: &[(
            "components/misc.rs",
            "`Dropdown` wraps egui's combo box popup",
        )],
    },
    Rule {
        name: "tooltips are kit tooltips",
        patterns: &[
            ".on_hover_text(",
            ".on_hover_ui(",
            ".on_disabled_hover_text(",
        ],
        instead: "`Tooltip::new(..).show_for(&response)` or `.with(Tooltip::new(..))` (the \
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
        "Banner",
        "composition of links and an icon button, audited in tests/feedback.rs",
    ),
    ("BreadcrumbsResponse", "response"),
    ("Bucket", "chart data"),
    ("ButtonVariant", "enum"),
    ("CellEdit", "table edit state"),
    ("ChartColor", "enum"),
    ("CollectionResponse", "response"),
    (
        "Completer",
        "trait: suggestions for TextField, tested in tests/completion.rs",
    ),
    ("Completion", "data"),
    ("Completions", "data"),
    ("Counter", "display only"),
    (
        "Dialog",
        "composition of buttons and fields, audited in tests/overlays.rs",
    ),
    ("DialogAction", "enum"),
    (
        "EmptyState",
        "composition with one button, audited in tests/feedback.rs",
    ),
    ("FieldLabel", "display only: names the field after it"),
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
    ("MenuHeader", "display only: a group title"),
    ("MenuSeparator", "display only"),
    ("MessageResponse", "response"),
    ("Orientation", "enum"),
    (
        "Panel",
        "composition of icon buttons, audited in tests/island_header.rs and data.rs",
    ),
    ("ProgressBar", "display only"),
    ("PropertyGrid", "display only"),
    ("RowResponse", "response"),
    ("SectionHeader", "display only"),
    ("Separator", "display only"),
    ("Severity", "enum"),
    ("SortOrder", "enum"),
    ("Sparkline", "chart, audited in tests/chart.rs"),
    ("Spinner", "display only"),
    ("SplitResponse", "response"),
    ("TabBarResponse", "response"),
    ("Tag", "display only"),
    ("TextFieldOutput", "response"),
    (
        "Toast",
        "composition of links and an icon button, audited in tests/feedback.rs",
    ),
    (
        "ToolStripe",
        "lays out `StripeButton`s, which conform; audited in tests/workbench.rs",
    ),
    ("Tooltip", "behaviour"),
    ("TooltipExt", "trait"),
    ("ValueCell", "display only"),
    ("WindowChrome", "enum"),
    ("WindowToolbar", "toolbar frame, audited in tests/window.rs"),
    (
        "Workbench",
        "layout of conforming parts, audited in tests/workbench.rs",
    ),
    ("WorkbenchPlace", "enum"),
    ("WorkbenchRegion", "enum"),
    ("WorkbenchState", "layout state"),
];

#[test]
fn every_interactive_component_passes_conformance() {
    let module = fs::read_to_string("src/components/mod.rs").expect("components module");
    let mut names = Vec::new();
    let mut statement = String::new();
    for line in module.lines() {
        if line.starts_with("pub use ") || !statement.is_empty() {
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
