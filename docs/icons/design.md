# Icon design rules

The rules make 128 glyphs drawn at different times look like one family. The **checked**
rules are enforced by `crates/archipel-icons/tests/lint.rs`; the rest are judged in review.

## Grid

- **16×16 units**, one unit = one pixel at 1×.
- **Live area 1..15** (checked): a 1-unit margin on every side, including round stroke
  caps, so glyphs never touch their neighbours or clip.
- **Keylines** for optical balance. Pick the one that matches the shape, so a circle does
  not look smaller than a square:

  | Shape | Keyline (stroke centers) |
  |---|---|
  | Circle | radius 6 to 6.5 around (8, 8) or (7.5, 7.5) |
  | Square | 2.5..13.5 |
  | Landscape rectangle | 1.5..14.5 × 2.5..13.5 |
  | Portrait rectangle (pages, phones) | 3.5..12.5 × 1.5..14.5 |

## Strokes

- **1 unit wide** for all ink outlines (checked); 1.5 only for the marks of status glyphs,
  such as the check of `success` or the `!` of `error`.
- **Round caps and joins**: friendly, and corners stay legible at 16 px.
- **Centered** (checked): every glyph's bounding box is centered on (8, 8), within a
  quarter unit. Symmetric glyphs with a center stroke (plus, arrows, the step icons) put
  that stroke on 8.
- **Half-unit grid** (checked): straight horizontal and vertical strokes sit on whole or
  half units, so they cover whole pixels at 2×, the density the set is designed for. At 1×
  a stroke on a whole unit (such as a center stroke on 8) is slightly soft; centering wins
  over 1× crispness.
- **Corner radius 1.5** for containers (windows, boxes), 1 for small inner boxes.
- **Arrowheads** are 3.5 units long at 45° and open (two strokes), never filled.

## Fills

- Fills are the exception: play, stop, pause, breakpoint and the status badges.
- **Crisp edges** (checked): straight horizontal and vertical edges of filled shapes sit on
  the half-unit grid.
- **Convex polygons only** (checked): egui fills convex polygons; anything else is built from
  several shapes.

## Language icons

Five styles share color and label (see `LanguageStyle`); the rules below are for the default,
*file with band*. The pixel tile uses the stroke font in `letters.rs` (4×6 units, capitals).


- **File outline** in the ink color, from 1.5 to 6.5: the page ends half a unit above the
  band, so the band never fights the outline.
- **Band** 1..15 × 7.5..15, radius 2, in the language's Linguist color.
- **Label**: one to three characters in the bold UI font, centered on its ink (not its line
  box) at (8, 11.25); 7.4 units for one or two characters, 6.2 for three. Mixed case reads
  better than capitals at this size (`Rs`, `Kt`, `Py`); symbols where a format is better
  known by them (`{}` JSON, `</>` XML, `>_` Shell, `#` CSS).
- **Label color**: black or white, whichever contrasts more with the band; at least 4.5:1
  (checked). Pure black and white guarantee that for any band color.
- **Outline**: bands below 1.6:1 against the dark background or 1.3:1 against the light one
  get a 1-unit outline mixed halfway towards mid-gray, so they read on both.
- Labels are unique (checked) and no language logos are used.

## Color

- One ink color; meaning comes from shape, not color. A glyph must work in monochrome.
- Semantic tones only for status: error red, warning amber, success green, info blue, run
  green, stop red, muted for weak warnings.
- **Contrast** (checked): every tone meets the WCAG 2.1 non-text contrast of 3:1 against its
  background, in dark and light.

## Metaphors

- Prefer the metaphor developers already know (magnifier, gear, branch graph, play
  triangle) over a clever new one.
- One concept, one glyph: revert, undo and rollback all use `undo`; variants and environments
  use `layers`.
- Details that vanish at 16 px are removed, not shrunk: the bell has no top nub, the gear has
  six teeth.
