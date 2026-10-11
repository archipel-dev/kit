//! Numeric input built on [`TextField`].

use super::TextField;
use egui::{Id, Key, Modifiers, Response, Ui, Widget};
use std::fmt::Display;
use std::ops::RangeInclusive;

mod sealed {
    pub trait Sealed {}
}

/// A number type [`NumberField`] edits: `i32`, `i64`, `u32`, `u64`, `usize`, `f32` and `f64`.
/// Integers step exactly and saturate at their type's bounds; floats reject `NaN` and
/// infinities.
pub trait Number: Copy + PartialOrd + Display + sealed::Sealed + 'static {
    /// Parses user text; `None` when it is not a finite number of this type.
    fn parse(text: &str) -> Option<Self>;
    /// `self + step * times`, saturating for integers.
    fn nudge(self, step: Self, times: i32) -> Self;
    /// The default step.
    fn one() -> Self;
}

macro_rules! integer {
    ($($t:ty),*) => {$(
        impl sealed::Sealed for $t {}
        impl Number for $t {
            fn parse(text: &str) -> Option<Self> {
                text.trim().parse().ok()
            }
            fn nudge(self, step: Self, times: i32) -> Self {
                let value = self as i128 + step as i128 * times as i128;
                value.clamp(<$t>::MIN as i128, <$t>::MAX as i128) as $t
            }
            fn one() -> Self {
                1
            }
        }
    )*};
}
macro_rules! float {
    ($($t:ty),*) => {$(
        impl sealed::Sealed for $t {}
        impl Number for $t {
            fn parse(text: &str) -> Option<Self> {
                text.trim().parse::<$t>().ok().filter(|v| v.is_finite())
            }
            fn nudge(self, step: Self, times: i32) -> Self {
                // Rounded, so 0.1 + 0.2 reads 0.3 and not 0.30000000000000004.
                let value = self as f64 + step as f64 * times as f64;
                ((value * 1e9).round() / 1e9) as $t
            }
            fn one() -> Self {
                1.0
            }
        }
    )*};
}
integer!(i32, i64, u32, u64, usize);
float!(f32, f64);

/// A numeric input: a [`TextField`] (same look, focus ring and states) that edits a number
/// and keeps the last valid value.
///
/// Integers and floats are one generic type: `NumberField::new(&mut count)` works for any
/// [`Number`], with [`min`](Self::min), [`max`](Self::max) and [`step`](Self::step) typed
/// like the value (the default step is 1).
///
/// <kbd>↑</kbd>/<kbd>↓</kbd> add or subtract the step, <kbd>Shift</kbd> with them ten
/// steps, and the result stays within the range. Typing updates the value as soon as the
/// text is a valid number in range; otherwise the field shows the error state with a
/// message, the value stays what it was, and leaving the field restores the text of that
/// last valid value. The returned [`Response`] is the text edit's, and `changed()` is true
/// on the frames the value changed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::NumberField;
///
/// let mut iterations = 10_u32;
/// if ui
///     .add(NumberField::new(&mut iterations).label("Iterations").range(1..=1000))
///     .changed()
/// { /* ... */ }
///
/// let mut delay = 0.5_f64;
/// ui.add(NumberField::new(&mut delay).label("Delay (s)").min(0.0).step(0.1));
/// # });
/// ```
#[must_use = "add it with `ui.add(field)`"]
pub struct NumberField<'a, N: Number> {
    value: &'a mut N,
    min: Option<N>,
    max: Option<N>,
    step: N,
    label: Option<String>,
    enabled: bool,
    width: Option<f32>,
    id: Option<Id>,
}

impl<'a, N: Number> NumberField<'a, N> {
    /// An input editing `value`.
    pub fn new(value: &'a mut N) -> Self {
        Self {
            value,
            min: None,
            max: None,
            step: N::one(),
            label: None,
            enabled: true,
            width: None,
            id: None,
        }
    }

    /// Smallest allowed value.
    pub fn min(mut self, min: N) -> Self {
        self.min = Some(min);
        self
    }

