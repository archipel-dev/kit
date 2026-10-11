# Contributing a component

A component is done when it meets every point below. The checklist is what makes the kit
feel like one suite instead of a pile of widgets.

1. **API**: a builder that implements `egui::Widget`, or has `show(ui)` when it reports more
   than a `Response`. Stateless: it borrows the caller's state or reports what happened.
   Every public item is documented, with a compiled example.
2. **Theme**: colors only from `Theme::colors(ctx)`, sizes only from `tokens`. No hard-coded
   colors (except white on accent knobs), no global state.
3. **States**: enabled, disabled, hover, pressed, focused (focus ring), selected where it
   applies, in dark and light.
4. **Keyboard**: reachable with <kbd>Tab</kbd>; <kbd>Enter</kbd>/<kbd>Space</kbd> activate.
5. **Accessibility**: `response.widget_info(...)` with the right role, the visible text as
   its name and its checked/selected/disabled state, so it can be found with
   `get_by_label`.
6. **Tests** in `crates/archipel-kit/tests/`: behavior through clicks and keys, assertions
   on accessible state, and a `snapshot_themes` image of every visual state.
7. **Docs**: a page in `docs/components/` with the snapshot (`<picture>` with the dark and
   light image), a usage example, an API table, behavior, accessibility, and links to the
   source and tests; a row in `docs/README.md`.
8. **Gallery**: the component on its gallery page, with its states.
9. **Built from parts**: `State` for colors, `RowContent`/`Surface`/`Popover` where they
   fit, sizes from `tokens`, cursor via `cursor::on_hover` / `on_hover_action`. No
   one-off painting for something a part does. See
   [building a component](building-components.md).
10. **Slots and behaviours**: content the caller decides (counts, actions, extra widgets)
    goes in a `Slot`. Components with a `show` method take behaviours with `with` and
    attach them last. Tooltips, context menus and drag and drop are never built into a
    component.
11. **Conformance**: interactive components pass `testing::conformance` in
    `tests/conformance.rs`, enabled and disabled, with the keys they handle
    (`.keys(...)`).
12. **Audit**: the component and every composition shown in its docs pass
    `testing::audit`, and so does its gallery page. Run the audit with the slots filled:
    no unnamed controls, no target under 16×16, no partly overlapping targets, nothing
    <kbd>Tab</kbd> can't reach.
13. **No duplicates**: before writing a helper (elision, span layout, a popup, a color
    mapping), search the kit for one; extend it instead of adding a second. One concept,
    one place: a token kind has one color, wherever it is drawn.
14. **API that can grow**: public enums that may get variants (actions, kinds, events)
    and structs callers may build by literal are `#[non_exhaustive]`, with constructors.
    A change that breaks callers anyway is marked `!` in the PR title with a
    `BREAKING CHANGE:` footer, so release-please bumps the version.
15. **Proof stays out of the repo**: verification screenshots go in the PR description.
    Only snapshot images a docs page uses live under `docs/`.

A component that fails conformance or the audit is not done, whatever its snapshot looks
like: they are the checks a user would otherwise do by hand.

`cargo test --workspace` checks the docs too: every image a page references exists, every
snapshot is used by a page, every component is documented, and every code block compiles.

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
(`feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`).
