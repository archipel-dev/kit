//! The layer stack: popups, dialogs, tooltips and balloons above the app.
//!
//! [`Layers`] holds two groups. The *stack* is what the user is in right now (a popup menu, a
//! dialog); the top layer is modal and gets every key and mouse event first, and an
//! unhandled <kbd>Esc</kbd> unwinds it ("Esc closes the top layer first"). The *ambient*
//! layers (notification balloons) are always drawn on top, only take the events that hit
//! them, and are skipped by <kbd>Esc</kbd> unless they have focus.
//!
//! A layer reports what happened as messages of the app's own type `M`; the app
//! decides what to do with them, as an Elm update loop does.

use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crate::runtime::Runtime;
use archipel_kit::action_system::{KeymapUtil, Shortcut};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// What a layer did with an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handled<M> {
    /// The event was used: nothing below sees it.
    pub consumed: bool,
    /// The layer is done and should be removed.
    pub close: bool,
    /// Messages for the app.
    pub messages: Vec<M>,
}

impl<M> Handled<M> {
    /// The event is not for this layer.
    pub fn ignored() -> Self {
        Self {
            consumed: false,
            close: false,
            messages: Vec::new(),
        }
    }

    /// The layer used the event.
    pub fn consumed() -> Self {
        Self {
            consumed: true,
            ..Self::ignored()
        }
    }

    /// The layer used the event and is done.
    pub fn closed() -> Self {
        Self {
            consumed: true,
            close: true,
            messages: Vec::new(),
        }
    }

    /// Adds a message.
    #[must_use]
    pub fn with(mut self, message: M) -> Self {
        self.messages.push(message);
        self
    }

    /// Adds messages.
    #[must_use]
    pub fn with_all(mut self, messages: impl IntoIterator<Item = M>) -> Self {
        self.messages.extend(messages);
        self
    }
}

/// A layer above the app.
pub trait Overlay<M> {
    /// Applies `event` (a key, a mouse event or a paste).
    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<M>;

    /// Draws the layer. A modal layer's hit rectangles go above everything drawn before.
    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer);

    /// Runs timers; called every loop turn.
    fn tick(&mut self, rt: &mut Runtime) -> Vec<M> {
        let _ = rt;
        Vec::new()
    }

    /// Whether the layer blocks input and focus of what is below (default: yes).
    fn modal(&self) -> bool {
        true
    }

    /// The next time [`tick`](Self::tick) has something to do.
    fn deadline(&self) -> Option<f64> {
        None
    }

    /// Whether the layer is done on its own (a popup that closed itself).
    fn finished(&self) -> bool {
        false
    }
}

struct Entry<M> {
    layer: Box<dyn Overlay<M>>,
    saved_focus: Option<Id>,
}

/// The stack of layers.
pub struct Layers<M> {
    stack: Vec<Entry<M>>,
    ambient: Vec<Box<dyn Overlay<M>>>,
}

impl<M> Default for Layers<M> {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            ambient: Vec::new(),
        }
    }
}

impl<M> Layers<M> {
    /// An empty stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a layer on top of the stack; the focus is saved and restored when it goes.
    pub fn push(&mut self, rt: &Runtime, layer: impl Overlay<M> + 'static) {
        self.stack.push(Entry {
            layer: Box::new(layer),
            saved_focus: rt.focus.current(),
        });
    }

    /// Adds an ambient layer (always drawn on top, only takes events that hit it).
    pub fn push_ambient(&mut self, layer: impl Overlay<M> + 'static) {
        self.ambient.push(Box::new(layer));
    }

    /// The number of layers in the stack (ambient layers not counted).
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Whether the stack is empty: the app has the input.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Removes the top layer.
    pub fn pop(&mut self, rt: &mut Runtime) {
        if let Some(entry) = self.stack.pop()
            && let Some(id) = entry.saved_focus
        {
            rt.focus.focus(id);
        }
    }

    /// Delivers `event`. Returns whether a layer consumed it and the messages produced.
    pub fn event(&mut self, rt: &mut Runtime, event: &Event) -> (bool, Vec<M>) {
        let mut messages = Vec::new();
        for i in (0..self.ambient.len()).rev() {
            let handled = self.ambient[i].event(rt, event);
            messages.extend(handled.messages);
            if handled.close {
                self.ambient.remove(i);
            }
            if handled.consumed {
                return (true, messages);
            }
        }
        let mut consumed = false;
        if let Some(top) = self.stack.last_mut() {
            let handled = top.layer.event(rt, event);
            messages.extend(handled.messages);
            let modal = top.layer.modal();
            let finished = handled.close || top.layer.finished();
            consumed = handled.consumed;
            if finished {
                self.pop(rt);
            } else if !consumed && modal {
                // Esc unwinds the top layer; other keys and clicks stop here.
                if is_escape(event) {
                    self.pop(rt);
                    consumed = true;
                } else {
                    consumed = matches!(event, Event::Key(_) | Event::Mouse(_) | Event::Paste(_));
                }
            }
        }
        (consumed, messages)
    }

    /// Runs the timers of all layers, and removes the ones that finished.
    pub fn tick(&mut self, rt: &mut Runtime) -> Vec<M> {
        let mut messages = Vec::new();
        for entry in &mut self.stack {
            messages.extend(entry.layer.tick(rt));
        }
        for layer in &mut self.ambient {
            messages.extend(layer.tick(rt));
        }
        while self.stack.last().is_some_and(|e| e.layer.finished()) {
            self.pop(rt);
        }
        self.ambient.retain(|l| !l.finished());
        messages
    }

