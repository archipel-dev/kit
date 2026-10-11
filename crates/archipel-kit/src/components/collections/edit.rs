//! Inline cell editing, shared by the collections that support it.
//!
//! The collection never owns the edited data. It owns only the transient edit session (which
//! cell, the text typed so far), kept in the `egui::Context` like scroll positions, and reports
//! the outcome as [`CellEdit`] in [`CollectionResponse::edited`](super::CollectionResponse).

use crate::parts::Slot;
use crate::theme::Theme;
use crate::tokens::{radius, typography};
use egui::{
    Context, CornerRadius, CursorIcon, Id, Key, Modifiers, Rect, Stroke, StrokeKind, TextEdit, Ui,
    WidgetInfo, WidgetType, vec2,
};

/// A committed inline edit: which cell, and the text the user typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellEdit {
    /// Row index, as passed to the cell callback.
    pub row: usize,
    /// Column index.
    pub column: usize,
    /// The new text. Validate it, store it, and show it from your cell callback.
    pub text: String,
}

/// An edit in progress.
#[derive(Clone)]
struct Session {
    row: usize,
    column: usize,
    text: String,
    /// The field has had keyboard focus: leaving it afterwards commits.
    focused_once: bool,
}

/// What the editor did this frame.
pub(super) enum Outcome {
    /// Still open.
    Open,
    /// <kbd>Enter</kbd> (the collection takes the focus back) or the field lost focus (the
    /// focus went elsewhere on purpose: leave it there).
    Commit(CellEdit, bool),
    /// <kbd>Escape</kbd>.
    Cancel,
}

fn session_id(id: Id) -> Id {
    id.with("edit-session")
}

fn field_id(id: Id) -> Id {
    id.with("edit-field")
}

fn column_id(id: Id) -> Id {
    id.with("edit-column")
}

/// The cell being edited, if any.
pub(super) fn current(ctx: &Context, id: Id) -> Option<(usize, usize)> {
    ctx.data(|d| d.get_temp::<Session>(session_id(id)))
        .map(|s| (s.row, s.column))
}

/// Opens the editor on a cell with its current `text`.
pub(super) fn begin(ctx: &Context, id: Id, row: usize, column: usize, text: &str) {
    ctx.data_mut(|d| {
        d.insert_temp(
            session_id(id),
            Session {
                row,
                column,
                text: text.to_owned(),
                focused_once: false,
            },
        );
    });
    // The field takes the focus when it is first drawn: a focus on a widget that doesn't
    // exist yet would name a node the accessibility tree doesn't have.
    ctx.request_repaint();
}

/// Closes the editor, handing the focus back to the collection if `refocus`.
pub(super) fn end(ctx: &Context, id: Id, refocus: bool) {
    ctx.data_mut(|d| d.remove::<Session>(session_id(id)));
    if refocus {
        ctx.memory_mut(|m| m.request_focus(id.with("focus")));
    }
    ctx.request_repaint();
}

/// The column <kbd>F2</kbd> and <kbd>Enter</kbd> edit: the one last clicked.
pub(super) fn active_column(ctx: &Context, id: Id) -> Option<usize> {
    ctx.data(|d| d.get_temp(column_id(id)))
}

/// Remembers the clicked column.
pub(super) fn set_active_column(ctx: &Context, id: Id, column: usize) {
    ctx.data_mut(|d| d.insert_temp(column_id(id), column));
}

