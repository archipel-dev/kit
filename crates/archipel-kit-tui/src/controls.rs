//! Interactive controls: buttons, check boxes, radio buttons, switches, segmented buttons,
//! text and search fields, and inline banners.
//!
//! A control is a small value that describes it for one frame (`id`, label, flags) with two
//! methods: `show` draws it and registers it with the [`Runtime`] (hit map, focus order, tooltip),
//! and `handle` applies an [`Event`] to the state the caller owns and says what happened.
//! The state types are plain data, as in the GUI kit: the caller keeps them.

use crate::components::{
    Button, InlineBanner, JBCheckBox, JBRadioButton, JBTextField, OnOffButton, RowIcon,
    SegmentedButton, choice_width, field_scroll, text_width,
};
use crate::hit::Id;
use crate::input::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use crate::tooltip::Tooltip;
use archipel_kit::ui::{ButtonVariant, Status};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

fn plain_press(event: &Event) -> Option<&KeyEvent> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => Some(key),
        _ => None,
    }
}

/// Whether `event` activates a focused control: Space or Enter without Ctrl or Alt.
fn is_activation(key: &KeyEvent, enter: bool) -> bool {
    input::is_plain(key, KeyCode::Char(' ')) || (enter && input::is_plain(key, KeyCode::Enter))
}

/// The state of a [`ButtonControl`]: whether the mouse is down on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ButtonState {
    /// The left button is down on the button and the pointer is still over it.
    pub pressed: bool,
}

/// A push button. Enter or Space on the focused button, a click, or Alt plus its
/// mnemonic key press it; a click completes on release inside, and dragging off cancels.
#[derive(Debug, Clone)]
pub struct ButtonControl<'a> {
    /// The identity of the button.
    pub id: Id,
    /// The label.
    pub label: &'a str,
    /// The variant: the default button of a dialog is `Primary`.
    pub variant: ButtonVariant,
    /// An icon before the label.
    pub icon: Option<crate::Glyph>,
    /// The mnemonic: underlined in the label, Alt plus the key presses the button.
    pub key: Option<char>,
    /// Disabled buttons ignore input.
    pub enabled: bool,
    /// The background the button sits on.
    pub on: Option<Color>,
    /// The tooltip, if any.
    pub tooltip: Option<Tooltip>,
}

impl<'a> ButtonControl<'a> {
    /// A secondary button.
    pub fn new(id: Id, label: &'a str) -> Self {
        Self {
            id,
            label,
            variant: ButtonVariant::Secondary,
            icon: None,
            key: None,
            enabled: true,
            on: None,
            tooltip: None,
        }
    }

