//! The find bar shared by [`CodeView`](super::CodeView) and [`CodeEditor`](super::CodeEditor),
//! built from kit components.

use crate::Glyph;
use crate::components::{IconButton, TextField};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{
    EventFilter, Id, Key, Margin, Modifiers, Rect, RichText, Stroke, Ui, UiBuilder, pos2, vec2,
};

/// Find state of one view, kept in the egui context between frames.
#[derive(Clone, Default)]
pub(super) struct FindState {
    pub open: bool,
    pub query: String,
    pub current: usize,
    /// Move focus to the query field on the next frame.
    pub focus: bool,
    /// Scroll the current match into view on the next frame.
    pub jump: bool,
    pub field: Option<Id>,
}

impl FindState {
    pub fn load(ctx: &egui::Context, id: Id) -> Self {
        ctx.data(|d| d.get_temp(id.with("archipel-code-find")))
            .unwrap_or_default()
    }

    pub fn store(self, ctx: &egui::Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id.with("archipel-code-find"), self));
    }

    /// Whether the query field has the keyboard focus.
    pub fn field_focused(&self, ctx: &egui::Context) -> bool {
        self.open && self.field.is_some_and(|f| ctx.memory(|m| m.has_focus(f)))
    }

    pub fn open(&mut self) {
        self.open = true;
        self.focus = true;
    }

    /// Steps to the next (`+1`) or previous (`-1`) of `count` matches.
    pub fn step(&mut self, delta: isize, count: usize) {
        if count > 0 {
            self.current = (self.current as isize + delta).rem_euclid(count as isize) as usize;
            self.jump = true;
        }
    }

    /// Ctrl+F (Cmd+F) and Escape, for a view with `focused` keyboard focus. Returns
    /// whether the bar was closed with Escape, so the caller can take the focus back.
    pub fn keys(&mut self, ui: &Ui, focused: bool) -> bool {
        let field_focused = self.field_focused(ui.ctx());
        if !(focused || field_focused) {
            return false;
        }
        let (find, escape) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::F),
                self.open && i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if find {
            self.open();
        }
        if escape {
            self.open = false;
        }
        escape
    }
}

/// Width of the bar: its field, count, three buttons, gaps and padding.
const BAR_WIDTH: f32 = 290.0;

/// What the user did in the bar this frame.
#[derive(Default)]
pub(super) struct BarEvents {
    pub query_changed: bool,
    pub step: isize,
}

/// Draws the find bar in the top right corner of `area` and applies its events to `state`.
/// `count` is the number of matches.
pub(super) fn bar(ui: &mut Ui, area: Rect, state: &mut FindState, count: usize) -> BarEvents {
    let theme = Theme::current(ui.ctx());
    let (colors, editor) = (theme.colors, theme.editor);
    let inset = size::CODE_PADDING / 2.0;
    let mut events = BarEvents::default();
    let width = BAR_WIDTH.min(area.width() - 2.0 * inset);
    let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(
        pos2(area.right() - inset - width, area.top() + inset),
        vec2(width, area.height() - inset),
    )));
    let more_than = if count >= super::text::MAX_MATCHES {
        "+"
    } else {
        ""
    };
    let status = if state.query.is_empty() {
        String::new()
    } else if count == 0 {
        "No results".to_owned()
    } else {
        format!("{} of {count}{more_than}", state.current.min(count - 1) + 1)
    };
    egui::Frame::new()
        .fill(editor.background_panel)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .corner_radius(radius::CONTROL)
        .inner_margin(Margin::same(4))
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing.x = size::TOOLBAR_GAP;
            ui.horizontal(|ui| {
                ui.set_height(size::CONTROL_HEIGHT);
                let field = ui.add(
                    TextField::new(&mut state.query)
                        .placeholder("Find")
                        .monospace(true)
                        .width(140.0),
                );
                state.field = Some(field.id);
                ui.ctx()
                    .accesskit_node_builder(field.id, |node| node.set_label("Find"));
                if field.has_focus() {
                    // Escape closes the bar instead of dropping the focus.
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            field.id,
                            EventFilter {
                                escape: true,
                                ..EventFilter::default()
                            },
                        );
                    });
                }
                if std::mem::take(&mut state.focus) {
                    field.request_focus();
                }
                if field.changed() {
                    events.query_changed = true;
                    state.current = 0;
                    state.jump = true;
                }
                // Enter in the field steps; the single-line edit gives up focus on Enter.
                if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    events.step = if ui.input(|i| i.modifiers.shift) {
                        -1
                    } else {
                        1
                    };
                    field.request_focus();
                }
                ui.add_sized(
                    [64.0, size::CONTROL_HEIGHT],
                    egui::Label::new(
                        RichText::new(status.clone())
                            .font(typography::body(typography::SMALL + 1.0))
                            .color(colors.text_secondary),
                    )
                    .truncate(),
                );
                if ui
                    .add(IconButton::new(Glyph::ChevronUp, "Previous match").small(true))
                    .clicked()
                {
                    events.step = -1;
                }
                if ui
                    .add(IconButton::new(Glyph::ChevronDown, "Next match").small(true))
                    .clicked()
                {
                    events.step = 1;
                }
                if ui
                    .add(IconButton::new(Glyph::Close, "Close find").small(true))
                    .clicked()
                {
                    state.open = false;
                }
            });
        });
    if events.step != 0 {
        state.step(events.step, count);
    }
    events
}
