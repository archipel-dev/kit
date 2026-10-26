//! Shortcuts and keymaps: [`KeyStroke`], [`KeyboardShortcut`], [`Shortcut`], [`Keymap`] and
//! [`KeymapUtil`].

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use egui::{Key, Modifiers, PointerButton};

/// How long the second stroke of a two-stroke shortcut may take: 2000 ms.
///
/// Source: the IntelliJ registry value `actionSystem.secondKeystrokeTimeout` (2000 ms), see
/// the UX notes. It is a plain constant, not a design token: it is behavior, not look.
pub const SECOND_KEYSTROKE_TIMEOUT: Duration = Duration::from_millis(2000);

/// The operating system a keymap or a shortcut text is for.
///
/// IntelliJ ships one keymap file per family (`$default.xml` for Windows and Linux, the
/// `Mac OS X` ones for macOS); this is the switch between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Os {
    /// Windows, with the default (`$default`) keymap.
    Windows,
    /// Linux, with the default (`$default`) keymap.
    Linux,
    /// macOS, with the `Mac OS X` keymap and symbol shortcut texts.
    MacOs,
}

impl Os {
    /// The operating system this binary was built for.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }

    /// Whether this is macOS (symbol shortcut texts, Cmd instead of Ctrl).
    pub fn is_mac(self) -> bool {
        self == Self::MacOs
    }
}

/// Drops `command`, which is derived from `ctrl` or `mac_cmd` depending on the platform, so
/// strokes compare the same everywhere.
fn normalize(modifiers: Modifiers) -> Modifiers {
    Modifiers {
        alt: modifiers.alt,
        ctrl: modifiers.ctrl,
        shift: modifiers.shift,
        mac_cmd: modifiers.mac_cmd,
        command: false,
    }
}

fn modifier_bits(modifiers: &Modifiers) -> u8 {
    u8::from(modifiers.ctrl)
        | u8::from(modifiers.alt) << 1
        | u8::from(modifiers.shift) << 2
        | u8::from(modifiers.mac_cmd) << 3
}

/// A key with modifiers.
///
/// IntelliJ counterpart: `javax.swing.KeyStroke` as used by `KeyboardShortcut`. Comparison
/// ignores egui's derived `command` flag, so a stroke built from a key event equals one
/// parsed from `"ctrl C"` (or `"cmd C"` on macOS, where `cmd` is `mac_cmd`).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct KeyStroke {
    /// The key.
    pub key: Key,
    /// The modifiers held with it.
    pub modifiers: Modifiers,
}

impl KeyStroke {
    /// A stroke of `key` with `modifiers`.
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers: normalize(modifiers),
        }
    }

    /// Parses IntelliJ's textual form: modifiers then the key, separated by spaces, such as
    /// `"ctrl shift A"` or `"alt Enter"` (IntelliJ `KeyStroke.getKeyStroke` and
    /// `CustomShortcutSet.fromString`). Modifiers: `ctrl`, `alt` (`option`), `shift`, `cmd`
    /// (`meta`, `command`). Keys: egui key names plus `Esc`, `PgUp`, `PgDn`, `Del`, `Ins`,
    /// and the symbols `/ . , [ ] - = ' ; \` ` `. Returns `None` for anything else.
    ///
    /// ```
    /// use archipel_kit::action_system::KeyStroke;
    /// assert!(KeyStroke::parse("ctrl shift A").is_some());
    /// assert!(KeyStroke::parse("hyper A").is_none());
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        let mut modifiers = Modifiers::NONE;
        let mut key = None;
        for token in text.split_whitespace() {
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" | "option" | "opt" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                "cmd" | "command" | "meta" => modifiers.mac_cmd = true,
                _ => {
                    if key.is_some() {
                        return None;
                    }
                    key = Some(parse_key(token)?);
                }
            }
        }
        Some(Self::new(key?, modifiers))
    }
}

fn parse_key(token: &str) -> Option<Key> {
    let lower = token.to_ascii_lowercase();
    let alias = match lower.as_str() {
        "esc" => Key::Escape,
        "return" => Key::Enter,
        "bksp" => Key::Backspace,
        "del" => Key::Delete,
        "ins" => Key::Insert,
        "pgup" => Key::PageUp,
        "pgdn" => Key::PageDown,
        "left" => Key::ArrowLeft,
        "right" => Key::ArrowRight,
        "up" => Key::ArrowUp,
        "down" => Key::ArrowDown,
        "/" => Key::Slash,
        "." => Key::Period,
        "," => Key::Comma,
        "[" => Key::OpenBracket,
        "]" => Key::CloseBracket,
        "-" => Key::Minus,
        "=" => Key::Equals,
        "'" => Key::Quote,
        ";" => Key::Semicolon,
        "\\" => Key::Backslash,
        "`" => Key::Backtick,
        _ => return Key::from_name(token),
    };
    Some(alias)
}