    /// The next time any layer needs a tick.
    pub fn deadline(&self) -> Option<f64> {
        self.stack
            .iter()
            .map(|e| e.layer.deadline())
            .chain(self.ambient.iter().map(|l| l.deadline()))
            .flatten()
            .reduce(f64::min)
    }

    /// Draws the stack bottom to top, then the ambient layers.
    pub fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        for entry in &mut self.stack {
            if entry.layer.modal() {
                rt.hits.block(area);
                rt.focus.trap();
            }
            entry.layer.draw(rt, area, buf);
        }
        for layer in &mut self.ambient {
            layer.draw(rt, area, buf);
        }
    }
}

/// Draws the key hints of a pending chord at the bottom of `area` (Neovim's which-key): the
/// second strokes that complete a chord that starts with the stroke just pressed, and the
/// action each one runs (`label_of` turns an action id into text).
pub fn draw_which_key(
    rt: &Runtime,
    area: Rect,
    buf: &mut Buffer,
    label_of: &dyn Fn(&str) -> String,
) {
    if !rt.pending_active() {
        return;
    }
    let Some(pending) = rt.pending_stroke() else {
        return;
    };
    let mac = rt.caps.os.is_mac();
    let mut entries: Vec<(String, String)> = Vec::new();
    for id in rt.keymap.action_ids() {
        for shortcut in rt.keymap.shortcuts(&id) {
            if let Shortcut::Keyboard(k) = shortcut
                && k.first == pending.first
                && let Some(second) = k.second
            {
                entries.push((KeymapUtil::keystroke_text(&second, mac), label_of(&id)));
            }
        }
    }
    if entries.is_empty() {
        return;
    }
    entries.sort();
    entries.dedup();
    let p = rt.look.palette;
    let col_w = entries
        .iter()
        .map(|(k, l)| crate::components::text_width(k) + crate::components::text_width(l) + 6)
        .max()
        .unwrap_or(10)
        .min(area.width.max(1));
    let cols = (area.width / col_w.max(1)).max(1) as usize;
    let rows = entries.len().div_ceil(cols) as u16;
    let height = (rows + 1).min(area.height);
    if height < 2 {
        return;
    }
    // Above the last row (the status line).
    let y = area.bottom().saturating_sub(height + 1).max(area.y);
    let rect = Rect::new(area.x, y, area.width, height.min(area.height));
    crate::components::fill_rect(buf, rect, Style::default().bg(p.one_bg).fg(p.white));
    let first = KeymapUtil::keystroke_text(&pending.first, mac);
    crate::components::title_block(
        buf,
        rect,
        rect.x + 1,
        rect.y,
        &first,
        p.on(p.nord_blue),
        p.nord_blue,
    );
    for (i, (key, label)) in entries.iter().enumerate() {
        let (c, r) = (i % cols, i / cols);
        let x = rect.x + 1 + c as u16 * col_w;
        let cy = rect.y + 1 + r as u16;
        if cy >= rect.bottom() {
            break;
        }
        let xx = crate::components::put(
            buf,
            rect,
            x,
            cy,
            key,
            Style::default()
                .fg(p.nord_blue)
                .bg(p.one_bg)
                .add_modifier(Modifier::BOLD),
        );
        let xx = crate::components::put(
            buf,
            rect,
            xx,
            cy,
            " → ",
            Style::default().fg(p.grey_fg).bg(p.one_bg),
        );
        crate::components::put(
            buf,
            rect,
            xx,
            cy,
            label,
            Style::default().fg(p.white).bg(p.one_bg),
        );
    }
}

/// Whether `event` is a key press of Esc with no modifiers.
pub fn is_escape(event: &Event) -> bool {
    matches!(event, Event::Key(k) if k.code == KeyCode::Esc && k.modifiers == KeyModifiers::NONE && k.kind != KeyEventKind::Release)
}

/// A popup as a layer: opens it above the app and reports how it ended.
pub struct PopupOverlay<S: crate::popup::PopupSource, M> {
    /// The popup.
    pub host: crate::popup::PopupHost<S>,
    on_done: DoneFn<S, M>,
}

type DoneFn<S, M> = Box<dyn FnMut(&mut S, crate::popup::PopupState) -> Vec<M>>;

impl<S: crate::popup::PopupSource, M> PopupOverlay<S, M> {
    /// Wraps an opened `host`; `on_done` turns the end of the popup into messages.
    pub fn new(
        host: crate::popup::PopupHost<S>,
        on_done: impl FnMut(&mut S, crate::popup::PopupState) -> Vec<M> + 'static,
    ) -> Self {
        Self {
            host,
            on_done: Box::new(on_done),
        }
    }
}

impl<S: crate::popup::PopupSource, M> Overlay<M> for PopupOverlay<S, M> {
    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<M> {
        let state = self.host.event(rt, event);
        if state == crate::popup::PopupState::Open {
            return Handled::consumed();
        }
        let messages = (self.on_done)(&mut self.host.source, state);
        Handled::closed().with_all(messages)
    }

    fn tick(&mut self, rt: &mut Runtime) -> Vec<M> {
        self.host.tick(rt);
        Vec::new()
    }

    fn deadline(&self) -> Option<f64> {
        self.host.deadline()
    }

    fn finished(&self) -> bool {
        !self.host.is_open()
    }

    fn draw(&mut self, rt: &mut Runtime, _area: Rect, buf: &mut Buffer) {
        self.host.draw(rt, buf);
    }
}