    /// Makes it the primary (default) button.
    #[must_use]
    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    /// Sets the variant.
    #[must_use]
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the icon.
    #[must_use]
    pub fn icon(mut self, icon: crate::Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the mnemonic.
    #[must_use]
    pub fn key(mut self, key: char) -> Self {
        self.key = Some(key);
        self
    }

    /// Enables or disables it.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the background it sits on.
    #[must_use]
    pub fn on(mut self, color: Color) -> Self {
        self.on = Some(color);
        self
    }

    /// Sets the tooltip.
    #[must_use]
    pub fn tooltip(mut self, tooltip: Tooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    fn widget<'b>(&'b self, rt: &'b Runtime, state: &ButtonState) -> Button<'b> {
        let mut button = Button::new(&rt.look, self.label, self.variant)
            .enabled(self.enabled)
            .focused(rt.focused(self.id))
            .pressed(state.pressed);
        if let Some(icon) = self.icon {
            button = button.icon(icon);
        }
        if let Some(key) = self.key {
            button = button.key(key);
        }
        if let Some(on) = self.on {
            button = button.on(on);
        }
        button
    }

    /// The width in cells.
    pub fn width(&self, rt: &Runtime) -> u16 {
        self.widget(rt, &ButtonState::default()).width()
    }

    /// Draws the button at (`x`, `y`) and registers it; returns its rectangle.
    pub fn show(
        &self,
        rt: &mut Runtime,
        state: &ButtonState,
        x: u16,
        y: u16,
        buf: &mut Buffer,
    ) -> Rect {
        let width = self.width(rt);
        let rect = Rect::new(x, y, width, 1);
        let area = rect.intersection(buf.area);
        if area.is_empty() {
            return rect;
        }
        self.widget(rt, state).render(rect, buf);
        if self.enabled {
            rt.widget(self.id, rect);
            if let Some(tip) = &self.tooltip {
                rt.tooltip.offer(self.id, rect, tip.clone());
            }
        }
        rect
    }

    /// Applies `event`; returns true when the button was pressed (clicked or activated).
    pub fn handle(&self, rt: &Runtime, state: &mut ButtonState, event: &Event) -> bool {
        if !self.enabled {
            state.pressed = false;
            return false;
        }
        if let Some(key) = plain_press(event) {
            if rt.focused(self.id) && is_activation(key, true) {
                return true;
            }
            if let (Some(mnemonic), KeyCode::Char(c)) = (self.key, key.code)
                && key.modifiers.contains(KeyModifiers::ALT)
                && c.eq_ignore_ascii_case(&mnemonic)
            {
                return true;
            }
            return false;
        }
        let Event::Mouse(_) = event else {
            return false;
        };
        let Some(p) = rt.pointer() else {
            return false;
        };
        match p.kind {
            PointerKind::Down(crate::input::MouseButton::Left) if p.over(self.id) => {
                state.pressed = true;
            }
            PointerKind::Drag(crate::input::MouseButton::Left) => {
                state.pressed = p.origin == Some(self.id) && p.over(self.id);
            }
            PointerKind::Up(crate::input::MouseButton::Left) => {
                let hit = p.clicked(self.id);
                state.pressed = false;
                return hit;
            }
            _ => {}
        }
        false
    }
}

/// A three-state check box value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckState {
    /// Off.
    #[default]
    Unchecked,
    /// On.
    Checked,
    /// Some, not all, children are on.
    Mixed,
}

impl CheckState {
    /// The state after a click: unchecked, checked, and for a tri-state box mixed, in turn.
    pub fn next(self, tri_state: bool) -> Self {
        match (self, tri_state) {
            (CheckState::Unchecked, _) => CheckState::Checked,
            (CheckState::Checked, true) => CheckState::Mixed,
            (CheckState::Checked, false) | (CheckState::Mixed, _) => CheckState::Unchecked,
        }
    }

    /// Whether it is checked.
    pub fn is_checked(self) -> bool {
        self == CheckState::Checked
    }
}

impl From<bool> for CheckState {
    fn from(on: bool) -> Self {
        if on {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        }
    }
}

/// A check box: Space toggles it when focused, a click toggles on release inside.
#[derive(Debug, Clone)]
pub struct CheckBoxControl<'a> {
    /// The identity.
    pub id: Id,
    /// The label.
    pub label: &'a str,
    /// Disabled boxes ignore input.
    pub enabled: bool,
    /// Whether a click cycles through the mixed state too.
    pub tri_state: bool,
    /// The background it sits on.
    pub on: Option<Color>,
}

impl<'a> CheckBoxControl<'a> {
    /// An enabled two-state check box.
    pub fn new(id: Id, label: &'a str) -> Self {
        Self {
            id,
            label,
            enabled: true,
            tri_state: false,
            on: None,
        }
    }

    /// Enables or disables it.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Makes it tri-state.
    #[must_use]
    pub fn tri_state(mut self, tri: bool) -> Self {
        self.tri_state = tri;
        self
    }

    /// Sets the background it sits on.
    #[must_use]
    pub fn on(mut self, color: Color) -> Self {
        self.on = Some(color);
        self
    }

    /// Draws it at (`x`, `y`) and registers it; returns its rectangle.
    pub fn show(
        &self,
        rt: &mut Runtime,
        state: CheckState,
        x: u16,
        y: u16,
        buf: &mut Buffer,
    ) -> Rect {
        let rect = Rect::new(x, y, choice_width(self.label), 1);
        let mut widget = JBCheckBox::new(&rt.look, state.is_checked(), self.label)
            .mixed(state == CheckState::Mixed)
            .enabled(self.enabled)
            .focused(rt.focused(self.id));
        if let Some(on) = self.on {
            widget = widget.on(on);
        }
        widget.render(rect, buf);
        if self.enabled {
            rt.widget(self.id, rect);
        }
        rect
    }

