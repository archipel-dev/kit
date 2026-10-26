//! Terminal input as the kit's input model: crossterm events become [`Event`]s, and key
//! events become [`KeyStroke`]s that the GUI [`Keymap`] resolves, so the GUI and the
//! terminal share one keymap.
//!
//! Terminals are limited: a legacy terminal cannot send <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+letter
//! (it looks like <kbd>Ctrl</kbd>+letter), <kbd>Ctrl</kbd>+<kbd>Tab</kbd>, or
//! <kbd>Ctrl</kbd>+digit, and on macOS <kbd>Option</kbd>+digit types a symbol. The kitty
//! keyboard protocol, which [`crate::runtime::setup`] enables when the terminal has it,
//! removes these limits. [`terminal_keymap`] adds fallbacks for the combos a legacy terminal
//! cannot send, and [`display_shortcut`] shows the shortcut that the terminal can send.

use archipel_kit::action_system::{KeyStroke, KeyboardShortcut, Keymap, KeymapUtil, Os, Shortcut};
use archipel_kit::egui::{Key, Modifiers};
use ratatui::crossterm::event as ct;
pub use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

/// An input event, mapped from crossterm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// A key was pressed (or repeated).
    Key(KeyEvent),
    /// A key was released. Only terminals with the kitty keyboard protocol send these.
    KeyUp(KeyEvent),
    /// The mouse moved, clicked, dragged or scrolled.
    Mouse(MouseEvent),
    /// The terminal was resized to (columns, rows).
    Resize(u16, u16),
    /// Text was pasted (bracketed paste).
    Paste(String),
    /// The terminal window gained focus.
    FocusGained,
    /// The terminal window lost focus.
    FocusLost,
}

impl Event {
    /// The kit event for a crossterm `event`.
    pub fn from_crossterm(event: ct::Event) -> Option<Self> {
        Some(match event {
            ct::Event::Key(key) => match key.kind {
                KeyEventKind::Release => Event::KeyUp(key),
                _ => Event::Key(key),
            },
            ct::Event::Mouse(mouse) => Event::Mouse(mouse),
            ct::Event::Resize(w, h) => Event::Resize(w, h),
            ct::Event::Paste(text) => Event::Paste(text),
            ct::Event::FocusGained => Event::FocusGained,
            ct::Event::FocusLost => Event::FocusLost,
        })
    }
}

/// What the terminal can send and the operating system the keymap follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// The kitty keyboard protocol is on: Ctrl+Shift combos, key release and modifier keys
    /// are reported.
    pub kitty: bool,
    /// The operating system, which picks the keymap and shortcut symbols.
    pub os: Os,
}

impl Caps {
    /// A legacy terminal on this operating system.
    pub fn legacy() -> Self {
        Self {
            kitty: false,
            os: Os::current(),
        }
    }

    /// A terminal with the kitty keyboard protocol.
    pub fn kitty() -> Self {
        Self {
            kitty: true,
            os: Os::current(),
        }
    }

    /// Whether the terminal reports key release and modifier keys.
    pub fn reports_modifiers(&self) -> bool {
        self.kitty
    }
}

impl Default for Caps {
    fn default() -> Self {
        Self::legacy()
    }
}

/// Shifted US-layout symbols and the key plus Shift that types them.
const SHIFTED: &[(char, Key)] = &[
    ('!', Key::Num1),
    ('@', Key::Num2),
    ('#', Key::Num3),
    ('$', Key::Num4),
    ('%', Key::Num5),
    ('^', Key::Num6),
    ('&', Key::Num7),
    ('*', Key::Num8),
    ('(', Key::Num9),
    (')', Key::Num0),
    ('_', Key::Minus),
    ('+', Key::Equals),
    ('{', Key::OpenBracket),
    ('}', Key::CloseBracket),
    ('|', Key::Backslash),
    (':', Key::Semicolon),
    ('"', Key::Quote),
    ('<', Key::Comma),
    ('>', Key::Period),
    ('?', Key::Slash),
    ('~', Key::Backtick),
];

