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
