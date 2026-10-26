//! Speed search: type in a focused list, tree or table to jump to (or filter down to) the
//! rows that match. IntelliJ's `SpeedSearch`, `SpeedSearchComparator` and the popup of
//! `SpeedSearchBase` (spec section 3.4).

use crate::Glyph;
use crate::common;
use crate::icons;
use crate::theme::Theme;
use crate::tokens::{radius, size, speed_search as metrics, typography};
use egui::{
    CornerRadius, Event, InputState, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};
use std::ops::Range;

/// The punctuation that can start a speed search besides letters and digits.
pub const PUNCTUATION_MARKS: &str = "*_-+\"'/.#$>: ,;?!@%^&";

/// The state of a speed search, owned by the caller and passed to a collection with
/// [`speed_search`](crate::ui::JBList::speed_search).
///
/// The collection opens it when a letter, digit or punctuation mark is typed while it has
/// focus, edits [`text`](Self::text) as the user types and closes it again with
/// <kbd>Esc</kbd>, <kbd>Enter</kbd>, <kbd>←</kbd>, <kbd>→</kbd>, <kbd>PgUp</kbd> or
/// <kbd>PgDn</kbd>.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpeedSearchState {
    text: String,
    active: bool,
}

impl SpeedSearchState {
    /// A closed speed search.
    pub fn new() -> Self {
        Self::default()
    }

    /// What has been typed.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the popup is open. It stays open when the text is emptied with
    /// <kbd>Backspace</kbd> (`ide.speed.search.close.when.empty` is off).
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Opens the search with `text`, as if it had been typed.
    pub fn activate(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.active = true;
    }

    /// Empties the text; the popup stays open if it was.
    pub fn clear(&mut self) {
        self.text.clear();
    }

    /// Closes the popup and forgets the text.
    pub fn reset(&mut self) {
        self.text.clear();
        self.active = false;
    }
}

/// A speed search attached to a collection: its state and how it treats the rows that do not
/// match.
///
/// By default the search only *navigates*: all rows stay and the selection jumps to the first
/// match; <kbd>↑</kbd>/<kbd>↓</kbd>/<kbd>Home</kbd>/<kbd>End</kbd> then move between matches.
/// With [`filter`](Self::filter) the rows that do not match are hidden, as in list popups.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{ColoredListCellRenderer, JBList, SpeedSearch, SpeedSearchState};
///
/// let names = ["Cargo.toml", "README.md", "src"];
/// let mut selected = None;
/// let mut search = SpeedSearchState::new();
/// JBList::new("files", names.len(), &mut selected)
///     .speed_search(SpeedSearch::new(&mut search))
///     .show(ui, |i| ColoredListCellRenderer::new(names[i]));
/// # });
/// ```
pub struct SpeedSearch<'a> {
    pub(crate) state: &'a mut SpeedSearchState,
    pub(crate) filter: bool,
}

impl<'a> SpeedSearch<'a> {
    /// A speed search keeping its state in `state`.
    pub fn new(state: &'a mut SpeedSearchState) -> Self {
        Self {
            state,
            filter: false,
        }
    }

    /// Hides the rows that do not match (and shows "No matches" when none does), instead of
    /// only moving the selection between matches.
    pub fn filter(mut self, filter: bool) -> Self {
        self.filter = filter;
        self
    }
}

