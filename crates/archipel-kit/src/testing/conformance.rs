//! The component checklist as a test: every interactive component, the kit's own and any
//! built on it, passes the same battery.

use super::{NodeExt, NodeT, Queryable, audit, harness_state};
use crate::theme::Theme;
use egui::Key;

/// Draws the component under test, `enabled` or disabled, and returns its response.
type App<'a> = Box<dyn FnMut(&mut egui::Ui, bool) -> egui::Response + 'a>;

/// A conformance check for one interactive component: see [`conformance`].
#[must_use = "run it with `.run()`"]
pub struct Conformance<'a> {
    label: String,
    app: App<'a>,
    keys: Vec<Key>,
    click: bool,
    focusable: bool,
}

/// Checks that the component drawn by `app` behaves like a kit component. `app` draws it
/// enabled or disabled (its `bool` argument) and returns its response; `label` is its
/// visible label (its accessible name).
///
/// [`Conformance::run`] checks, in both themes:
///
/// 1. It renders without panicking and passes [`audit`] (named, big enough, no overlaps,
///    reachable by Tab).
/// 2. It is found by its label, with an interactive role.
/// 3. Enabled: a click activates it (`response.clicked()`), Tab focuses it, and
///    <kbd>Enter</kbd> and <kbd>Space</kbd> activate it while focused.
/// 4. Disabled: it reports disabled, clicks and keys do nothing, Tab skips it.
///
/// ```no_run
/// use archipel_kit::components::Button;
/// use archipel_kit::testing;
///
/// testing::conformance("Run", |ui, enabled| ui.add(Button::primary("Run").enabled(enabled)))
///     .run();
/// ```
pub fn conformance<'a>(
    label: impl Into<String>,
    app: impl FnMut(&mut egui::Ui, bool) -> egui::Response + 'a,
) -> Conformance<'a> {
    Conformance {
        label: label.into(),
        app: Box::new(app),
        keys: vec![Key::Enter, Key::Space],
        click: true,
        focusable: true,
    }
}

impl Conformance<'_> {
    /// The keys that activate it while focused (default <kbd>Enter</kbd> and
    /// <kbd>Space</kbd>); empty for components keys don't activate (text fields).
    pub fn keys(mut self, keys: &[Key]) -> Self {
        self.keys = keys.to_vec();
        self
    }

    /// Components that a click doesn't "activate" (text fields take focus instead).
    pub fn no_click(mut self) -> Self {
        self.click = false;
        self
    }

    /// Components that are operated by pointer only and never take focus (rows inside a
    /// collection, which holds the focus itself).
    pub fn not_focusable(mut self) -> Self {
        self.focusable = false;
        self.keys.clear();
        self
    }

    /// Runs the checks.
    ///
    /// # Panics
    /// With a message naming the first check that failed.
    #[track_caller]
    pub fn run(mut self) {
        for theme in [Theme::dark(), Theme::light()] {
            let name = if theme.dark { "dark" } else { "light" };
            self.check_enabled(theme.clone(), name);
            self.check_disabled(theme, name);
        }
    }

    #[track_caller]
    fn check_enabled(&mut self, theme: Theme, name: &str) {
        let label = self.label.clone();
        let app = &mut self.app;
        let mut harness = harness_state(theme, 0usize, |ui, activations| {
            if app(ui, true).clicked() {
                *activations += 1;
            }
        });
        audit(&mut harness).assert_clean();
        let node = harness
            .query_by_label(&label)
            .unwrap_or_else(|| panic!("[{name}] no element labelled {label:?}"));
        assert!(
            !node.is_disabled(),
            "[{name}] {label:?} reports disabled while enabled"
        );

        if self.click {
            let before = *harness.state();
            harness.get_by_label(&label).click();
            harness.run_ok();
            assert_eq!(
                *harness.state(),
                before + 1,
                "[{name}] a click on {label:?} did not activate it"
            );
        }
        if self.focusable {
            harness.get_by_label(&label).focus();
            harness.run_ok();
            assert!(
                harness.get_by_label(&label).accesskit_node().is_focused(),
                "[{name}] {label:?} cannot take keyboard focus"
            );
            for key in self.keys.clone() {
                let before = *harness.state();
                harness.key_press(key);
                harness.run_ok();
                assert_eq!(
                    *harness.state(),
                    before + 1,
                    "[{name}] {key:?} on focused {label:?} did not activate it"
                );
                harness.get_by_label(&label).focus();
                harness.run_ok();
            }
        }
    }

    #[track_caller]
    fn check_disabled(&mut self, theme: Theme, name: &str) {
        let label = self.label.clone();
        let app = &mut self.app;
        let mut harness = harness_state(theme, 0usize, |ui, activations| {
            if app(ui, false).clicked() {
                *activations += 1;
            }
        });
        let node = harness
            .query_by_label(&label)
            .unwrap_or_else(|| panic!("[{name}] no element labelled {label:?} when disabled"));
        assert!(
            node.is_disabled(),
            "[{name}] disabled {label:?} doesn't report disabled"
        );
        harness.get_by_label(&label).click();
        harness.run_ok();
        // egui advertises a focus action on disabled widgets but never grants it: check
        // what happens, not what is advertised.
        harness.get_by_label(&label).focus();
        harness.run_ok();
        assert!(
            !harness.get_by_label(&label).accesskit_node().is_focused(),
            "[{name}] disabled {label:?} took keyboard focus"
        );
        for key in [Key::Tab, Key::Enter, Key::Space] {
            harness.key_press(key);
            harness.run_ok();
        }
        assert_eq!(
            *harness.state(),
            0,
            "[{name}] disabled {label:?} was activated"
        );
    }
}