impl PartialEq for KeyStroke {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && modifier_bits(&self.modifiers) == modifier_bits(&other.modifiers)
    }
}

impl Eq for KeyStroke {}

impl Hash for KeyStroke {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        modifier_bits(&self.modifiers).hash(state);
    }
}

/// One or two key strokes: a shortcut such as `Ctrl+C`, or a chord such as `Ctrl+K, Ctrl+C`.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.KeyboardShortcut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct KeyboardShortcut {
    /// The first stroke (IntelliJ `getFirstKeyStroke`).
    pub first: KeyStroke,
    /// The second stroke of a chord (IntelliJ `getSecondKeyStroke`).
    pub second: Option<KeyStroke>,
}

impl KeyboardShortcut {
    /// A one-stroke shortcut.
    pub fn new(first: KeyStroke) -> Self {
        Self {
            first,
            second: None,
        }
    }

    /// A two-stroke shortcut.
    pub fn chord(first: KeyStroke, second: KeyStroke) -> Self {
        Self {
            first,
            second: Some(second),
        }
    }

    /// Parses one stroke (`"ctrl C"`) or two joined by `>` (`"ctrl K > ctrl C"`); see
    /// [`KeyStroke::parse`] (IntelliJ `KeyboardShortcut.fromString`).
    pub fn parse(text: &str) -> Option<Self> {
        match text.split_once('>') {
            Some((first, second)) => Some(Self::chord(
                KeyStroke::parse(first)?,
                KeyStroke::parse(second)?,
            )),
            None => KeyStroke::parse(text).map(Self::new),
        }
    }
}

/// A pointer button with a click count and modifiers, as a shortcut.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.MouseShortcut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MouseShortcut {
    /// The button.
    pub button: PointerButton,
    /// 1 for a click, 2 for a double click.
    pub click_count: u8,
    /// The modifiers held.
    pub modifiers: Modifiers,
}

impl MouseShortcut {
    /// A mouse shortcut; modifiers are normalized like [`KeyStroke`]'s.
    pub fn new(button: PointerButton, click_count: u8, modifiers: Modifiers) -> Self {
        Self {
            button,
            click_count,
            modifiers: normalize(modifiers),
        }
    }
}

/// A way to invoke an action from input: keys or the mouse.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.Shortcut` with its subclasses
/// `KeyboardShortcut` and `MouseShortcut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Shortcut {
    /// One or two key strokes.
    Keyboard(KeyboardShortcut),
    /// A mouse click.
    Mouse(MouseShortcut),
}

impl Shortcut {
    /// Parses a keyboard shortcut, see [`KeyboardShortcut::parse`].
    pub fn parse(text: &str) -> Option<Self> {
        KeyboardShortcut::parse(text).map(Self::Keyboard)
    }

    /// Whether this is a keyboard shortcut (IntelliJ `Shortcut.isKeyboard`).
    pub fn is_keyboard(&self) -> bool {
        matches!(self, Self::Keyboard(_))
    }

    /// Whether this shortcut begins with `stroke` (IntelliJ `Shortcut.startsWith`).
    pub fn starts_with(&self, stroke: &KeyStroke) -> bool {
        matches!(self, Self::Keyboard(shortcut) if shortcut.first == *stroke)
    }
}

impl From<KeyboardShortcut> for Shortcut {
    fn from(shortcut: KeyboardShortcut) -> Self {
        Self::Keyboard(shortcut)
    }
}

impl From<KeyStroke> for Shortcut {
    fn from(stroke: KeyStroke) -> Self {
        Self::Keyboard(KeyboardShortcut::new(stroke))
    }
}

/// A first stroke that was pressed and waits for a second one. The caller keeps it between
/// frames (IntelliJ keeps it inside `IdeKeyEventDispatcher`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingStroke {
    /// The first stroke.
    pub first: KeyStroke,
    /// When it was pressed, in seconds on any monotonic clock, such as `egui::InputState::time`.
    pub at: f64,
}