/// How a typed pattern matches a text: by the start of words and camel humps, with `*` for
/// anything and a space starting a new word (`SpeedSearchComparator`).
///
/// The match is case-insensitive unless the pattern has an uppercase letter, which then only
/// matches an uppercase letter. The first letter may match anywhere; later letters continue
/// the previous one or start a word (`rm` matches `README.md`, `nPE` matches
/// `NullPointerException`).
///
/// ```
/// use archipel_kit::ui::SpeedSearchComparator;
///
/// let comparator = SpeedSearchComparator::new("rm");
/// assert_eq!(comparator.matching_fragments("README.md"), Some(vec![0..1, 7..8]));
/// assert!(!comparator.matches("Cargo.toml"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedSearchComparator {
    tokens: Vec<Token>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gap {
    /// Anywhere after the previous match (the first letter, and after `*`).
    Anywhere,
    /// Right after the previous match, or at the start of a word.
    Continue,
    /// At the start of a word (after a space).
    WordStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Token {
    ch: char,
    gap: Gap,
}

/// Work limit of one match, so a pathological pattern cannot hang the UI.
const MATCH_BUDGET: u32 = 20_000;

impl SpeedSearchComparator {
    /// A comparator for `pattern`.
    pub fn new(pattern: &str) -> Self {
        let mut tokens = Vec::new();
        let mut gap = Gap::Anywhere;
        for ch in pattern.trim().chars() {
            match ch {
                '*' => gap = Gap::Anywhere,
                ' ' => {
                    if !tokens.is_empty() {
                        gap = Gap::WordStart;
                    }
                }
                _ => {
                    tokens.push(Token { ch, gap });
                    gap = Gap::Continue;
                }
            }
        }
        Self { tokens }
    }

    /// Whether the pattern is empty (it matches everything).
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Whether `text` matches.
    pub fn matches(&self, text: &str) -> bool {
        self.matching_fragments(text).is_some()
    }

    /// The parts of `text` that match, as character ranges (not byte ranges), or `None` when
    /// `text` does not match. An empty pattern matches with no fragments.
    pub fn matching_fragments(&self, text: &str) -> Option<Vec<Range<usize>>> {
        if self.tokens.is_empty() {
            return Some(Vec::new());
        }
        let chars: Vec<char> = text.chars().collect();
        let starts: Vec<bool> = (0..chars.len()).map(|i| word_start(&chars, i)).collect();
        let mut found = Vec::with_capacity(self.tokens.len());
        let mut budget = MATCH_BUDGET;
        if !self.search(&chars, &starts, 0, 0, &mut found, &mut budget) {
            return None;
        }
        let mut ranges: Vec<Range<usize>> = Vec::new();
        for index in found {
            match ranges.last_mut() {
                Some(last) if last.end == index => last.end += 1,
                _ => ranges.push(index..index + 1),
            }
        }
        Some(ranges)
    }

    fn search(
        &self,
        text: &[char],
        starts: &[bool],
        token: usize,
        from: usize,
        found: &mut Vec<usize>,
        budget: &mut u32,
    ) -> bool {
        let Some(current) = self.tokens.get(token) else {
            return true;
        };
        let candidates: Vec<usize> = match current.gap {
            Gap::Anywhere => {
                // Word starts first: the better match, as in the IDE.
                let all = from..text.len();
                all.clone()
                    .filter(|&i| starts[i])
                    .chain(all.filter(|&i| !starts[i]))
                    .collect()
            }
            Gap::Continue => {
                let next = found.last().map_or(from, |&last| last + 1);
                let mut list = vec![next];
                list.extend((next + 1..text.len()).filter(|&i| starts[i]));
                list
            }
            Gap::WordStart => (from..text.len()).filter(|&i| starts[i]).collect(),
        };
        for index in candidates {
            if index >= text.len() {
                continue;
            }
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            if !same_letter(current.ch, text[index]) {
                continue;
            }
            found.push(index);
            if self.search(text, starts, token + 1, index + 1, found, budget) {
                return true;
            }
            found.pop();
        }
        false
    }
}

/// An uppercase pattern letter matches only itself; anything else ignores case.
fn same_letter(pattern: char, text: char) -> bool {
    pattern == text || (!pattern.is_uppercase() && pattern.to_lowercase().eq(text.to_lowercase()))
}

/// Whether a word or camel hump starts at `index`.
fn word_start(text: &[char], index: usize) -> bool {
    let Some(&current) = text.get(index) else {
        return false;
    };
    if index == 0 {
        return true;
    }
    let previous = text[index - 1];
    if !current.is_alphanumeric() {
        return false;
    }
    if !previous.is_alphanumeric() {
        return true;
    }
    if current.is_uppercase() && previous.is_lowercase() {
        return true;
    }
    // An acronym followed by a word: "HTTPServer" starts a hump at "S".
    if current.is_uppercase()
        && previous.is_uppercase()
        && text.get(index + 1).is_some_and(|next| next.is_lowercase())
    {
        return true;
    }
    current.is_ascii_digit() != previous.is_ascii_digit()
}

/// Whether typing `ch` in a closed speed search opens it: letters, digits and the
/// [`PUNCTUATION_MARKS`]; a space alone does not.
pub(crate) fn starts_search(ch: char) -> bool {
    ch != ' ' && !ch.is_control() && (ch.is_alphanumeric() || PUNCTUATION_MARKS.contains(ch))
}

/// Where a speed search moves the selection among the matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Jump {
    Previous,
    Next,
    First,
    Last,
}

