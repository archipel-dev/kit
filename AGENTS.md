# Archipel Kit engineering guide

Archipel Kit is the UI component library of the Archipel tools (IDE, Nexus HTTP, Mongo
Raabit, plugins). Quality bar: it should feel like one polished suite.

## Non-negotiables

- Every component meets the checklist in `docs/contributing.md`: themed, all states, keyboard,
  accessible name and state, behavior + snapshot tests, a docs page, a gallery entry.
- Colors only from `Theme::colors(ctx)`, sizes only from `tokens`. No global mutable state:
  the theme lives in the `egui::Context`.
- Components are stateless: they borrow caller state or report what happened. No filesystem,
  network or app policy in the kit.
- No panics on user input or when `install` was not called (fonts fall back).
- Icons are data in `crates/archipel-icons/src/glyphs.rs`. New or changed glyphs follow
  `docs/icons/design.md`, pass the lints, and go through the review in `docs/icons/review.md`
  (large and actual-size sheets, dark and light); add a line to its review log.
- Look at the snapshot images you change (`docs/components/img/`) before committing; they
  are the docs.
- Clean-room: no third-party code, icons or artwork. Theme colors are Apache-2.0 facts (see
  `NOTICE.md`).

## Required checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Snapshot tests need a GPU or `mesa-vulkan-drivers`. Accept intended image changes with
`UPDATE_SNAPSHOTS=1 cargo test`; `*.old.png`, `*.new.png` and `*.diff.png` are comparison
artifacts and git-ignored.

Commit messages and PR titles are Conventional Commits (`feat`, `fix`, `perf`, `refactor`,
`docs`, `test`, `chore`, `ci`; `!` for breaking).
