# Building a component

New components are built from the kit's **parts**, not from scratch. A component made of
parts gets the kit's spacing, colors, hover and focus behavior, and cursor for free. Two
tests then prove that it also *behaves* like a kit component:

- `testing::conformance` presses it like a user would.
- `testing::audit` looks at the whole screen.

This page builds `ResultRow`, the row of a Find in Files result: an icon, the file name, its
path, a match count at the right end, and a hover action. Every step uses a public part, so a
plugin or an app can do the same.

## 1. Pick the parts

| Need | Part |
|---|---|
| Hover, press, focus, selection, disabled → colors | [`State`](../crates/archipel-kit/src/parts/state.rs) |
| Icon, label and dimmed secondary text, aligned like every row | [`RowContent`](../crates/archipel-kit/src/parts/row.rs) |
| The caller's content at the right end (count, action) | [`Slot`](../crates/archipel-kit/src/parts/slot.rs) |
| Rounded row highlight, focus ring | `row_pill`, `focus_ring` |
| Cursor | `cursor::on_hover` (rows keep the arrow; actions use `cursor::on_hover_action`) |
| Tooltips, context menus, drag and drop | [behaviours](behaviours.md): accept them with `with`, don't build them in |
| Colors by meaning | [`Tone`](../crates/archipel-kit/src/parts/tone.rs) |
| Sizes | `tokens::size` (never literals) |

If something you need is not a part yet, add the part first. Don't paint around the missing
part inside one component.

## 2. Write the builder

Builders are stateless. They borrow the caller's data and return what happened. The
component takes **slots** where the caller decides the content, and **behaviours** for
interaction that is not part of its look.

```rust
use archipel_kit::behaviour::{Behaviour, Behaviours};
use archipel_kit::parts::{RowContent, Slot, State, focus_ring, row_pill};
use archipel_kit::tokens::size;
use archipel_kit::{Icon, Theme, cursor, egui};
use egui::{Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// One Find in Files result: file, path, and a trailing slot for the match count.
pub struct ResultRow<'a> {
    file: &'a str,
    path: &'a str,
    icon: Icon,
    selected: bool,
    enabled: bool,
    trailing: Option<Slot<'a>>,
    behaviours: Behaviours<'a>,
}

impl<'a> ResultRow<'a> {
    pub fn new(file: &'a str, path: &'a str) -> Self {
        Self {
            file,
            path,
            icon: Icon::for_file(file),
            selected: false,
            enabled: true,
            trailing: None,
            behaviours: Behaviours::default(),
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The match count, or actions.
    pub fn trailing(mut self, slot: Slot<'a>) -> Self {
        self.trailing = Some(slot);
        self
    }

    /// A tooltip, context menu, drag source …
    pub fn with(mut self, behaviour: impl Behaviour + 'a) -> Self {
        self.behaviours.push(behaviour);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        // Size from tokens; the whole row is the target.
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), size::ROW_HEIGHT), sense);

        // Cursor and accessible name: what the user and the screen reader see.
        cursor::on_hover(&response, egui::CursorIcon::Default);
        response.widget_info(|| {
            let mut info =
                WidgetInfo::selected(WidgetType::SelectableLabel, self.enabled, self.selected, self.file);
            info.label = Some(format!("{} {}", self.file, self.path));
            info
        });

        // One State drives every color.
        let state = State::of(&response).selected(self.selected);
        let colors = Theme::colors(ui.ctx());
        if ui.is_rect_visible(rect) {
            row_pill(ui.painter(), rect, state.row_fill(&colors));
            if state.focused && !state.selected {
                focus_ring(ui.painter(), rect.shrink(size::ROW_PILL_INSET), 4, &colors);
            }
            let content = RowContent {
                icon: Some(self.icon),
                secondary: Some(self.path),
                ..RowContent::new(self.file)
            };
            content.paint(ui, rect, rect.left() + size::ROW_PILL_INSET + 8.0, state);
        }

        // The slot draws after the row, so its widgets sit on top and get the clicks.
        if let Some(slot) = self.trailing {
            let width = slot.width(ui);
            let right = rect.right() - size::ROW_PILL_INSET - 4.0;
            let slot_rect = Rect::from_min_max(pos2(right - width, rect.top()), pos2(right, rect.bottom()));
            slot.show(ui, slot_rect, state);
        }

        // Behaviours last: they see the final response.
        self.behaviours.attach(ui, response)
    }
}

# egui::__run_test_ui(|ui| {
use archipel_kit::components::Tooltip;
let row = ResultRow::new("main.rs", "src/")
    .trailing(Slot::text("3"))
    .with(Tooltip::new("3 matches"))
    .show(ui);
# let _ = row;
# });
```

Use slots instead of options when the caller decides what goes there: a count today, an
action tomorrow. A slot paints text and icons in the row's state, so they dim when the row
is disabled and strengthen when it is selected. Widgets in a slot draw in a child `Ui`, so
they get their own clicks without a special case in the row.

Accept behaviours with `with` instead of building tooltips, menus or drag handling into a
component. Every component then gets them, and they look the same everywhere.

## 3. Prove it conforms

`conformance` runs the component checklist as a test, in both themes:
- It renders and passes the audit.
- It is found by its label.
- Click, <kbd>Tab</kbd>, <kbd>Enter</kbd> and <kbd>Space</kbd> work.
- Disabled, it ignores all of them.

```rust,no_run
use archipel_kit::parts::Slot;
use archipel_kit::testing;
# struct ResultRow;
# impl ResultRow {
#     fn new(_: &str, _: &str) -> Self { Self }
#     fn trailing(self, _: Slot<'_>) -> Self { self }
#     fn enabled(self, _: bool) -> Self { self }
#     fn show(self, ui: &mut archipel_kit::egui::Ui) -> archipel_kit::egui::Response { ui.label("") }
# }

testing::conformance("main.rs src/", |ui, enabled| {
    ResultRow::new("main.rs", "src/")
        .trailing(Slot::text("3"))
        .enabled(enabled)
        .show(ui)
})
.run();
```

## 4. Audit the composition

`audit` checks a whole screen:
- Every target is named and at least 16×16.
- No two targets partly overlap.
- <kbd>Tab</kbd> reaches every focusable element.

Run it on the component with its slots filled, because that is where compositions break.
An icon button in a row is the classic case: a regular `IconButton` (26 points) sticks out
of a 24-point row, and the audit reports the overlap. `IconButton::small` fits.

```rust,no_run
use archipel_kit::components::{IconButton, ListRow};
use archipel_kit::parts::Slot;
use archipel_kit::testing::{self, Problem};
use archipel_kit::{Glyph, Theme};

let mut harness = testing::harness(Theme::dark(), |ui| {
    ListRow::new("app")
        .trailing(Slot::ui(|ui| {
            ui.add(IconButton::new(Glyph::Close, "Remove app").small(true));
        }))
        .show(ui);
});
harness.run();
let audit = testing::audit(&mut harness);
assert!(audit.of(Problem::Overlap).is_empty());
audit.assert_clean();
```

## 5. Snapshot, document, show

1. **Snapshot every state**, in dark and light. The images are the docs.

   ```rust,no_run
   # use archipel_kit::testing;
   testing::snapshot_themes("result_row", "../../docs/components/img", |ui| {
       // normal, selected, disabled, with a slot …
   });
   ```

2. **Document it**: write a page in `docs/components/`, add a row in `docs/README.md`, and
   add its code blocks to the doctest list in `lib.rs`.
3. **Add it to the gallery** with its states. The gallery test audits every page.

Then go through the [checklist](contributing.md). A component is done when every box is
ticked.
