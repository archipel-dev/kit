# Icon review

## How a glyph is reviewed

Every new or changed glyph goes through four passes before it is merged.

1. **Lints** (`cargo test -p archipel-icons`): live area, pixel snapping, crisp fills,
   convex fills, stroke widths, tone contrast, unique names, keywords, valid SVG.
2. **Geometry** at 96 px on the grid (`REVIEW=1 cargo test -p archipel-icons --test catalog`,
   then `target/icon-review/`): keylines, symmetry, stroke joins, arrowheads, gaps.
3. **Actual size**: `img/actual-size.*.png` at 16 px and 1×, zoomed with nearest-neighbour
   scaling to see the pixels: are straight strokes one pixel wide, do small details merge,
   does the glyph's weight match its neighbours?
4. **In context**: the category sheet next to its siblings, in dark and light, and in a
   real component (toolbar, tree row) in the gallery.

Review questions:

- Is it recognizable at 16 px without its label?
- Does it look as heavy as its neighbours (no outlier in ink coverage)?
- Is the metaphor the familiar one? Is it distinct from similar glyphs (download vs
  scroll-end, file vs file-text)?
- Does it work in monochrome?

## Review log

### 2026-10 · Initial set (113 glyphs)

The set was designed for this kit (the earlier IDE icons were redrawn as data, and about 60
glyphs were added for the HTTP and database tools). Findings and what was done:

| Glyph | Finding | Pass | Resolution |
|---|---|---|---|
| `history` | Arrowhead reached x = 0.25, outside the live area | Lint | Moved the head inside 1..15. |
| `sun` | Horizontal and vertical rays touched the edge | Lint | Shortened the rays to end at 1.5 and 13.5. |
| `settings` | Tooth tips on 1.56 / 14.44: blurry at 1× | Lint | Snapped them to 1.5 / 14.5. |
| `new-folder`, `debug`, `test`, `diff`, `lightning` | Straight strokes on whole units (blurry) | Lint | Moved them to half units. |
| `rerun` | Filled triangle with an edge on 6.5 (soft edge) | Lint | Moved the edge to 6. |
| `weak-warning` (light) | #CCCCCC on white is 1.6:1, below the 3:1 non-text minimum | Lint | Muted tone darkened to #A88B4A (3.3:1). |
| `settings` | Eight teeth merged into a blob at 16 px | Actual size | Redrawn with six wider teeth. |
| `file-code` | The brackets were so small they read as a circle | Actual size | Larger, wider brackets. |
| `comment` | The bubble's bottom edge crossed the tail opening; a stray segment | Geometry | Outline redrawn as one path around the tail. |
| `debug` | Body slightly heavier than its neighbours | Actual size | Accepted: legs and the center seam need the room; revisit if a lighter bug is needed. |
| `image` | Sun and mountains crowd at 16 px | Actual size | Accepted for now; candidate for a simpler drawing. |

### 2026-10 · Round 2: centering and language icons (user feedback)

Feedback: "the language icons and add etc. are not centered; every programming language
should have an icon".

| Item | Finding | Resolution |
|---|---|---|
| 45 glyphs (`plus`, `add-box`, arrows, steps, folders, …) | Bounding box 0.5–0.75 units off center. Cause: the rule "center strokes on 7.5 for 1× crispness" put crispness above centering. | Rule changed: glyphs are centered on (8, 8), checked by a new lint (`glyphs_are_centered`, ±0.25); strokes stay on the half-unit grid, crisp at 2×. Every glyph moved by its rounded offset. |
| File-type badges | Letters were set in Inter at 8–9 px and positioned with hand-tuned offsets; they sat high or low depending on the letters. | Replaced by language tiles with a stroke font drawn on the grid: labels are centered by construction. |
| Languages | Only ~25 file types had a badge. | 78 languages and formats from GitHub Linguist, each with a tile in its Linguist color and a unique label. |
| Label contrast | Mid-tone tiles (Java) reached only 4.1:1 with near-black. | Labels are pure black or white: at least 4.58:1 on any tile (checked at 4.5:1). |
| Dark and very light tiles | Crystal, JSON, Ignore List, Lua, Gradle vanished on dark islands; EditorConfig and INI on white. | Low-contrast tiles get a soft outline that reads on both themes. |

### 2026-10 · Round 3: language icon design (user feedback)

Feedback: "the language icons don't look well designed". Five directions were prototyped
side by side at 16 and 24 px in dark and light: the stroke-font tile (then current), a tile
with a real font, a file outline with a color band, a tinted file and a circle. The user
picked **file outline + color band**.

| Item | Finding | Resolution |
|---|---|---|
| Stroke font | The 4×6 stroke letters looked like a pixel font; capitals (`RS`, `PY`) were stiff. | Removed. Labels use the bold UI font, centered on their ink; mixed case (`Rs`, `Py`). |
| Shape | A bare colored tile did not read as a file next to the glyphs. | File outline in the ink color with the language color as a band. |
| Light theme | First render drew the outline in white (invisible on white). | The renderer takes the theme ink. |
| Actual size | Labels at 6.4 units were mush at 1×. | Band and label enlarged (7.4 units); sheets now render with Inter SemiBold like the kit. |
| Java types | Same stroke font in the C/I/E/R/@ circles. | Same bold font, centered on its ink. |

### 2026-10 · Language icon styles (user request)

All five prototyped directions stay available as `LanguageStyle` (file with band, tile,
tinted file, circle, pixel tile), selected per app through `Theme::language_icons`. The
pixel tile keeps its own capital labels in the stroke font. Review: the tinted style drew
yellow (JavaScript) and near-black outlines that vanished on white or dark; its ink is now
mixed towards mid-gray until it reaches 2:1 on both backgrounds (checked at 1.9:1).

### 2026-10 · Symbol icons (user request)

Request: "interface, enum etc. icons for object-oriented languages like Java". Designed a
symbol system rather than one-off badges: shape for the category (circle type, rounded
square member, glyph namespace), letter for the kind, color for the group, overlays for
abstract, static and visibility. Colors were tuned into the luminance band 0.129–0.183, the
only band where white labels reach 4.5:1 and the shape 3:1 on both themes (checked).
Review: `module` first reused the package cube; it now uses the layers glyph. Overlay marks
touch the letter's corner at 16 px; accepted, as marks read as badges, like in other IDEs.

Open follow-ups: an outline/filled pair for toggles (star, pin, bookmark), and a 20 px
optical size for dense toolbars on high-DPI screens.
