//! Behaviours: interaction added to any widget with `.with(...)`.
//!
//! A component draws itself and reports a [`Response`]; a [`Behaviour`] takes that response
//! and adds something on top: a tooltip, a context menu, a cursor, dragging, dropping. The
//! same behaviour works on every component, the kit's and your own, so a tooltip or a
//! context menu looks and reacts the same everywhere.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use archipel_kit::behaviour::{ContextMenu, WidgetExt};
//! use archipel_kit::components::{IconButton, MenuItem, Tooltip};
//! use archipel_kit::Glyph;
//!
//! ui.add(
//!     IconButton::new(Glyph::Run, "Run")
//!         .with(Tooltip::new("Run 'app'").shortcut("Shift+F10"))
//!         .with(ContextMenu::new(|ui| {
//!             ui.add(MenuItem::new("Edit Configuration…"));
//!         })),
//! );
//! # });
//! ```
//!
//! Components with a `show` method (rows, tab bars) take behaviours with their own `with`.
//! Write a behaviour as a type implementing [`Behaviour`], or as a closure
//! `|ui, response| { …; response }`.

use crate::components::Tooltip;
use crate::parts::Surface;
use crate::theme::Theme;
use crate::tokens::radius;
use egui::{CornerRadius, CursorIcon, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use std::any::Any;
use std::cell::Cell;

/// Interaction added to a widget after it has drawn itself: see the [module docs](self).
pub trait Behaviour {
    /// Adds the behaviour to `response`, the widget's response this frame, and returns it
    /// (possibly with more sense, e.g. dragging).
    fn attach(self, ui: &mut Ui, response: Response) -> Response;

    /// What the widget must sense for this behaviour to work (default: nothing extra).
    ///
    /// A component that takes behaviours asks for it ([`Behaviours::sense`]) *before*
    /// allocating its response and adds it to its own sense, so the widget is registered
    /// once, in its place in the focus order. A behaviour that needs more sense than the
    /// widget was allocated with falls back to [`Response::interact`], which registers the
    /// widget a second time and moves it after widgets drawn in between (the audit then
    /// reports a broken Tab order on rows with a chevron).
    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    /// Whether this behaviour shows a tooltip, so a component with a tooltip of its own
    /// (an elided label) leaves it to the caller's.
    fn is_tooltip(&self) -> bool {
        false
    }
}

impl<F: FnOnce(&mut Ui, Response) -> Response> Behaviour for F {
    fn attach(self, ui: &mut Ui, response: Response) -> Response {
        self(ui, response)
    }
}

/// A [`Behaviour`] stored by a component until it has drawn (object-safe form).
pub trait BoxedBehaviour {
    /// [`Behaviour::attach`] on a boxed behaviour.
    fn attach_boxed(self: Box<Self>, ui: &mut Ui, response: Response) -> Response;
    /// [`Behaviour::sense`] on a boxed behaviour.
    fn sense(&self) -> Sense;
    /// [`Behaviour::is_tooltip`] on a boxed behaviour.
    fn is_tooltip(&self) -> bool;
}

impl<B: Behaviour> BoxedBehaviour for B {
    fn attach_boxed(self: Box<Self>, ui: &mut Ui, response: Response) -> Response {
        (*self).attach(ui, response)
    }

    fn sense(&self) -> Sense {
        Behaviour::sense(self)
    }

    fn is_tooltip(&self) -> bool {
        Behaviour::is_tooltip(self)
    }
}

/// The behaviours a component collects with its `with` method, attached in order.
#[derive(Default)]
pub struct Behaviours<'a>(Vec<Box<dyn BoxedBehaviour + 'a>>);

impl<'a> Behaviours<'a> {
    /// Adds `behaviour`.
    pub fn push(&mut self, behaviour: impl Behaviour + 'a) {
        self.0.push(Box::new(behaviour));
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// What the behaviours need the widget to sense: the union of [`Behaviour::sense`]. Add
    /// it to the component's own sense before allocating the response.
    pub fn sense(&self) -> Sense {
        self.0.iter().fold(Sense::HOVER, |all, b| all | b.sense())
    }

    /// Whether one of the behaviours is a tooltip.
    pub fn has_tooltip(&self) -> bool {
        self.0.iter().any(|b| b.is_tooltip())
    }

    /// Attaches every behaviour to `response`, in the order they were added.
    pub fn attach(self, ui: &mut Ui, response: Response) -> Response {
        self.0.into_iter().fold(response, |response, behaviour| {
            behaviour.attach_boxed(ui, response)
        })
    }
}

/// `.with(behaviour)` on every [`Widget`].
pub trait WidgetExt: Widget + Sized {
    /// Draws the widget, then attaches `behaviour` to its response. Chain it to add more.
    fn with<B: Behaviour>(self, behaviour: B) -> With<Self, B> {
        With {
            widget: self,
            behaviour,
        }
    }
}

impl<W: Widget> WidgetExt for W {}

/// A widget with a behaviour: see [`WidgetExt::with`].
#[must_use = "add it with `ui.add(widget)`"]
pub struct With<W, B> {
    widget: W,
    behaviour: B,
}

impl<W: Widget, B: Behaviour> Widget for With<W, B> {
    fn ui(self, ui: &mut Ui) -> Response {
        let response = self.widget.ui(ui);
        self.behaviour.attach(ui, response)
    }
}

impl Behaviour for Tooltip {
    fn attach(self, _ui: &mut Ui, response: Response) -> Response {
        self.show_for(&response);
        response
    }