/// Draws the editor over a cell. `rect` is the cell; `title` names the field for screen
/// readers. The field is placed through a [`Slot`], so it is a widget in the cell like any
/// other slot content.
fn field(ui: &mut Ui, id: Id, rect: Rect, title: &str, mono: bool) -> Outcome {
    let Some(mut session) = ui.ctx().data(|d| d.get_temp::<Session>(session_id(id))) else {
        return Outcome::Cancel;
    };
    let colors = Theme::colors(ui.ctx());
    let field_rect = rect.shrink2(vec2(1.0, 2.0));
    ui.painter().rect(
        field_rect,
        CornerRadius::same(radius::CONTROL),
        colors.field,
        Stroke::new(1.0_f32, colors.accent),
        StrokeKind::Inside,
    );

    let field = field_id(id);
    let cancel = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
    let mut response = None;
    let text = &mut session.text;
    let inner = field_rect.shrink2(vec2(super::CELL_PADDING - 1.0, 0.0));
    let slot = Slot::ui_width(inner.width(), |ui| {
        let font = if mono {
            egui::FontId::monospace(typography::CODE)
        } else {
            typography::body(typography::BODY)
        };
        let edit = TextEdit::singleline(text)
            .id(field)
            .frame(false)
            .font(font)
            .text_color(colors.text_strong)
            .desired_width(inner.width())
            .margin(vec2(0.0, 0.0));
        response = Some(ui.add(edit));
    });
    slot.show(
        ui,
        inner,
        crate::parts::State {
            enabled: true,
            ..Default::default()
        },
    );
    let response = response.expect("the slot draws the field");
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_label(format!("Edit {title}"));
    });
    // Reads `WidgetInfo` into the tree too, so tests and screen readers see a text input.
    response
        .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, format!("Edit {title}")));
    if response.hovered() || response.has_focus() {
        crate::cursor::on_hover(&response, CursorIcon::Text);
    }

    let (row, column) = (session.row, session.column);
    if !session.focused_once && !response.has_focus() {
        response.request_focus();
    }
    if response.has_focus() {
        session.focused_once = true;
    }
    let left = session.focused_once && !response.has_focus();
    let outcome = if cancel {
        Outcome::Cancel
    } else if left {
        Outcome::Commit(
            CellEdit {
                row,
                column,
                text: session.text.clone(),
            },
            ui.input(|i| i.key_pressed(Key::Enter)),
        )
    } else {
        Outcome::Open
    };
    ui.ctx()
        .data_mut(|d| d.insert_temp(session_id(id), session));
    outcome
}

/// The edit session of one frame of a collection: which cell is open, whether its editor was
/// drawn, and what it did. Shared by [`Table`](super::Table) and
/// [`TreeTable`](super::TreeTable) so both edit the same way.
pub(super) struct Frame {
    id: Id,
    editing: Option<(usize, usize)>,
    drawn: bool,
    outcome: Option<Outcome>,
}

impl Frame {
    pub(super) fn new(ctx: &Context, id: Id) -> Self {
        Self {
            id,
            editing: current(ctx, id),
            drawn: false,
            outcome: None,
        }
    }

    /// Whether the editor is open on this cell.
    pub(super) fn is_editing(&self, row: usize, column: usize) -> bool {
        self.editing == Some((row, column))
    }

    /// Draws the editor over the cell `rect` (call it instead of painting the cell).
    pub(super) fn show(&mut self, ui: &mut Ui, rect: Rect, title: &str, mono: bool) {
        self.drawn = true;
        self.outcome = Some(field(ui, self.id, rect, title, mono));
    }

    /// Ends the frame: applies what the editor did to `response`, and starts an edit when
    /// <kbd>F2</kbd> or <kbd>Enter</kbd> was pressed on the selected row (the last clicked
    /// column, else the first editable one). `Enter` then doesn't activate the row.
    pub(super) fn finish(
        self,
        ctx: &Context,
        response: &mut super::CollectionResponse,
        keys: &super::Keys,
        selected: Option<usize>,
        columns: usize,
        mut cell: impl FnMut(usize, usize) -> (bool, String),
    ) {
        let id = self.id;
        // A row scrolled out of view takes its editor with it: cancel rather than commit
        // something the user can't see.
        if self.editing.is_some() && !self.drawn {
            end(ctx, id, false);
        }
        match self.outcome {
            Some(Outcome::Commit(edited, refocus)) => {
                end(ctx, id, refocus);
                response.edited = Some(edited);
            }
            Some(Outcome::Cancel) => end(ctx, id, true),
            Some(Outcome::Open) | None => {}
        }
        if self.editing.is_none()
            && (keys.f2 || keys.enter)
            && let Some(row) = selected
        {
            let active = active_column(ctx, id).filter(|col| *col < columns);
            let target = active
                .filter(|col| cell(row, *col).0)
                .or_else(|| (0..columns).find(|col| cell(row, *col).0));
            if let Some(col) = target {
                begin(ctx, id, row, col, &cell(row, col).1);
                if keys.enter {
                    response.activated = None;
                }
            }
        }
    }
}