/// Unshifted symbols.
const SYMBOLS: &[(char, Key)] = &[
    ('-', Key::Minus),
    ('=', Key::Equals),
    ('[', Key::OpenBracket),
    (']', Key::CloseBracket),
    ('\\', Key::Backslash),
    (';', Key::Semicolon),
    ('\'', Key::Quote),
    (',', Key::Comma),
    ('.', Key::Period),
    ('/', Key::Slash),
    ('`', Key::Backtick),
];

fn digit_key(d: u32) -> Option<Key> {
    Some(match d {
        0 => Key::Num0,
        1 => Key::Num1,
        2 => Key::Num2,
        3 => Key::Num3,
        4 => Key::Num4,
        5 => Key::Num5,
        6 => Key::Num6,
        7 => Key::Num7,
        8 => Key::Num8,
        9 => Key::Num9,
        _ => return None,
    })
}

fn letter_key(c: char) -> Option<Key> {
    Key::from_name(&c.to_ascii_uppercase().to_string())
}

fn function_key(n: u8) -> Option<Key> {
    Key::from_name(&format!("F{n}"))
}

/// The modifiers of `mods` as the kit's.
pub fn modifiers(mods: KeyModifiers) -> Modifiers {
    Modifiers {
        alt: mods.contains(KeyModifiers::ALT),
        ctrl: mods.contains(KeyModifiers::CONTROL),
        shift: mods.contains(KeyModifiers::SHIFT),
        mac_cmd: mods.contains(KeyModifiers::SUPER),
        command: false,
    }
}

/// The key stroke of `event`: the key as the GUI's [`Key`] with [`Modifiers`], so that the
/// kit [`Keymap`] can resolve it. `None` for keys the keymap has no name for (modifier
/// keys, media keys).
///
/// A legacy terminal sends a capital letter without a Shift flag, a shifted symbol as the
/// symbol; both are mapped back to the key plus Shift.
pub fn key_stroke(event: &KeyEvent) -> Option<KeyStroke> {
    let mut mods = modifiers(event.modifiers);
    let key = match event.code {
        KeyCode::Char(c) => {
            if c == ' ' {
                Key::Space
            } else if let Some(d) = c.to_digit(10).filter(|_| c.is_ascii_digit()) {
                digit_key(d)?
            } else if c.is_ascii_alphabetic() {
                if c.is_ascii_uppercase() {
                    mods.shift = true;
                }
                letter_key(c)?
            } else if let Some((_, key)) = SHIFTED.iter().find(|(s, _)| *s == c) {
                mods.shift = true;
                *key
            } else if let Some((_, key)) = SYMBOLS.iter().find(|(s, _)| *s == c) {
                *key
            } else {
                return None;
            }
        }
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => {
            mods.shift = true;
            Key::Tab
        }
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Left => Key::ArrowLeft,
        KeyCode::Right => Key::ArrowRight,
        KeyCode::Up => Key::ArrowUp,
        KeyCode::Down => Key::ArrowDown,
        KeyCode::F(n) => function_key(n)?,
        _ => return None,
    };
    Some(KeyStroke::new(key, mods))
}

/// The character `event` types into a text field: no Ctrl, Alt or Super, and a character
/// that is not a control character (IntelliJ's "really typed").
pub fn typed_char(event: &KeyEvent) -> Option<char> {
    if event
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return None;
    }
    match event.code {
        KeyCode::Char(c) if !c.is_control() => Some(c),
        _ => None,
    }
}

/// Whether `event` is the key `code` with no Ctrl, Alt or Super (Shift is allowed).
pub fn is_plain(event: &KeyEvent, code: KeyCode) -> bool {
    event.code == code
        && !event
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
}

/// Whether the shift key is held in `event`.
pub fn shift(event: &KeyEvent) -> bool {
    event.modifiers.contains(KeyModifiers::SHIFT)
}

/// Whether Ctrl (or Cmd on macOS keyboards that send Super) is held: the toggle-selection
/// modifier of lists.
pub fn command(event: &KeyEvent) -> bool {
    event
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER)
}