    /// Largest allowed value.
    pub fn max(mut self, max: N) -> Self {
        self.max = Some(max);
        self
    }

    /// Both bounds at once.
    pub fn range(self, range: RangeInclusive<N>) -> Self {
        let (min, max) = range.into_inner();
        self.min(min).max(max)
    }

    /// What <kbd>↑</kbd>/<kbd>↓</kbd> add or subtract (default 1). <kbd>Shift</kbd> uses ten
    /// times as much.
    pub fn step(mut self, step: N) -> Self {
        self.step = step;
        self
    }

    /// A visible label above the input; it is also the accessible name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Disabled inputs are dimmed and read-only.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Total width in points (default: the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The id of the text edit, like [`TextField::id`]. The default is stable for a field
    /// that stays at the same place in its `Ui`; set it when fields come and go.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    fn in_range(&self, value: N) -> bool {
        self.min.is_none_or(|min| value >= min) && self.max.is_none_or(|max| value <= max)
    }

    fn clamp(&self, value: N) -> N {
        match (self.min, self.max) {
            (Some(min), _) if value < min => min,
            (_, Some(max)) if value > max => max,
            _ => value,
        }
    }

    /// Why `text` is not acceptable, `None` when it is.
    fn problem(&self, text: &str) -> Option<String> {
        match N::parse(text) {
            None => Some("Enter a number".to_owned()),
            Some(value) if self.in_range(value) => None,
            Some(_) => Some(match (self.min, self.max) {
                (Some(min), Some(max)) => format!("Enter a number from {min} to {max}"),
                (Some(min), None) => format!("Enter a number of at least {min}"),
                (None, Some(max)) => format!("Enter a number of at most {max}"),
                (None, None) => "Enter a number".to_owned(),
            }),
        }
    }
}

impl<N: Number> Widget for NumberField<'_, N> {
    fn ui(self, ui: &mut Ui) -> Response {
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        // The text being typed lives in egui's memory like a `TextEdit`'s cursor; the
        // caller only ever sees valid numbers.
        let mut draft: Option<String> = ui.data(|d| d.get_temp(id));
        let mut changed = false;

        if self.enabled && ui.memory(|m| m.has_focus(id)) {
            let direction = ui.input_mut(|input| {
                [
                    (Modifiers::SHIFT, Key::ArrowUp, 10),
                    (Modifiers::SHIFT, Key::ArrowDown, -10),
                    (Modifiers::NONE, Key::ArrowUp, 1),
                    (Modifiers::NONE, Key::ArrowDown, -1),
                ]
                .into_iter()
                .find(|&(modifiers, key, _)| input.consume_key(modifiers, key))
                .map(|(.., times)| times)
            });
            if let Some(times) = direction {
                let base = draft
                    .as_deref()
                    .and_then(N::parse)
                    .filter(|&v| self.in_range(v))
                    .unwrap_or(*self.value);
                let next = self.clamp(base.nudge(self.step, times));
                changed = next != *self.value;
                *self.value = next;
                draft = Some(next.to_string());
            }
        }

        let mut text = draft.clone().unwrap_or_else(|| self.value.to_string());
        let error = draft.as_deref().and_then(|d| self.problem(d));
        let mut field = TextField::new(&mut text)
            .id(id)
            .error(error)
            .enabled(self.enabled)
            .monospace(true);
        if let Some(label) = &self.label {
            field = field.label(label);
        }
        if let Some(width) = self.width {
            field = field.width(width);
        }
        let mut response = field.ui(ui);

        if response.changed() {
            if let Some(value) = N::parse(&text).filter(|&v| self.in_range(v)) {
                changed |= value != *self.value;
                *self.value = value;
            }
            draft = Some(text);
        }
        // Leaving the field drops the draft: an invalid one gives way to the last valid
        // value.
        if !response.has_focus() {
            draft = None;
        }
        ui.data_mut(|d| match draft {
            Some(draft) => d.insert_temp(id, draft),
            None => {
                d.remove_temp::<String>(id);
            }
        });
        if changed {
            response.mark_changed();
        }
        response
    }
}