/// The outcome of feeding a key stroke to a [`Keymap`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyResolution {
    /// The stroke completes a shortcut: the ids of the actions bound to it, in id order.
    /// More than one means a conflict; run the first that is enabled (IntelliJ does the
    /// same, by update).
    Matched(Vec<String>),
    /// The stroke starts a two-stroke shortcut: keep the pending stroke and feed the next
    /// one within [`SECOND_KEYSTROKE_TIMEOUT`].
    AwaitSecond,
    /// The stroke is bound to nothing.
    NoMatch,
}

/// A table from action ids to shortcuts, with an optional parent it inherits from.
///
/// IntelliJ counterpart: `com.intellij.openapi.keymap.Keymap`. An action's own entry replaces
/// the parent's whole list for that id (including with an empty list, which unbinds it);
/// actions without an entry use the parent's.
///
/// ```
/// use archipel_kit::action_system::{IdeActions, Keymap, KeyResolution, KeyStroke, Os};
///
/// let keymap = Keymap::default_for(Os::Linux);
/// let copy = KeyStroke::parse("ctrl C").expect("valid stroke");
/// assert_eq!(
///     keymap.resolve(&copy),
///     KeyResolution::Matched(vec![IdeActions::ACTION_COPY.to_owned()]),
/// );
/// ```
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    name: String,
    parent: Option<Rc<Keymap>>,
    bindings: BTreeMap<String, Vec<Shortcut>>,
}

/// The defaults: action id, Windows and Linux shortcuts, macOS shortcuts. From the keytable
/// and keymap notes (IntelliJ `$default.xml` and `Mac OS X.xml`). Only the actions the kit
/// itself handles are listed.
///
/// macOS: the notes say the macOS keymap swaps Ctrl for Cmd, but the exact macOS keys of the
/// tool-window navigation actions (tabs, switcher, close) are not recorded, so entries that
/// need Ctrl there are left unbound on macOS rather than guessed.
const DEFAULTS: &[(&str, &[&str], &[&str])] = &[
    (
        "$Cut",
        &["ctrl X", "shift Delete"],
        &["cmd X", "shift Delete"],
    ),
    (
        "$Copy",
        &["ctrl C", "ctrl Insert"],
        &["cmd C", "cmd Insert"],
    ),
    (
        "$Paste",
        &["ctrl V", "shift Insert"],
        &["cmd V", "shift Insert"],
    ),
    (
        "PasteMultiple",
        &["ctrl shift V", "ctrl shift Insert"],
        &["shift cmd V", "shift cmd Insert"],
    ),
    ("$Undo", &["ctrl Z", "alt Backspace"], &["cmd Z"]),
    (
        "$Redo",
        &["ctrl shift Z", "alt shift Backspace"],
        &["shift cmd Z", "alt shift Backspace"],
    ),
    ("$SelectAll", &["ctrl A"], &["cmd A"]),
    (
        "$Delete",
        &["Delete"],
        &["Delete", "Backspace", "cmd Backspace"],
    ),
    ("Find", &["ctrl F", "alt F3"], &["cmd F"]),
    ("Replace", &["ctrl R"], &["cmd R"]),
    ("FindNext", &["F3", "ctrl L"], &["cmd G"]),
    (
        "FindPrevious",
        &["shift F3", "ctrl shift L"],
        &["shift cmd G"],
    ),
    ("FindInPath", &["ctrl shift F"], &["shift cmd F"]),
    ("ReplaceInPath", &["ctrl shift R"], &["shift cmd R"]),
    ("GotoLine", &["ctrl G"], &["cmd L"]),
    ("GotoAction", &["ctrl shift A"], &["shift cmd A"]),
    ("SelectNextOccurrence", &["alt J"], &["ctrl G"]),
    (
        "UnselectPreviousOccurrence",
        &["alt shift J"],
        &["ctrl shift G"],
    ),
    (
        "SelectAllOccurrences",
        &["ctrl alt shift J"],
        &["ctrl cmd G"],
    ),
    ("EditorEscape", &["Esc"], &["Esc"]),
    ("EditorEnter", &["Enter"], &["Enter"]),
    ("EditorTab", &["Tab"], &["Tab"]),
    ("EditorUnindentSelection", &["shift Tab"], &["shift Tab"]),
    ("EditorDuplicate", &["ctrl D"], &["cmd D"]),
    ("EditorDeleteLine", &["ctrl Y"], &["cmd Backspace"]),
    ("EditorPreviousWord", &["ctrl Left"], &["alt Left"]),
    ("EditorNextWord", &["ctrl Right"], &["alt Right"]),
    ("CommentByLineComment", &["ctrl /"], &["cmd /"]),
    ("ShowIntentionActions", &["alt Enter"], &["alt Enter"]),
    ("QuickJavaDoc", &["ctrl Q"], &["F1", "ctrl J"]),
    ("NextTab", &["alt Right"], &["alt Right"]),
    ("PreviousTab", &["alt Left"], &["alt Left"]),
    ("Switcher", &["ctrl Tab", "ctrl shift Tab"], &[]),
    ("CloseContent", &["ctrl F4"], &[]),
    ("CloseActiveTab", &["ctrl shift F4"], &[]),
    ("HideActiveWindow", &["shift Esc"], &["shift Esc"]),
    ("HideAllWindows", &["ctrl shift F12"], &[]),
    ("JumpToLastWindow", &["F12"], &["F12"]),
    ("FocusEditor", &["Esc"], &["Esc"]),
    ("ShowSearchHistory", &["alt Down"], &["alt Down"]),
    ("MaximizeToolWindow", &["ctrl shift '"], &[]),
];