    /// Applies `event`; returns true when the state changed.
    pub fn handle(&self, rt: &Runtime, state: &mut CheckState, event: &Event) -> bool {
        if !self.enabled || !activated(rt, self.id, event) {
            return false;
        }
        *state = state.next(self.tri_state);
        true
    }
}

/// Whether `event` activates the control `id`: Space when it has focus, or a click on it.
fn activated(rt: &Runtime, id: Id, event: &Event) -> bool {
    match event {
        Event::Key(key) => {
            plain_press(event).is_some() && rt.focused(id) && is_activation(key, false)
        }
        Event::Mouse(_) => rt.pointer().is_some_and(|p| p.clicked(id)),
        _ => false,
    }
}

/// A switch: Space or a click toggles it.
#[derive(Debug, Clone)]
pub struct OnOffControl<'a> {
    /// The identity.
    pub id: Id,
    /// The label.
    pub label: &'a str,
    /// The background it sits on.
    pub on: Option<Color>,
}

impl<'a> OnOffControl<'a> {
    /// A switch.
    pub fn new(id: Id, label: &'a str) -> Self {
        Self {
            id,
            label,
            on: None,
        }
    }

    /// Draws it at (`x`, `y`) and registers it.
    pub fn show(&self, rt: &mut Runtime, value: bool, x: u16, y: u16, buf: &mut Buffer) -> Rect {
        let rect = Rect::new(x, y, choice_width(self.label) + 1, 1);
        let mut widget = OnOffButton::new(&rt.look, value, self.label).focused(rt.focused(self.id));
        if let Some(on) = self.on {
            widget = widget.on(on);
        }
        widget.render(rect, buf);
        rt.widget(self.id, rect);
        rect
    }

    /// Applies `event`; returns true when the switch was toggled (the caller flips its bool).
    pub fn handle(&self, rt: &Runtime, event: &Event) -> bool {
        activated(rt, self.id, event)
    }
}

/// A group of radio buttons: one focus stop for the group; Up/Down/Left/Right move the
/// selection, Space or a click selects.
#[derive(Debug, Clone)]
pub struct RadioGroupControl<'a> {
    /// The identity of the group; options use child ids.
    pub id: Id,
    /// The option labels.
    pub options: &'a [&'a str],
    /// The background it sits on.
    pub on: Option<Color>,
}

impl<'a> RadioGroupControl<'a> {
    /// A group of `options`.
    pub fn new(id: Id, options: &'a [&'a str]) -> Self {
        Self {
            id,
            options,
            on: None,
        }
    }

    /// Draws the options one per row from (`x`, `y`) and registers them; the group takes
    /// one focus stop.
    pub fn show(&self, rt: &mut Runtime, selected: usize, x: u16, y: u16, buf: &mut Buffer) {
        let focused = rt.focused(self.id);
        rt.focus.register(self.id);
        for (i, label) in self.options.iter().enumerate() {
            let rect = Rect::new(x, y + i as u16, choice_width(label), 1);
            let mut widget = JBRadioButton::new(&rt.look, i == selected, label)
                .focused(focused && i == selected);
            if let Some(on) = self.on {
                widget = widget.on(on);
            }
            widget.render(rect, buf);
            rt.hits.add(rect, self.id.with(i as u64));
        }
    }

    /// Applies `event`; returns the newly selected option.
    pub fn handle(&self, rt: &mut Runtime, selected: &mut usize, event: &Event) -> bool {
        let len = self.options.len();
        if len == 0 {
            return false;
        }
        if let Some(key) = plain_press(event) {
            if !rt.focused(self.id) {
                return false;
            }
            let next = match key.code {
                KeyCode::Up | KeyCode::Left => (*selected + len - 1) % len,
                KeyCode::Down | KeyCode::Right => (*selected + 1) % len,
                _ => return false,
            };
            *selected = next;
            return true;
        }
        if let Event::Mouse(_) = event
            && let Some(p) = rt.pointer()
        {
            for i in 0..len {
                if p.clicked(self.id.with(i as u64)) {
                    let changed = *selected != i;
                    *selected = i;
                    rt.focus.focus(self.id);
                    return changed;
                }
            }
        }
        false
    }
}

