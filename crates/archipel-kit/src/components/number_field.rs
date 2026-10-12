//! An integer input: a [`TextField`] that only writes valid numbers in range.

use super::TextField;
use crate::tokens::size;
use egui::{Id, Key, Modifiers, Response, Ui, Widget};
use std::ops::RangeInclusive;

/// An integer the [`NumberField`] edits: `i32`, `i64`, `u16`, `u32`, `u64` or `usize`.
pub trait Integer: Copy + sealed::Sealed {
    #[doc(hidden)]
    fn to_i128(self) -> i128;
    #[doc(hidden)]
    fn from_i128(value: i128) -> Self;
    #[doc(hidden)]
    const MIN: i128;
    #[doc(hidden)]
    const MAX: i128;
}

mod sealed {
    pub trait Sealed {}
}

macro_rules! integer {
    ($($t:ty),*) => {$(
        impl sealed::Sealed for $t {}
        impl Integer for $t {
            fn to_i128(self) -> i128 {
                self as i128
            }
            fn from_i128(value: i128) -> Self {
                value.clamp(<Self as Integer>::MIN, <Self as Integer>::MAX) as $t
            }
            const MIN: i128 = <$t>::MIN as i128;
            const MAX: i128 = <$t>::MAX as i128;
        }
    )*};
}
integer!(i32, i64, u16, u32, u64, usize);

/// An integer input (a page size, a timeout, a port), as Android Studio's integer fields:
/// the user types digits; a number in [`range`](Self::range) is written to the value at
/// once, anything else shows an error and leaves the value alone, and leaving the field
/// shows the value again. <kbd>Up</kbd>/<kbd>Down</kbd> step it by [`step`](Self::step).
///
/// The response is `changed()` when the value changed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::NumberField;
///
/// let mut limit: u64 = 50;
/// ui.add(NumberField::new(&mut limit).name("Limit").range(1..=1000).step(10));
/// # });
/// ```
#[must_use = "add it with `ui.add(field)`"]
pub struct NumberField<'a, T: Integer> {
    value: &'a mut T,
    name: Option<String>,
    min: i128,
    max: i128,
    step: i128,
    width: f32,
    enabled: bool,
    id: Option<Id>,
}

impl<'a, T: Integer> NumberField<'a, T> {
    /// A field editing `value`, accepting every value of its type.
    pub fn new(value: &'a mut T) -> Self {
        Self {
            value,
            name: None,
            min: T::MIN,
            max: T::MAX,
            step: 1,
            width: size::NUMBER_FIELD_WIDTH,
            enabled: true,
            id: None,
        }
    }

    /// The accessible name (the field has no visible label: put a
    /// [`FieldLabel`](super::FieldLabel) or form row label before it).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The values the field accepts.
    pub fn range(mut self, range: RangeInclusive<T>) -> Self {
        self.min = range.start().to_i128();
        self.max = range.end().to_i128().max(self.min);
        self
    }

    /// How much <kbd>Up</kbd> and <kbd>Down</kbd> change the value (default 1).
    pub fn step(mut self, step: T) -> Self {
        self.step = step.to_i128().max(1);
        self
    }

    /// Total width (default [`size::NUMBER_FIELD_WIDTH`]).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Disabled fields are dimmed and read-only.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The text edit's id, to focus it from outside.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    fn message(&self) -> String {
        match (self.min == T::MIN, self.max == T::MAX) {
            (true, true) => "Enter a whole number".to_owned(),
            (false, true) => format!("Enter a number of at least {}", self.min),
            (true, false) => format!("Enter a number of at most {}", self.max),
            (false, false) => format!("Enter a number from {} to {}", self.min, self.max),
        }
    }

    fn parse(&self, text: &str) -> Option<i128> {
        text.trim()
            .parse::<i128>()
            .ok()
            .filter(|n| (self.min..=self.max).contains(n))
    }
}

impl<T: Integer> Widget for NumberField<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let id = self.id.unwrap_or_else(|| ui.next_auto_id().with("number"));
        let buffer_id = id.with("archipel-number-text");
        let focused = ui.memory(|m| m.has_focus(id));
        let current = self.value.to_i128();
        let mut stepped = None;
        if focused && self.enabled {
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    stepped = Some(current.saturating_add(self.step));
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    stepped = Some(current.saturating_sub(self.step));
                }
            });
        }
        // While focused the field keeps what the user typed, valid or not; otherwise it
        // shows the value.
        let mut text = match (focused, stepped) {
            (_, Some(n)) => n.clamp(self.min, self.max).to_string(),
            (true, None) => ui
                .data(|d| d.get_temp::<String>(buffer_id))
                .unwrap_or_else(|| current.to_string()),
            (false, None) => current.to_string(),
        };
        let invalid = self.parse(&text).is_none();
        let error = invalid.then(|| self.message());
        let mut field = TextField::new(&mut text)
            .id(id)
            .width(self.width)
            .enabled(self.enabled)
            .error(error);
        if let Some(name) = &self.name {
            field = field.name(name.as_str());
        }
        let mut response = field.show(ui).response;
        if response.has_focus() {
            ui.data_mut(|d| d.insert_temp(buffer_id, text.clone()));
        } else {
            ui.data_mut(|d| d.remove_temp::<String>(buffer_id));
        }
        // `changed()` reports the value, not the text: an edit to something invalid or to
        // the same number is no change.
        let edited = response.changed() || stepped.is_some();
        response.flags.set(egui::response::Flags::CHANGED, false);
        if let Some(n) = self.parse(&text)
            && n != current
            && edited
        {
            *self.value = T::from_i128(n);
            response.mark_changed();
        }
        response
    }
}