/// A key press of `code` without modifiers, for tests and synthetic input.
pub fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// A key press of `code` with `mods`, for tests and synthetic input.
pub fn key_with(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

/// Key presses typing `text`, for tests and synthetic input.
pub fn typing(text: &str) -> Vec<Event> {
    text.chars().map(|c| key(KeyCode::Char(c))).collect()
}

/// A key press of Ctrl+`c`, for tests and synthetic input.
pub fn ctrl(c: char) -> Event {
    key_with(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// A key press of Alt+`c`, for tests and synthetic input.
pub fn alt(c: char) -> Event {
    key_with(KeyCode::Char(c), KeyModifiers::ALT)
}

/// The chord starters of [`terminal_keymap`]: <kbd>Ctrl</kbd>+<kbd>K</kbd> for editor
/// and general actions, <kbd>Ctrl</kbd>+<kbd>W</kbd> for tool windows (as Vim's window
/// commands).
pub const LEADER_GENERAL: &str = "ctrl K";
/// See [`LEADER_GENERAL`].
pub const LEADER_WINDOWS: &str = "ctrl W";

/// The action ids for tool window activation: `ActivateToolWindow1` … `ActivateToolWindow9`
/// (IntelliJ's `ActivateXToolWindow` family, bound to Alt+digit).
pub fn activate_tool_window_id(digit: u8) -> String {
    format!("ActivateToolWindow{digit}")
}

/// The kit's default keymap for `os` plus the tool window bindings the kit leaves to the
/// host, and terminal fallbacks for combos a legacy terminal cannot send.
///
/// Added bindings (all also work in kitty-protocol terminals):
///
/// | Action | GUI | Terminal fallback |
/// |---|---|---|
/// | `ActivateToolWindow1..9` | Alt+1..9 | Ctrl+W, then 1..9 |
/// | `HideAllWindows` | Ctrl+Shift+F12 | Ctrl+W, then H |
/// | `MaximizeToolWindow` | Ctrl+Shift+' | Ctrl+W, then M |
/// | `ResizeToolWindowLeft/Right/Up/Down` | Ctrl+Alt+Shift+arrow | Ctrl+W, then arrow |
/// | `GotoAction` | Ctrl+Shift+A | Ctrl+P, or Ctrl+K then A |
/// | `FindInPath` | Ctrl+Shift+F | Ctrl+K, then F |
/// | `$Redo` | Ctrl+Shift+Z | Ctrl+K, then Z |
/// | `Switcher` | Ctrl+Tab | Ctrl+K, then T |
/// | `CloseActiveTab` | Ctrl+Shift+F4 | Ctrl+K, then W |
/// | `ShowIntentionActions` | Alt+Enter | Ctrl+K, then Enter |
pub fn terminal_keymap(os: Os) -> Keymap {
    let mut keymap = Keymap::default_for(os);
    let mut add = |id: &str, text: &str| {
        if let Some(shortcut) = Shortcut::parse(text) {
            keymap.add_shortcut(id, shortcut);
        }
    };
    for digit in 1..=9u8 {
        add(&activate_tool_window_id(digit), &format!("alt {digit}"));
        add(
            &activate_tool_window_id(digit),
            &format!("{LEADER_WINDOWS} > {digit}"),
        );
    }
    add("HideAllWindows", &format!("{LEADER_WINDOWS} > H"));
    add("MaximizeToolWindow", &format!("{LEADER_WINDOWS} > M"));
    for (id, key) in [
        ("ResizeToolWindowLeft", "Left"),
        ("ResizeToolWindowRight", "Right"),
        ("ResizeToolWindowUp", "Up"),
        ("ResizeToolWindowDown", "Down"),
    ] {
        add(id, &format!("ctrl alt shift {key}"));
        add(id, &format!("{LEADER_WINDOWS} > {key}"));
    }
    add("GotoAction", "ctrl P");
    add("GotoAction", &format!("{LEADER_GENERAL} > A"));
    add("FindInPath", &format!("{LEADER_GENERAL} > F"));
    add("$Redo", &format!("{LEADER_GENERAL} > Z"));
    add("Switcher", &format!("{LEADER_GENERAL} > T"));
    add("CloseActiveTab", &format!("{LEADER_GENERAL} > W"));
    add("ShowIntentionActions", &format!("{LEADER_GENERAL} > Enter"));
    add("Exit", &format!("{LEADER_GENERAL} > Q"));
    keymap
}

/// Whether a terminal with `caps` can send `stroke`.
///
/// Without the kitty protocol: Ctrl with Shift and a letter, Ctrl+Tab, Ctrl+digit and
/// Ctrl+punctuation other than the classic control codes look like other keys; Alt+digit
/// and Alt+letter are not sent on macOS (Option types a symbol) unless the terminal maps
/// Option to Alt; Super (Cmd) is never sent.
pub fn sendable(stroke: &KeyStroke, caps: &Caps) -> bool {
    if caps.kitty {
        return true;
    }
    let m = &stroke.modifiers;
    if m.mac_cmd {
        return false;
    }
    let letter = matches!(
        stroke.key,
        Key::A
            | Key::B
            | Key::C
            | Key::D
            | Key::E
            | Key::F
            | Key::G
            | Key::H
            | Key::I
            | Key::J
            | Key::K
            | Key::L
            | Key::M
            | Key::N
            | Key::O
            | Key::P
            | Key::Q
            | Key::R
            | Key::S
            | Key::T
            | Key::U
            | Key::V
            | Key::W
            | Key::X
            | Key::Y
            | Key::Z
    );
    let digit = matches!(
        stroke.key,
        Key::Num0
            | Key::Num1
            | Key::Num2
            | Key::Num3
            | Key::Num4
            | Key::Num5
            | Key::Num6
            | Key::Num7
            | Key::Num8
            | Key::Num9
    );
    if m.ctrl {
        if letter && m.shift {
            return false;
        }
        if digit
            || matches!(
                stroke.key,
                Key::Quote | Key::Period | Key::Comma | Key::Semicolon
            )
        {
            return false;
        }
        if stroke.key == Key::Tab || stroke.key == Key::Enter || stroke.key == Key::Space && m.shift
        {
            return false;
        }
        if m.shift
            && matches!(
                stroke.key,
                Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown
            )
            && m.alt
        {
            // Ctrl+Alt+Shift+arrow is a legacy xterm sequence, but terminals differ.
            return true;
        }
    }
    if m.alt && caps.os.is_mac() && (digit || letter) {
        return false;
    }
    true
}

/// The text of the best shortcut of `action_id` for a terminal with `caps`: the first
/// keyboard shortcut the terminal can send, else the first one. Chords are shown with
/// their both strokes.
pub fn display_shortcut(keymap: &Keymap, action_id: &str, caps: &Caps) -> Option<String> {
    let shortcuts: Vec<KeyboardShortcut> = keymap
        .shortcuts(action_id)
        .into_iter()
        .filter_map(|s| match s {
            Shortcut::Keyboard(k) => Some(k),
            _ => None,
        })
        .collect();
    let can_send = |k: &KeyboardShortcut| {
        sendable(&k.first, caps) && k.second.as_ref().is_none_or(|s| sendable(s, caps))
    };
    let pick = shortcuts
        .iter()
        .find(|k| can_send(k))
        .or_else(|| shortcuts.first())?;
    Some(KeymapUtil::shortcut_text(pick, caps.os.is_mac()))
}

/// Detects the double-tap of <kbd>Shift</kbd> that opens Search Everywhere in IntelliJ.
///
/// Needs a terminal with modifier key reporting (kitty protocol with all keys reported);
/// on other terminals it never fires, and apps should offer the Go to Action shortcut.
#[derive(Debug, Clone, Default)]
pub struct DoubleShift {
    down: bool,
    dirty: bool,
    last_tap: Option<f64>,
}

/// How close two taps of Shift must be (IntelliJ: 500 ms).
pub const DOUBLE_SHIFT_WINDOW: f64 = 0.5;

impl DoubleShift {
    /// Feeds `event` seen at time `now`; returns true on the second tap.
    pub fn feed(&mut self, event: &Event, now: f64) -> bool {
        let is_shift = |k: &KeyEvent| {
            matches!(
                k.code,
                KeyCode::Modifier(ct::ModifierKeyCode::LeftShift | ct::ModifierKeyCode::RightShift)
            )
        };
        match event {
            Event::Key(k) if is_shift(k) => {
                if !self.down {
                    self.down = true;
                    self.dirty = false;
                }
            }
            Event::KeyUp(k) if is_shift(k) => {
                self.down = false;
                if self.dirty {
                    self.last_tap = None;
                    return false;
                }
                if let Some(last) = self.last_tap.take()
                    && now - last <= DOUBLE_SHIFT_WINDOW
                {
                    return true;
                }
                self.last_tap = Some(now);
            }
            Event::Key(_)
            | Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(_),
                ..
            }) => {
                self.dirty = true;
                self.last_tap = None;
            }
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_kit::action_system::KeyResolution;

    fn stroke(text: &str) -> KeyStroke {
        KeyStroke::parse(text).expect("stroke")
    }

    #[test]
    fn letters_digits_and_symbols_map_to_the_gui_keys() {
        let k = |code, mods| key_stroke(&KeyEvent::new(code, mods)).expect("stroke");
        assert_eq!(k(KeyCode::Char('a'), KeyModifiers::NONE), stroke("A"));
        assert_eq!(k(KeyCode::Char('A'), KeyModifiers::NONE), stroke("shift A"));
        assert_eq!(
            k(KeyCode::Char('a'), KeyModifiers::CONTROL),
            stroke("ctrl A")
        );
        assert_eq!(k(KeyCode::Char('1'), KeyModifiers::ALT), stroke("alt 1"));
        assert_eq!(k(KeyCode::Char('?'), KeyModifiers::NONE), stroke("shift /"));
        assert_eq!(k(KeyCode::BackTab, KeyModifiers::NONE), stroke("shift Tab"));
        assert_eq!(
            k(KeyCode::F(12), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            stroke("ctrl shift F12")
        );
        assert_eq!(k(KeyCode::Esc, KeyModifiers::SHIFT), stroke("shift Esc"));
        assert_eq!(
            k(
                KeyCode::Char('\''),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            ),
            stroke("ctrl shift '")
        );
    }

    #[test]
    fn modifier_keys_have_no_stroke() {
        let key = KeyEvent::new(
            KeyCode::Modifier(ct::ModifierKeyCode::LeftShift),
            KeyModifiers::NONE,
        );
        assert_eq!(key_stroke(&key), None);
    }

    #[test]
    fn typed_chars_exclude_control_alt_and_super() {
        assert_eq!(
            typed_char(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::SHIFT)),
            Some('x')
        );
        assert_eq!(
            typed_char(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            typed_char(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
            None
        );
        assert_eq!(
            typed_char(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            None
        );
    }

    #[test]
    fn the_terminal_keymap_resolves_the_gui_keys_and_the_fallback_chords() {
        let keymap = terminal_keymap(Os::Linux);
        assert_eq!(
            keymap.resolve(&stroke("alt 1")),
            KeyResolution::Matched(vec!["ActivateToolWindow1".into()])
        );
        assert_eq!(
            keymap.resolve(&stroke("ctrl W")),
            KeyResolution::AwaitSecond
        );
        let mut pending = None;
        assert_eq!(
            keymap.feed(&mut pending, stroke("ctrl W"), 0.0),
            KeyResolution::AwaitSecond
        );
        assert_eq!(
            keymap.feed(&mut pending, stroke("3"), 0.5),
            KeyResolution::Matched(vec!["ActivateToolWindow3".into()])
        );
        // The second stroke must come within two seconds.
        assert_eq!(
            keymap.feed(&mut pending, stroke("ctrl W"), 1.0),
            KeyResolution::AwaitSecond
        );
        assert_ne!(
            keymap.feed(&mut pending, stroke("3"), 4.0),
            KeyResolution::Matched(vec!["ActivateToolWindow3".into()])
        );
    }

    #[test]
    fn a_legacy_terminal_cannot_send_ctrl_shift_letters_or_ctrl_tab() {
        let legacy = Caps {
            kitty: false,
            os: Os::Linux,
        };
        let kitty = Caps {
            kitty: true,
            os: Os::Linux,
        };
        for text in ["ctrl shift A", "ctrl Tab", "ctrl 1", "cmd C"] {
            assert!(!sendable(&stroke(text), &legacy), "{text}");
            assert!(
                text.starts_with("cmd") && !sendable(&stroke(text), &legacy)
                    || sendable(&stroke(text), &kitty),
                "{text}"
            );
        }
        assert!(sendable(&stroke("ctrl A"), &legacy));
        assert!(sendable(&stroke("alt Left"), &legacy));
        assert!(!sendable(
            &stroke("alt 1"),
            &Caps {
                kitty: false,
                os: Os::MacOs
            }
        ));
    }

    #[test]
    fn the_shortcut_text_shows_the_combo_the_terminal_can_send() {
        let keymap = terminal_keymap(Os::Linux);
        let legacy = Caps {
            kitty: false,
            os: Os::Linux,
        };
        let kitty = Caps {
            kitty: true,
            os: Os::Linux,
        };
        assert_eq!(
            display_shortcut(&keymap, "GotoAction", &kitty).as_deref(),
            Some("Ctrl+Shift+A")
        );
        assert_eq!(
            display_shortcut(&keymap, "GotoAction", &legacy).as_deref(),
            Some("Ctrl+P")
        );
        assert_eq!(
            display_shortcut(&keymap, "ActivateToolWindow1", &legacy).as_deref(),
            Some("Alt+1")
        );
        let mac = Caps {
            kitty: false,
            os: Os::MacOs,
        };
        let mac_keymap = terminal_keymap(Os::MacOs);
        assert_eq!(
            display_shortcut(&mac_keymap, "ActivateToolWindow1", &mac).as_deref(),
            Some("⌃W 1"),
            "Option+digit types a symbol on macOS: the chord is shown"
        );
    }

    fn shift(up: bool) -> Event {
        let key = KeyEvent::new_with_kind(
            KeyCode::Modifier(ct::ModifierKeyCode::LeftShift),
            KeyModifiers::SHIFT,
            if up {
                KeyEventKind::Release
            } else {
                KeyEventKind::Press
            },
        );
        if up {
            Event::KeyUp(key)
        } else {
            Event::Key(key)
        }
    }

    #[test]
    fn two_taps_of_shift_within_half_a_second_are_a_double_shift() {
        let mut d = DoubleShift::default();
        assert!(!d.feed(&shift(false), 0.0));
        assert!(!d.feed(&shift(true), 0.05));
        assert!(!d.feed(&shift(false), 0.2));
        assert!(d.feed(&shift(true), 0.25));
    }

    #[test]
    fn slow_taps_or_a_key_in_between_are_not_a_double_shift() {
        let mut d = DoubleShift::default();
        d.feed(&shift(false), 0.0);
        d.feed(&shift(true), 0.05);
        d.feed(&shift(false), 0.8);
        assert!(!d.feed(&shift(true), 0.85), "too slow");
        let mut d = DoubleShift::default();
        d.feed(&shift(false), 0.0);
        d.feed(&shift(true), 0.05);
        d.feed(&key(KeyCode::Char('a')), 0.1);
        d.feed(&shift(false), 0.2);
        assert!(!d.feed(&shift(true), 0.25), "a key in between resets it");
        let mut d = DoubleShift::default();
        d.feed(&shift(false), 0.0);
        d.feed(&key(KeyCode::Char('A')), 0.02);
        d.feed(&shift(true), 0.05);
        d.feed(&shift(false), 0.1);
        assert!(
            !d.feed(&shift(true), 0.15),
            "Shift used as a modifier is not a tap"
        );
    }
}