/// A segmented button: Left/Right move the selection when focused, a click selects.
#[derive(Debug, Clone)]
pub struct SegmentedControl<'a> {
    /// The identity; segments use child ids.
    pub id: Id,
    /// The segment labels.
    pub options: &'a [&'a str],
    /// The background it sits on.
    pub on: Option<Color>,
}

impl<'a> SegmentedControl<'a> {
    /// A control of `options`.
    pub fn new(id: Id, options: &'a [&'a str]) -> Self {
        Self {
            id,
            options,
            on: None,
        }
    }

    /// Draws the control at (`x`, `y`) and registers it; returns the width.
    pub fn show(&self, rt: &mut Runtime, selected: usize, x: u16, y: u16, buf: &mut Buffer) -> u16 {
        let focused = rt.focused(self.id);
        rt.focus.register(self.id);
        let mut widget = SegmentedButton::new(&rt.look, self.options, selected).focused(focused);
        if let Some(on) = self.on {
            widget = widget.on(on);
        }
        let area = Rect::new(x, y, buf.area.width.saturating_sub(x), 1);
        let rects = widget.option_rects(area);
        let width = rects.last().map_or(0, |r| r.right() - x);
        widget.render(area, buf);
        for (i, rect) in rects.iter().enumerate() {
            rt.hits.add(*rect, self.id.with(i as u64));
        }
        width
    }

    /// Applies `event`; returns true when the selection changed.
    pub fn handle(&self, rt: &mut Runtime, selected: &mut usize, event: &Event) -> bool {
        let len = self.options.len();
        if len == 0 {
            return false;
        }
        if let Some(key) = plain_press(event) {
            if !rt.focused(self.id) {
                return false;
            }
            let next = match key.code {
                KeyCode::Left => selected.saturating_sub(1),
                KeyCode::Right => (*selected + 1).min(len - 1),
                KeyCode::Home => 0,
                KeyCode::End => len - 1,
                _ => return false,
            };
            let changed = next != *selected;
            *selected = next;
            return changed;
        }
        if let Event::Mouse(_) = event
            && let Some(p) = rt.pointer()
        {
            for i in 0..len {
                if p.clicked(self.id.with(i as u64)) {
                    let changed = *selected != i;
                    *selected = i;
                    rt.focus.focus(self.id);
                    return changed;
                }
            }
        }
        false
    }
}

/// What a text field did with an event.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldEvent {
    /// The event is not for the field.
    Ignored,
    /// The field used the event without changing its text (movement, selection, focus).
    Consumed,
    /// The text changed.
    Changed,
    /// Enter was pressed.
    Submitted,
    /// Escape was pressed in a field with nothing to clear: the surrounding dialog or popup
    /// should handle it.
    Cancelled,
    /// A search field was cleared (Escape or the clear button).
    Cleared,
    /// The search history popup was requested (`ShowSearchHistory`).
    ShowHistory,
}

/// The editing state of a single-line text field: text, caret, selection, undo.
///
/// Positions are character indices. The state works without a terminal: feed it keys with
/// [`TextFieldState::apply_key`] and read [`text`](Self::text).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextFieldState {
    text: String,
    caret: usize,
    anchor: Option<usize>,
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
    drag_origin: Option<usize>,
    /// Bumped on every text change (a dialog validates when it changes).
    pub revision: u64,
}

impl TextFieldState {
    /// An empty field.
    pub fn new() -> Self {
        Self::default()
    }

    /// A field holding `text`, caret at the end.
    pub fn with_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let caret = text.chars().count();
        Self {
            text,
            caret,
            ..Self::default()
        }
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The caret as a character index.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selected character range, if any.
    pub fn selection(&self) -> Option<std::ops::Range<usize>> {
        let anchor = self.anchor?;
        (anchor != self.caret).then(|| anchor.min(self.caret)..anchor.max(self.caret))
    }

    /// The selected text.
    pub fn selected_text(&self) -> String {
        self.selection()
            .map(|r| self.text.chars().skip(r.start).take(r.len()).collect())
            .unwrap_or_default()
    }