/// What a frame's input does to a collection with a speed search, in the order it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// The text changed (a letter was typed or deleted): select the first match.
    Changed,
    /// Move among the matches.
    Jump(Jump),
    /// A key the speed search did not take; the collection handles it.
    Pass(Key, Modifiers),
}

/// Reads this frame's keyboard input for a focused collection, updates `state` and returns
/// what the collection has to do, in order.
pub(crate) fn actions(input: &InputState, state: &mut SpeedSearchState) -> Vec<Action> {
    let mut out = Vec::new();
    let typing_allowed = !input.modifiers.alt && !input.modifiers.command;
    for event in &input.events {
        match event {
            Event::Text(text) if typing_allowed => {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    if state.active {
                        state.text.push(ch);
                        out.push(Action::Changed);
                    } else if starts_search(ch) {
                        state.activate(ch);
                        out.push(Action::Changed);
                    }
                }
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let plain = !modifiers.shift && !modifiers.alt && !modifiers.command;
                if !state.active {
                    out.push(Action::Pass(*key, *modifiers));
                    continue;
                }
                match key {
                    Key::Backspace => {
                        if modifiers.command {
                            state.text.clear();
                        } else {
                            state.text.pop();
                        }
                        out.push(Action::Changed);
                    }
                    Key::Escape => state.reset(),
                    Key::ArrowUp if plain => out.push(Action::Jump(Jump::Previous)),
                    Key::ArrowDown if plain => out.push(Action::Jump(Jump::Next)),
                    Key::Home if plain => out.push(Action::Jump(Jump::First)),
                    Key::End if plain => out.push(Action::Jump(Jump::Last)),
                    Key::PageUp | Key::PageDown | Key::ArrowLeft | Key::ArrowRight | Key::Enter => {
                        state.reset();
                        out.push(Action::Pass(*key, *modifiers));
                    }
                    _ => out.push(Action::Pass(*key, *modifiers)),
                }
            }
            _ => {}
        }
    }
    out
}

/// Paints the speed search popup at the top-left of `viewport`: a bordered field with a
/// search icon, the text (in the error color when `no_hits`) or the "Search" hint.
pub(crate) fn paint_popup(ui: &Ui, viewport: Rect, state: &SpeedSearchState, no_hits: bool) {
    let colors = Theme::colors(ui.ctx());
    let font = typography::body(typography::BODY);
    let hint = common::galley(ui, metrics::PLACEHOLDER, font.clone());
    let text = common::galley(ui, state.text(), font);
    let text_width = text.size().x.max(hint.size().x);
    let width =
        metrics::ICON_INSET + size::ICON + metrics::ICON_GAP + text_width + metrics::PADDING_END;
    let rect = Rect::from_min_size(
        viewport.min,
        vec2(width.min(viewport.width()), metrics::HEIGHT),
    );
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect(
        rect,
        CornerRadius::same(radius::CONTROL),
        colors.speed_search_background,
        Stroke::new(1.0_f32, colors.speed_search_border),
        StrokeKind::Inside,
    );
    let icon = Rect::from_min_size(
        pos2(
            rect.left() + metrics::ICON_INSET,
            rect.center().y - size::ICON / 2.0,
        ),
        vec2(size::ICON, size::ICON),
    );
    icons::paint(&painter, icon, Glyph::Search, colors.icon);
    let x: f32 = icon.right() + metrics::ICON_GAP;
    if state.text().is_empty() {
        common::text_left(&painter, rect, x, hint, colors.text_placeholder);
    } else {
        let color = if no_hits { colors.error } else { colors.text };
        common::text_left(&painter, rect, x, text, color);
    }
    // For screen readers and tests: a named search field with the typed text.
    let field = ui.interact(rect, ui.id().with("speed-search-popup"), Sense::hover());
    field.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Speed search")
    });
    ui.ctx().accesskit_node_builder(field.id, |node| {
        node.set_role(egui::accesskit::Role::SearchInput);
        node.set_value(state.text());
    });
}
