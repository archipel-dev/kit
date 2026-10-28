use serde::{Deserialize, Serialize};

/// How a platform writes shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Notation {
    /// `Ctrl+Shift+A`
    Text,
    /// `⇧⌘A`
    Mac,
}

/// One key press with modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chord {
    /// Control.
    pub ctrl: bool,
    /// Alt (Option on macOS).
    pub alt: bool,
    /// Shift.
    pub shift: bool,
    /// Meta (Command on macOS, the Windows key elsewhere).
    pub meta: bool,
    /// The key name as the keymap spells it: `A`, `F10`, `Enter`, `Page Up`, `\`.
    pub key: String,
}

impl Chord {
    /// Parses `Ctrl+Shift+A` (`Cmd` and `Meta` set meta).
    pub fn parse(text: &str) -> Chord {
        let mut chord = Chord {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            key: String::new(),
        };
        let mut rest = text.trim();
        while let Some((head, tail)) = rest.split_once('+') {
            // A literal `+` key is written `Ctrl++`.
            if tail.is_empty() {
                break;
            }
            match head.trim() {
                "Ctrl" | "Control" => chord.ctrl = true,
                "Alt" | "Option" => chord.alt = true,
                "Shift" => chord.shift = true,
                "Meta" | "Cmd" | "Command" | "Super" => chord.meta = true,
                _ => break,
            }
            rest = tail;
        }
        chord.key = rest.trim().to_owned();
        chord
    }

    /// The chord as text in `notation`.
    pub fn label(&self, notation: Notation) -> String {
        let mut out = String::new();
        match notation {
            Notation::Mac => {
                for (on, symbol) in [
                    (self.ctrl, "⌃"),
                    (self.alt, "⌥"),
                    (self.shift, "⇧"),
                    (self.meta, "⌘"),
                ] {
                    if on {
                        out.push_str(symbol);
                    }
                }
                out.push_str(match self.key.as_str() {
                    "Enter" => "↩",
                    "Backspace" => "⌫",
                    "Delete" => "⌦",
                    "Esc" => "⎋",
                    "Tab" => "⇥",
                    "Up" => "↑",
                    "Down" => "↓",
                    "Left" => "←",
                    "Right" => "→",
                    "Page Up" => "⇞",
                    "Page Down" => "⇟",
                    "Home" => "↖",
                    "End" => "↘",
                    other => other,
                });
            }
            Notation::Text => {
                for (on, name) in [
                    (self.ctrl, "Ctrl+"),
                    (self.alt, "Alt+"),
                    (self.shift, "Shift+"),
                    (self.meta, "Meta+"),
                ] {
                    if on {
                        out.push_str(name);
                    }
                }
                out.push_str(&self.key);
            }
        }
        out
    }
}

/// A keyboard shortcut as data. The renderer formats it ([`Shortcut::label`]) so the same
/// props read `Ctrl+Shift+A` on Linux and `⇧⌘A` on macOS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Shortcut {
    /// One chord or a two-stroke sequence (`Ctrl+K, Ctrl+1`).
    Keys(Vec<Chord>),
    /// A modifier pressed twice: `Double Shift`.
    DoubleModifier(String),
    /// Anything else, shown as is.
    Other(String),
}

impl Shortcut {
    /// A shortcut from text such as `Ctrl+Shift+A` or `Ctrl+K, Ctrl+1`; `double Shift`
    /// becomes [`Shortcut::DoubleModifier`].
    pub fn keys(text: &str) -> Shortcut {
        let trimmed = text.trim();
        if trimmed.len() > "double ".len()
            && trimmed[.."double ".len()].eq_ignore_ascii_case("double ")
        {
            return Shortcut::DoubleModifier(trimmed["double ".len()..].to_owned());
        }
        Shortcut::Keys(text.split(", ").map(Chord::parse).collect())
    }

    /// The text of the shortcut in `notation`.
    pub fn label(&self, notation: Notation) -> String {
        match self {
            Shortcut::Keys(chords) => chords
                .iter()
                .map(|chord| chord.label(notation))
                .collect::<Vec<_>>()
                .join(", "),
            Shortcut::DoubleModifier(name) => match (notation, name.as_str()) {
                (Notation::Mac, "Shift") => "Double ⇧".to_owned(),
                (Notation::Mac, "Ctrl") | (Notation::Mac, "Control") => "Double ⌃".to_owned(),
                (Notation::Mac, "Alt") => "Double ⌥".to_owned(),
                (_, name) => format!("Double {name}"),
            },
            Shortcut::Other(text) => text.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_the_notation() {
        let s = Shortcut::keys("Ctrl+Shift+A");
        assert_eq!(s.label(Notation::Text), "Ctrl+Shift+A");
        assert_eq!(s.label(Notation::Mac), "⌃⇧A");
        assert_eq!(Shortcut::keys("Alt+Enter").label(Notation::Mac), "⌥↩");
        assert_eq!(
            Shortcut::keys("double Shift").label(Notation::Text),
            "Double Shift"
        );
        assert_eq!(
            Shortcut::keys("Ctrl+K, Ctrl+1").label(Notation::Text),
            "Ctrl+K, Ctrl+1"
        );
        assert_eq!(Chord::parse("Ctrl++").key, "+");
    }
}
