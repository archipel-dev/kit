# Testing

Archipel Kit tests UI the way web frameworks do: find elements by **role and visible label**
(like Testing Library), drive them with **clicks and keys**, assert on **state and
accessibility**, and compare **screenshots** in both themes. The same helpers are public in
`archipel_kit::testing` (feature `testing`) for your own panels and plugins.

```toml
[dev-dependencies]
archipel-kit = { git = "https://github.com/archipel-dev/kit", features = ["testing"] }
```

## Behavior tests

`harness_state` renders your UI on an island with the kit installed and keeps your state, so
the test can drive the UI and inspect the result:

```rust
use archipel_kit::components::{Checkbox, TextField};
use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::Theme;

#[derive(Default)]
struct Form {
    name: String,
    private: bool,
}

let mut harness = testing::harness_state(Theme::dark(), Form::default(), |ui, form| {
    ui.add(TextField::new(&mut form.name).label("Repository name"));
    ui.add(Checkbox::new(&mut form.private, "Private"));
});

let input = harness.get_by_label("Repository name");
input.focus();
input.type_text("kit");
harness.get_by_label("Private").click();
harness.run();

assert_eq!(harness.state().name, "kit");
assert!(harness.state().private);
assert!(harness.get_by_label("Private").is_checked());
```

Queries (from [kittest](https://github.com/rerun-io/kittest)): `get_by_label`,
`get_by_role`, `get_by_role_and_label`, `get_by_label_contains`, `query_by_*` (returns
`Option`, for asserting absence) and `get_all_by_*`. Actions: `click()`, `hover()`,
`focus()`, `type_text()`, `harness.key_press(Key)`. `NodeExt` adds `is_checked()`,
`is_disabled()`, `accessible_label()`, `numeric_value()` and `role()`.

If a test cannot find your widget by its label, a screen reader cannot either: fix the
widget's accessible name rather than the test.

## Snapshot tests

`snapshot_themes` renders a UI in dark and light at 2× and compares it with
`{dir}/{name}.dark.png` and `{name}.light.png`:

```rust,no_run
use archipel_kit::components::{Banner, Severity};
use archipel_kit::testing;

testing::snapshot_themes("sync_banner", "tests/snapshots", |ui| {
    Banner::new(Severity::Warning, "Gradle sync needed").action("Sync").show(ui);
});
```

- Missing or different images fail the test; `UPDATE_SNAPSHOTS=1 cargo test` writes new
  ones (the old image is kept as `.old.png`, a diff as `.diff.png`).
- Commit the images: reviewers see UI changes as image diffs in the pull request.
- Rendering uses wgpu. Without a GPU (CI), install a software Vulkan driver: on Ubuntu
  `sudo apt-get install mesa-vulkan-drivers`.

## Tips

- Interaction harnesses run at 1×: kittest clicks at AccessKit bounds, which are in physical
  pixels. Snapshots use 2× for crisp images.
- Popups (dropdowns, menus) open outside the content: enlarge the canvas first with
  `harness.set_size(...)`.
- Animated widgets (spinners) never settle; `harness.run()` panics after 30 steps, so use
  `harness.run_ok()` or `run_steps(n)` around them.
