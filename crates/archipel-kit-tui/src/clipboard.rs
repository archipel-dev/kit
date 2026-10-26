//! The clipboard: an in-app buffer that is also published to the terminal's clipboard with
//! the OSC 52 escape sequence, which most modern terminals (kitty, WezTerm, iTerm2, Ghostty,
//! Alacritty, Windows Terminal, tmux with `set-clipboard on`) accept. Pasting from the
//! system clipboard arrives as a bracketed paste ([`crate::Event::Paste`]); the terminal
//! sends it when the user pastes (Ctrl+Shift+V or Cmd+V), so the kit never reads the
//! clipboard itself.

/// The clipboard of one app.
#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    text: String,
    whole_line: bool,
    outbox: Option<String>,
}

impl Clipboard {
    /// An empty clipboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies `text`: keeps it for in-app paste and queues the OSC 52 sequence.
    pub fn set(&mut self, text: impl Into<String>) {
        self.set_content(text, false);
    }

    /// Copies `text`; `whole_line` says it is whole lines copied from an empty selection
    /// (the editor then pastes it above the caret line, like IntelliJ).
    pub fn set_content(&mut self, text: impl Into<String>, whole_line: bool) {
        self.text = text.into();
        self.whole_line = whole_line;
        self.outbox = Some(osc52(&self.text));
    }

    /// Whether the text is whole lines copied from an empty selection and has not changed
    /// since.
    pub fn is_whole_line(&self) -> bool {
        self.whole_line
    }

    /// The last copied text (or pasted by the terminal into the app).
    pub fn get(&self) -> &str {
        &self.text
    }

    /// Records text that arrived from the system clipboard (a bracketed paste) so that an
    /// in-app paste action pastes the same.
    pub fn remember(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text != self.text {
            self.whole_line = false;
        }
        self.text = text;
    }

    /// The OSC 52 sequence to write to the terminal, if a copy is waiting.
    pub fn take_sequence(&mut self) -> Option<String> {
        self.outbox.take()
    }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding.
pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// The OSC 52 sequence that sets the system clipboard to `text`.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn copy_queues_one_sequence() {
        let mut clipboard = Clipboard::new();
        clipboard.set("hi");
        assert_eq!(clipboard.get(), "hi");
        assert_eq!(
            clipboard.take_sequence().as_deref(),
            Some("\x1b]52;c;aGk=\x07")
        );
        assert_eq!(clipboard.take_sequence(), None);
    }
}