    /// The number of characters.
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Replaces the text (caret to the end, selection and undo cleared).
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.caret = self.len();
        self.anchor = None;
        self.undo.clear();
        self.redo.clear();
        self.revision += 1;
    }

    fn byte(&self, index: usize) -> usize {
        self.text
            .char_indices()
            .nth(index)
            .map_or(self.text.len(), |(b, _)| b)
    }

    fn remember(&mut self) {
        self.undo.push((self.text.clone(), self.caret));
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    /// Deletes the selection; returns whether there was one.
    fn delete_selection(&mut self) -> bool {
        let Some(range) = self.selection() else {
            self.anchor = None;
            return false;
        };
        self.remember();
        let (a, b) = (self.byte(range.start), self.byte(range.end));
        self.text.replace_range(a..b, "");
        self.caret = range.start;
        self.anchor = None;
        self.changed();
        true
    }

    /// Inserts `text` at the caret, replacing the selection. Newlines become spaces.
    pub fn insert(&mut self, text: &str) {
        let text: String = text
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .filter(|c| !c.is_control())
            .collect();
        if text.is_empty() {
            return;
        }
        self.remember();
        if let Some(range) = self.selection() {
            let (a, b) = (self.byte(range.start), self.byte(range.end));
            self.text.replace_range(a..b, "");
            self.caret = range.start;
        }
        self.anchor = None;
        let at = self.byte(self.caret);
        self.text.insert_str(at, &text);
        self.caret += text.chars().count();
        self.changed();
    }

    /// Moves the caret to `to`, extending the selection when `select`.
    pub fn move_to(&mut self, to: usize, select: bool) {
        let to = to.min(self.len());
        if select {
            self.anchor.get_or_insert(self.caret);
        } else {
            self.anchor = None;
        }
        self.caret = to;
    }

    /// Selects all text.
    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.caret = self.len();
    }

    fn word_start(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = from.min(chars.len());
        while i > 0 && !is_word(chars[i - 1]) {
            i -= 1;
        }
        while i > 0 && is_word(chars[i - 1]) {
            i -= 1;
        }
        i
    }

    fn word_end(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = from.min(chars.len());
        while i < chars.len() && !is_word(chars[i]) {
            i += 1;
        }
        while i < chars.len() && is_word(chars[i]) {
            i += 1;
        }
        i
    }

    /// Selects the word at `at`.
    pub fn select_word(&mut self, at: usize) {
        let chars: Vec<char> = self.text.chars().collect();
        let at = at.min(chars.len());
        let mut a = at;
        let mut b = at;
        if chars.get(at).copied().is_some_and(is_word) || (at > 0 && is_word(chars[at - 1])) {
            while a > 0 && is_word(chars[a - 1]) {
                a -= 1;
            }
            while b < chars.len() && is_word(chars[b]) {
                b += 1;
            }
        }
        self.anchor = Some(a);
        self.caret = b;
    }

    /// Applies the keymap action `id` (`$Copy`, `$Paste` …); returns whether it applied.
    pub fn apply_action(&mut self, id: &str, clipboard: &mut crate::clipboard::Clipboard) -> bool {
        match id {
            "$Copy" => {
                let selected = self.selected_text();
                if !selected.is_empty() {
                    clipboard.set(selected);
                }
                true
            }
            "$Cut" => {
                let selected = self.selected_text();
                if !selected.is_empty() {
                    clipboard.set(selected);
                    self.delete_selection();
                }
                true
            }
            "$Paste" => {
                let text = clipboard.get().to_owned();
                self.insert(&text);
                true
            }
            "$SelectAll" => {
                self.select_all();
                true
            }
            "$Undo" => {
                if let Some((text, caret)) = self.undo.pop() {
                    self.redo
                        .push((std::mem::replace(&mut self.text, text), self.caret));
                    self.caret = caret.min(self.len());
                    self.anchor = None;
                    self.changed();
                }
                true
            }
            "$Redo" => {
                if let Some((text, caret)) = self.redo.pop() {
                    self.undo
                        .push((std::mem::replace(&mut self.text, text), self.caret));
                    self.caret = caret.min(self.len());
                    self.anchor = None;
                    self.changed();
                }
                true
            }
            "EditorDeleteLine" => {
                if !self.text.is_empty() {
                    self.remember();
                    self.text.clear();
                    self.caret = 0;
                    self.anchor = None;
                    self.changed();
                }
                true
            }
            "EditorPreviousWord" => {
                self.move_to(self.word_start(self.caret), false);
                true
            }
            "EditorNextWord" => {
                self.move_to(self.word_end(self.caret), false);
                true
            }
            _ => false,
        }
    }

    /// Applies a key to the text: typing, deletion, caret movement and selection with
    /// Shift. Clipboard and undo go through [`apply_action`](Self::apply_action).
    pub fn apply_key(&mut self, key: &KeyEvent) -> FieldEvent {
        let shift = input::shift(key);
        let word = key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        let before = self.revision;
        let result = match key.code {
            KeyCode::Left => {
                if !shift && let Some(range) = self.selection() {
                    self.move_to(range.start, false);
                } else {
                    let to = if word {
                        self.word_start(self.caret)
                    } else {
                        self.caret.saturating_sub(1)
                    };
                    self.move_to(to, shift);
                }
                FieldEvent::Consumed
            }
            KeyCode::Right => {
                if !shift && let Some(range) = self.selection() {
                    self.move_to(range.end, false);
                } else {
                    let to = if word {
                        self.word_end(self.caret)
                    } else {
                        self.caret + 1
                    };
                    self.move_to(to, shift);
                }
                FieldEvent::Consumed
            }
            KeyCode::Home => {
                self.move_to(0, shift);
                FieldEvent::Consumed
            }
            KeyCode::End => {
                self.move_to(self.len(), shift);
                FieldEvent::Consumed
            }
            KeyCode::Backspace => {
                if !self.delete_selection() && self.caret > 0 {
                    self.remember();
                    let from = if word {
                        self.word_start(self.caret)
                    } else {
                        self.caret - 1
                    };
                    let (a, b) = (self.byte(from), self.byte(self.caret));
                    self.text.replace_range(a..b, "");
                    self.caret = from;
                    self.changed();
                }
                FieldEvent::Consumed
            }
            KeyCode::Delete => {
                if !self.delete_selection() && self.caret < self.len() {
                    self.remember();
                    let to = if word {
                        self.word_end(self.caret)
                    } else {
                        self.caret + 1
                    };
                    let (a, b) = (self.byte(self.caret), self.byte(to));
                    self.text.replace_range(a..b, "");
                    self.changed();
                }
                FieldEvent::Consumed
            }
            KeyCode::Enter => FieldEvent::Submitted,
            KeyCode::Char(_) => match input::typed_char(key) {
                Some(c) => {
                    self.insert(&c.to_string());
                    FieldEvent::Consumed
                }
                None => FieldEvent::Ignored,
            },
            _ => FieldEvent::Ignored,
        };
        if self.revision != before && result == FieldEvent::Consumed {
            FieldEvent::Changed
        } else {
            result
        }
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A single-line text field (Telescope prompt look): a label block or a search icon, the
/// text, a caret, a placeholder and an error line.
#[derive(Debug, Clone)]
pub struct TextFieldControl<'a> {
    /// The identity.
    pub id: Id,
    /// The label block at the left; none shows a search icon.
    pub label: Option<&'a str>,
    /// The hint while empty.
    pub placeholder: &'a str,
    /// A validation message shown below the field.
    pub error: Option<&'a str>,
    /// A search field: shows the clear button, and Escape clears the text first.
    pub search: bool,
    /// The background it sits on.
    pub on: Option<Color>,
}