    fn is_tooltip(&self) -> bool {
        true
    }
}

/// A context menu on secondary click (right click, or Ctrl+click on macOS), on the popup
/// surface. Fill it with [`MenuItem`](crate::components::MenuItem)s; a click on an item
/// closes it.
pub struct ContextMenu<'a> {
    add: Box<dyn FnOnce(&mut Ui) + 'a>,
}

impl<'a> ContextMenu<'a> {
    /// A context menu drawn by `add`.
    pub fn new(add: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self { add: Box::new(add) }
    }
}

impl Behaviour for ContextMenu<'_> {
    fn attach(self, _ui: &mut Ui, response: Response) -> Response {
        let frame = Surface::Popup.frame(&response.ctx);
        egui::Popup::context_menu(&response)
            .frame(frame)
            .show(|ui| (self.add)(ui));
        response
    }
}

/// The mouse cursor over the widget, instead of the one the component picks. Use it for a
/// widget that does something special: a splitter, a drag handle, a busy control.
#[derive(Debug, Clone, Copy)]
pub struct Cursor(pub CursorIcon);

impl Behaviour for Cursor {
    fn attach(self, _ui: &mut Ui, response: Response) -> Response {
        crate::cursor::on_hover(&response, self.0);
        response
    }
}

/// Makes the widget draggable, carrying `payload`. The cursor is the open hand on hover
/// and the closed hand while dragging; pair it with a [`DropTarget`] for the same payload
/// type. A plain click still clicks.
///
/// Components with a `with` method (rows, tabs) allocate with [`Behaviour::sense`] up front,
/// so a draggable row keeps its place in the Tab order.
#[derive(Debug, Clone)]
pub struct DragSource<P>(pub P);

impl<P: Any + Send + Sync> Behaviour for DragSource<P> {
    fn sense(&self) -> Sense {
        Sense::DRAG
    }

    fn attach(self, _ui: &mut Ui, response: Response) -> Response {
        if !response.enabled() {
            return response;
        }
        // Components that take behaviours allocate with `sense()` up front. A plain widget
        // wrapped with `.with(...)` did not: register it again as a last resort.
        let response = if response.sense.senses_drag() {
            response
        } else {
            response.interact(Sense::drag())
        };
        if response.dragged() {
            response.dnd_set_drag_payload(self.0);
            response.ctx.set_cursor_icon(CursorIcon::Grabbing);
        } else {
            crate::cursor::on_hover(&response, CursorIcon::Grab);
        }
        response
    }
}

/// Accepts a dragged payload of type `P` (from a [`DragSource`]): outlines the widget while
/// a payload of that type hovers it, and stores the payload in `dropped` when it is
/// released over it.
pub struct DropTarget<'a, P> {
    dropped: Dropped<'a, P>,
}

enum Dropped<'a, P> {
    Mut(&'a mut Option<P>),
    Shared(&'a Cell<Option<P>>),
}

impl<'a, P> DropTarget<'a, P> {
    /// A drop target writing a released payload to `dropped`.
    pub fn new(dropped: &'a mut Option<P>) -> Self {
        Self {
            dropped: Dropped::Mut(dropped),
        }
    }

    /// A drop target writing to a [`Cell`], for many targets sharing one slot: the rows of a
    /// tree, drawn by a closure that cannot hand out `&mut` to each of them.
    pub fn shared(dropped: &'a Cell<Option<P>>) -> Self {
        Self {
            dropped: Dropped::Shared(dropped),
        }
    }
}

impl<P: Any + Send + Sync + Clone> Behaviour for DropTarget<'_, P> {
    fn attach(self, ui: &mut Ui, response: Response) -> Response {
        if response.dnd_hover_payload::<P>().is_some() {
            let colors = Theme::colors(ui.ctx());
            ui.painter().rect_stroke(
                response.rect,
                CornerRadius::same(radius::ROW),
                Stroke::new(1.5_f32, colors.focus_ring),
                StrokeKind::Inside,
            );
        }
        if let Some(payload) = response.dnd_release_payload::<P>() {
            let payload = Some((*payload).clone());
            match self.dropped {
                Dropped::Mut(dropped) => *dropped = payload,
                Dropped::Shared(dropped) => dropped.set(payload),
            }
        }
        response
    }
}
