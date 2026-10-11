# Icon design rules

The rules make 113 glyphs drawn at different times look like one family. The **checked**
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
- **Pixel snapping** (checked): straight horizontal and vertical 1-unit strokes sit on
  half units (x.5), so at 1× they fill exactly one pixel row instead of blurring over two.
  That is why symmetric glyphs with a center stroke (plus, arrows, the step icons) are
  centered on 7.5, not 8.
- **Corner radius 1.5** for containers (windows, boxes), 1 for small inner boxes.
- **Arrowheads** are 3.5 units long at 45° and open (two strokes), never filled.

## Fills

- Fills are the exception: play, stop, pause, breakpoint and the status badges.
- **Crisp edges** (checked): straight horizontal and vertical edges of filled shapes sit on
  whole units.
- **Convex polygons only** (checked): egui fills convex polygons; anything else is built from
  several shapes.

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