impl<'a> TextFieldControl<'a> {
    /// A plain field.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            label: None,
            placeholder: "",
            error: None,
            search: false,
            on: None,
        }
    }

    /// A search field (IntelliJ `SearchTextField`).
    pub fn search(id: Id) -> Self {
        Self {
            search: true,
            ..Self::new(id)
        }
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the placeholder.
    #[must_use]
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Sets the validation message.
    #[must_use]
    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    /// Sets the background it sits on.
    #[must_use]
    pub fn on(mut self, color: Color) -> Self {
        self.on = Some(color);
        self
    }

    /// The rows the field takes (more with an error).
    pub fn height(&self) -> u16 {
        1 + u16::from(self.error.is_some())
    }

    fn clear_id(&self) -> Id {
        self.id.with(0xC1EA)
    }

    fn widget<'b>(&'b self, rt: &'b Runtime, state: &'b TextFieldState) -> JBTextField<'b> {
        let mut field = JBTextField::new(&rt.look, state.text())
            .placeholder(self.placeholder)
            .error(self.error)
            .focused(rt.focused(self.id))
            .caret(state.caret())
            .selection(state.selection())
            .clear_button(self.search);
        if let Some(label) = self.label {
            field = field.label(label);
        }
        if let Some(on) = self.on {
            field = field.on(on);
        }
        field
    }

    /// Draws the field in `area` and registers it (and its clear button).
    pub fn show(&self, rt: &mut Runtime, state: &TextFieldState, area: Rect, buf: &mut Buffer) {
        self.widget(rt, state).render(area, buf);
        let row = Rect::new(area.x, area.y, area.width, 1);
        rt.widget(self.id, row);
        if self.search && !state.is_empty() && area.width > 3 {
            let cell = JBTextField::clear_cell(area);
            rt.hits.add(Rect::new(cell, area.y, 2, 1), self.clear_id());
        }
    }

    fn char_at(&self, rt: &Runtime, state: &TextFieldState, x: u16) -> Option<usize> {
        let rect = rt.hits.rect_of(self.id)?;
        let widget = self.widget(rt, state);
        let start = widget.text_start(rect);
        let end = if self.search && !state.is_empty() {
            JBTextField::clear_cell(rect)
        } else {
            rect.right().saturating_sub(1)
        };
        let chars: Vec<char> = state.text().chars().collect();
        let scroll = field_scroll(&chars, state.caret(), end.saturating_sub(start) as usize);
        let mut cx = start;
        let mut index = scroll;
        while index < chars.len() {
            let w = text_width(&chars[index].to_string()).max(1);
            if x < cx + w.div_ceil(2) {
                break;
            }
            cx += w;
            index += 1;
        }
        Some(index)
    }

    /// Applies `event` to `state` when the field has focus (keys, paste) or is under the
    /// mouse; says what happened.
    pub fn handle(
        &self,
        rt: &mut Runtime,
        state: &mut TextFieldState,
        event: &Event,
    ) -> FieldEvent {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if !rt.focused(self.id) {
                    return FieldEvent::Ignored;
                }
                self.handle_key(rt, state, key)
            }
            Event::Paste(text) if rt.focused(self.id) => {
                state.insert(text);
                FieldEvent::Changed
            }
            Event::Mouse(_) => self.handle_mouse(rt, state),
            _ => FieldEvent::Ignored,
        }
    }

    fn handle_key(
        &self,
        rt: &mut Runtime,
        state: &mut TextFieldState,
        key: &KeyEvent,
    ) -> FieldEvent {
        if key.code == KeyCode::Esc {
            if self.search && !state.is_empty() {
                state.set_text("");
                return FieldEvent::Cleared;
            }
            return FieldEvent::Cancelled;
        }
        if rt.is_action("ShowSearchHistory") && self.search {
            return FieldEvent::ShowHistory;
        }
        let before = state.revision;
        let mut applied = false;
        let actions: Vec<String> = rt.actions().to_vec();
        for id in &actions {
            // Editing actions only; the rest of the keymap is the app's.
            if state.apply_action(id, &mut rt.clipboard) {
                applied = true;
                break;
            }
        }
        if applied {
            return if state.revision != before {
                FieldEvent::Changed
            } else {
                FieldEvent::Consumed
            };
        }
        match key.code {
            KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown => FieldEvent::Ignored,
            KeyCode::Char(_) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                FieldEvent::Ignored
            }
            _ => state.apply_key(key),
        }
    }

    fn handle_mouse(&self, rt: &mut Runtime, state: &mut TextFieldState) -> FieldEvent {
        let Some(p) = rt.pointer().copied() else {
            return FieldEvent::Ignored;
        };
        if p.clicked(self.clear_id()) {
            state.set_text("");
            rt.focus.focus(self.id);
            return FieldEvent::Cleared;
        }
        match p.kind {
            PointerKind::Down(crate::input::MouseButton::Left) if p.over(self.id) => {
                rt.focus.focus(self.id);
                let Some(at) = self.char_at(rt, state, p.x) else {
                    return FieldEvent::Consumed;
                };
                match p.clicks {
                    2 => state.select_word(at),
                    3.. => state.select_all(),
                    _ => {
                        state.move_to(at, p.shift());
                        state.drag_origin = Some(at);
                        if !p.shift() {
                            state.anchor = None;
                        }
                    }
                }
                FieldEvent::Consumed
            }
            PointerKind::Drag(crate::input::MouseButton::Left) if p.origin == Some(self.id) => {
                if let (Some(origin), Some(at)) = (state.drag_origin, self.char_at(rt, state, p.x))
                {
                    state.anchor = Some(origin);
                    state.caret = at.min(state.len());
                }
                FieldEvent::Consumed
            }
            PointerKind::Up(crate::input::MouseButton::Left) => {
                state.drag_origin = None;
                FieldEvent::Ignored
            }
            _ => FieldEvent::Ignored,
        }
    }
}