impl Keymap {
    /// An empty keymap (IntelliJ `KeymapImpl` with no parent).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            bindings: BTreeMap::new(),
        }
    }

    /// An empty keymap that inherits from `parent` (IntelliJ `Keymap.getParent`).
    pub fn with_parent(name: impl Into<String>, parent: Rc<Keymap>) -> Self {
        Self {
            name: name.into(),
            parent: Some(parent),
            bindings: BTreeMap::new(),
        }
    }

    /// The default keymap for `os`: `"Default"` on Windows and Linux, `"macOS"` on macOS,
    /// with the entries for the actions the kit handles itself (copy, cut, paste, undo,
    /// redo, select all, find and replace, occurrences, escape, tab switching ...). See
    /// [`IdeActions`](crate::action_system::IdeActions) for the ids.
    pub fn default_for(os: Os) -> Self {
        let mac = os.is_mac();
        let mut keymap = Self::new(if mac { "macOS" } else { "Default" });
        for (id, windows_linux, macos) in DEFAULTS {
            let texts = if mac { macos } else { windows_linux };
            for text in *texts {
                if let Some(shortcut) = Shortcut::parse(text) {
                    keymap.add_shortcut(id, shortcut);
                }
            }
        }
        keymap
    }

    /// The keymap's name (IntelliJ `Keymap.getName`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The keymap it inherits from (IntelliJ `Keymap.getParent`).
    pub fn parent(&self) -> Option<&Rc<Keymap>> {
        self.parent.as_ref()
    }

    /// Adds a shortcut to an action, keeping the ones it already has, here or inherited
    /// (IntelliJ `Keymap.addShortcut`). Adding the same shortcut twice is a no-op.
    pub fn add_shortcut(&mut self, action_id: &str, shortcut: impl Into<Shortcut>) -> &mut Self {
        let shortcut = shortcut.into();
        let mut list = self.shortcuts(action_id);
        if !list.contains(&shortcut) {
            list.push(shortcut);
        }
        self.bindings.insert(action_id.to_owned(), list);
        self
    }

    /// Removes a shortcut from an action, also when it is inherited (IntelliJ
    /// `Keymap.removeShortcut`).
    pub fn remove_shortcut(&mut self, action_id: &str, shortcut: &Shortcut) -> &mut Self {
        let mut list = self.shortcuts(action_id);
        list.retain(|candidate| candidate != shortcut);
        self.bindings.insert(action_id.to_owned(), list);
        self
    }

    /// Replaces all shortcuts of an action; an empty list unbinds it.
    pub fn set_shortcuts(&mut self, action_id: &str, shortcuts: Vec<Shortcut>) -> &mut Self {
        self.bindings.insert(action_id.to_owned(), shortcuts);
        self
    }

    /// The shortcuts of an action, own or inherited (IntelliJ `Keymap.getShortcuts`).
    pub fn shortcuts(&self, action_id: &str) -> Vec<Shortcut> {
        match self.bindings.get(action_id) {
            Some(list) => list.clone(),
            None => self
                .parent
                .as_ref()
                .map(|parent| parent.shortcuts(action_id))
                .unwrap_or_default(),
        }
    }

    /// The ids of all actions with at least one shortcut (IntelliJ `Keymap.getActionIdList`).
    pub fn action_ids(&self) -> Vec<String> {
        self.effective()
            .into_iter()
            .filter(|(_, list)| !list.is_empty())
            .map(|(id, _)| id)
            .collect()
    }

    /// The ids of the actions whose shortcut is exactly `first` (and `second`, for a chord),
    /// in id order (IntelliJ `Keymap.getActionIds(KeyStroke)` and `getConflicts`).
    pub fn action_ids_for(&self, first: &KeyStroke, second: Option<&KeyStroke>) -> Vec<String> {
        self.effective()
            .into_iter()
            .filter(|(_, list)| {
                list.iter().any(|shortcut| match shortcut {
                    Shortcut::Keyboard(keyboard) => {
                        keyboard.first == *first && keyboard.second.as_ref() == second
                    }
                    Shortcut::Mouse(_) => false,
                })
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Whether some two-stroke shortcut begins with `first`.
    pub fn has_chord_starting_with(&self, first: &KeyStroke) -> bool {
        self.effective().iter().any(|(_, list)| {
            list.iter().any(|shortcut| {
                matches!(shortcut, Shortcut::Keyboard(keyboard)
                    if keyboard.first == *first && keyboard.second.is_some())
            })
        })
    }

    /// Looks a single stroke up: an action match, or the start of a chord, or nothing.
    /// Stateless; use [`feed`](Self::feed) for chords. An exact one-stroke match wins over
    /// the start of a chord.
    pub fn resolve(&self, stroke: &KeyStroke) -> KeyResolution {
        let ids = self.action_ids_for(stroke, None);
        if !ids.is_empty() {
            KeyResolution::Matched(ids)
        } else if self.has_chord_starting_with(stroke) {
            KeyResolution::AwaitSecond
        } else {
            KeyResolution::NoMatch
        }
    }

    /// Feeds the next stroke, tracking a half-typed chord in the caller's `pending` (the
    /// kit keeps no state). With a pending first stroke younger than
    /// [`SECOND_KEYSTROKE_TIMEOUT`], the stroke is tried as the second; if that fails, or the
    /// pending one expired, it is tried as a fresh first stroke. `now` is in seconds.
    pub fn feed(
        &self,
        pending: &mut Option<PendingStroke>,
        stroke: KeyStroke,
        now: f64,
    ) -> KeyResolution {
        let fresh = pending
            .take()
            .filter(|previous| now - previous.at <= SECOND_KEYSTROKE_TIMEOUT.as_secs_f64());
        if let Some(previous) = fresh {
            let ids = self.action_ids_for(&previous.first, Some(&stroke));
            if !ids.is_empty() {
                return KeyResolution::Matched(ids);
            }
        }
        let resolution = self.resolve(&stroke);
        if resolution == KeyResolution::AwaitSecond {
            *pending = Some(PendingStroke {
                first: stroke,
                at: now,
            });
        }
        resolution
    }

    /// The merged table: the parent's entries overridden by this keymap's, by id.
    fn effective(&self) -> BTreeMap<String, Vec<Shortcut>> {
        let mut merged = self
            .parent
            .as_ref()
            .map(|parent| parent.effective())
            .unwrap_or_default();
        for (id, list) in &self.bindings {
            merged.insert(id.clone(), list.clone());
        }
        merged
    }
}

/// Shortcut texts for menus and tooltips.
///
/// IntelliJ counterpart: `com.intellij.openapi.keymap.KeymapUtil`. Windows and Linux style is
/// `Ctrl+Shift+A` with chords as `Ctrl+K, Ctrl+C`; macOS style is the symbols `⌃⌥⇧⌘` in that
/// order followed by the key, with chords separated by a space, like `⇧⌘A`.
pub struct KeymapUtil;

impl KeymapUtil {
    /// The text of a keyboard shortcut (IntelliJ `KeymapUtil.getShortcutText`).
    ///
    /// ```
    /// use archipel_kit::action_system::{KeyboardShortcut, KeymapUtil};
    /// let shortcut = KeyboardShortcut::parse("ctrl shift A").expect("valid");
    /// assert_eq!(KeymapUtil::shortcut_text(&shortcut, false), "Ctrl+Shift+A");
    /// let mac = KeyboardShortcut::parse("shift cmd A").expect("valid");
    /// assert_eq!(KeymapUtil::shortcut_text(&mac, true), "⇧⌘A");
    /// ```
    pub fn shortcut_text(shortcut: &KeyboardShortcut, mac: bool) -> String {
        let first = Self::keystroke_text(&shortcut.first, mac);
        match &shortcut.second {
            None => first,
            Some(second) => {
                let separator = if mac { " " } else { ", " };
                format!("{first}{separator}{}", Self::keystroke_text(second, mac))
            }
        }
    }

    /// The text of one key stroke (IntelliJ `KeymapUtil.getKeystrokeText`).
    pub fn keystroke_text(stroke: &KeyStroke, mac: bool) -> String {
        format!(
            "{}{}",
            modifier_prefix(&stroke.modifiers, mac),
            key_text(stroke.key, mac)
        )
    }

    /// The text of any shortcut; mouse shortcuts read like `Ctrl+Click` or `Double Click`
    /// (IntelliJ `KeymapUtil.getShortcutText(Shortcut)`).
    pub fn text(shortcut: &Shortcut, mac: bool) -> String {
        match shortcut {
            Shortcut::Keyboard(keyboard) => Self::shortcut_text(keyboard, mac),
            Shortcut::Mouse(mouse) => {
                let button = match (mouse.button, mouse.click_count) {
                    (PointerButton::Middle, _) => "Middle Click",
                    (PointerButton::Secondary, _) => "Right Click",
                    (_, 2) => "Double Click",
                    _ => "Click",
                };
                format!("{}{button}", modifier_prefix(&mouse.modifiers, mac))
            }
        }
    }

    /// The text of the first keyboard shortcut of an action, for a menu's right column or a
    /// tooltip (IntelliJ `KeymapUtil.getFirstKeyboardShortcutText`).
    pub fn first_keyboard_shortcut_text(
        keymap: &Keymap,
        action_id: &str,
        mac: bool,
    ) -> Option<String> {
        keymap
            .shortcuts(action_id)
            .into_iter()
            .find_map(|shortcut| match shortcut {
                Shortcut::Keyboard(keyboard) => Some(Self::shortcut_text(&keyboard, mac)),
                Shortcut::Mouse(_) => None,
            })
    }
}

/// `Ctrl+Shift+` or `⌃⇧`: the modifiers in IntelliJ's order (Ctrl, Alt, Shift, Meta or Cmd).
fn modifier_prefix(modifiers: &Modifiers, mac: bool) -> String {
    let table = [
        (modifiers.ctrl, "Ctrl", "⌃"),
        (modifiers.alt, "Alt", "⌥"),
        (modifiers.shift, "Shift", "⇧"),
        (modifiers.mac_cmd, "Meta", "⌘"),
    ];
    let mut text = String::new();
    for (on, name, symbol) in table {
        if on {
            text.push_str(if mac { symbol } else { name });
            if !mac {
                text.push('+');
            }
        }
    }
    text
}

fn key_text(key: Key, mac: bool) -> String {
    let text = match key {
        Key::Escape => {
            if mac {
                "⎋"
            } else {
                "Esc"
            }
        }
        Key::Enter => {
            if mac {
                "↩"
            } else {
                "Enter"
            }
        }
        Key::Backspace => {
            if mac {
                "⌫"
            } else {
                "Backspace"
            }
        }
        Key::Delete => {
            if mac {
                "⌦"
            } else {
                "Delete"
            }
        }
        Key::Tab => {
            if mac {
                "⇥"
            } else {
                "Tab"
            }
        }
        Key::Space => {
            if mac {
                "␣"
            } else {
                "Space"
            }
        }
        Key::Insert => "Insert",
        Key::Home => {
            if mac {
                "↖"
            } else {
                "Home"
            }
        }
        Key::End => {
            if mac {
                "↘"
            } else {
                "End"
            }
        }
        Key::PageUp => {
            if mac {
                "⇞"
            } else {
                "PgUp"
            }
        }
        Key::PageDown => {
            if mac {
                "⇟"
            } else {
                "PgDn"
            }
        }
        Key::ArrowLeft => {
            if mac {
                "←"
            } else {
                "Left"
            }
        }
        Key::ArrowRight => {
            if mac {
                "→"
            } else {
                "Right"
            }
        }
        Key::ArrowUp => {
            if mac {
                "↑"
            } else {
                "Up"
            }
        }
        Key::ArrowDown => {
            if mac {
                "↓"
            } else {
                "Down"
            }
        }
        other => return other.symbol_or_name().to_owned(),
    };
    text.to_owned()
}