/// The state of a search field: the text and the history of committed searches.
#[derive(Debug, Clone, Default)]
pub struct SearchFieldState {
    /// The editing state.
    pub field: TextFieldState,
    history: Vec<String>,
    cursor: Option<usize>,
}

/// How many searches the history keeps (IntelliJ's default: 5).
pub const SEARCH_HISTORY_LENGTH: usize = 5;

impl SearchFieldState {
    /// An empty search field.
    pub fn new() -> Self {
        Self::default()
    }

    /// The remembered searches, newest first.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Adds the current text to the history (on Enter or when a popup item is chosen).
    pub fn commit(&mut self) {
        let text = self.field.text().trim().to_owned();
        if text.is_empty() {
            return;
        }
        self.history.retain(|h| *h != text);
        self.history.insert(0, text);
        self.history.truncate(SEARCH_HISTORY_LENGTH);
        self.cursor = None;
    }

    /// Replaces the text with the next older (`older = true`) or newer history entry,
    /// wrapping around (Alt+Down / Alt+Up).
    pub fn cycle(&mut self, older: bool) {
        if self.history.is_empty() {
            return;
        }
        let len = self.history.len();
        let next = match (self.cursor, older) {
            (None, true) => 0,
            (None, false) => len - 1,
            (Some(i), true) => (i + 1) % len,
            (Some(i), false) => (i + len - 1) % len,
        };
        self.cursor = Some(next);
        let text = self.history[next].clone();
        self.field.set_text(text);
    }
}

/// An inline banner with action links: hover underlines a link, a click or Enter/Space on a
/// focused link performs it.
#[derive(Debug, Clone)]
pub struct BannerControl<'a> {
    /// The identity; links use child ids.
    pub id: Id,
    /// The severity.
    pub severity: Status,
    /// The message.
    pub text: &'a str,
    /// The action labels.
    pub actions: &'a [&'a str],
}

impl<'a> BannerControl<'a> {
    /// A banner.
    pub fn new(id: Id, severity: Status, text: &'a str, actions: &'a [&'a str]) -> Self {
        Self {
            id,
            severity,
            text,
            actions,
        }
    }

    /// Draws the banner in `area` (one row) and registers its links.
    pub fn show(&self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let hovered = self
            .actions
            .iter()
            .enumerate()
            .find(|(i, _)| rt.hovered() == Some(self.id.with(*i as u64)))
            .map(|(i, _)| i)
            .or_else(|| (0..self.actions.len()).find(|i| rt.focused(self.id.with(*i as u64))));
        let widget =
            InlineBanner::new(&rt.look, self.severity, self.text, self.actions).hovered(hovered);
        let rects = widget.action_rects(area);
        widget.render(area, buf);
        for (i, rect) in rects.iter().enumerate() {
            rt.widget(self.id.with(i as u64), *rect);
        }
    }

    /// Applies `event`; returns the index of the performed action.
    pub fn handle(&self, rt: &Runtime, event: &Event) -> Option<usize> {
        for i in 0..self.actions.len() {
            let id = self.id.with(i as u64);
            if activated(rt, id, event)
                || matches!(event, Event::Key(k) if rt.focused(id) && is_activation(k, true))
            {
                return Some(i);
            }
        }
        None
    }
}

/// Draws a [`RowIcon`] followed by text, for small label rows.
pub fn label_with_icon(
    rt: &Runtime,
    buf: &mut Buffer,
    area: Rect,
    icon: RowIcon,
    text: &str,
    bg: Color,
) {
    let x = crate::components::draw_icon(&rt.look, buf, area, area.x, area.y, icon, bg);
    crate::components::put(
        buf,
        area,
        x,
        area.y,
        text,
        ratatui::style::Style::default()
            .fg(rt.look.palette.white)
            .bg(bg),
    );
}
