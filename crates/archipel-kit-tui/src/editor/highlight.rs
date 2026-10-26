//! A small code highlighter for the terminal editor and the demos: Rust-like code, Markdown
//! and TOML. It implements the GUI kit's [`SyntaxHighlighter`], so anything that highlights for
//! the GUI editor (JSON, GraphQL, your tree-sitter adapter) works here too; this one only
//! saves the examples from needing a grammar.

use archipel_kit::editor::{Span, SyntaxHighlighter, TextAttributesKey};

/// What [`CodeHighlighter`] tokenizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CodeLanguage {
    /// Rust and C-like code: keywords, types, macros, strings, numbers, comments, attributes.
    #[default]
    Rust,
    /// Headings, code spans and fences, links, list bullets, quotes.
    Markdown,
    /// Tables, keys, strings, numbers, comments.
    Toml,
    /// No highlighting.
    Plain,
}

impl CodeLanguage {
    /// The language of a file name, by its extension.
    pub fn of_file(name: &str) -> Self {
        match name.rsplit('.').next() {
            Some("md" | "markdown") => CodeLanguage::Markdown,
            Some("toml") => CodeLanguage::Toml,
            Some("txt" | "lock" | "log") => CodeLanguage::Plain,
            _ => CodeLanguage::Rust,
        }
    }
}

/// The highlighter: a [`SyntaxHighlighter`] for a [`CodeLanguage`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CodeHighlighter {
    /// What to tokenize.
    pub language: CodeLanguage,
}

impl CodeHighlighter {
    /// A highlighter for `language`.
    pub fn new(language: CodeLanguage) -> Self {
        Self { language }
    }

    /// A highlighter for the file `name`.
    pub fn for_file(name: &str) -> Self {
        Self::new(CodeLanguage::of_file(name))
    }
}

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "def", "class", "import", "from", "function", "var", "null",
    "None",
];

fn push(out: &mut Vec<Span>, start: usize, end: usize, kind: TextAttributesKey) {
    if kind == TextAttributesKey::Plain || start >= end {
        return;
    }
    if let Some(last) = out.last_mut()
        && last.kind == kind
        && last.end == start
    {
        last.end = end;
        return;
    }
    out.push(Span { start, end, kind });
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b >= 0x80
}

fn is_ident(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

/// The end of the string that starts at `i` (on the quote), or the line end.
fn string_end(bytes: &[u8], i: usize, quote: u8) -> usize {
    let mut j = i + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            b if b == quote => return j + 1,
            _ => j += 1,
        }
    }
    bytes.len()
}

fn number_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
    {
        // `1..2` is a range, not a number with two dots.
        if bytes[i] == b'.' && bytes.get(i + 1) == Some(&b'.') {
            break;
        }
        i += 1;
    }
    i
}

fn rust_line(line: &str, state: &mut u32, out: &mut Vec<Span>) {
    use TextAttributesKey as K;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if *state > 0 {
            let start = i;
            while i < bytes.len() {
                if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                    i += 2;
                    *state -= 1;
                    if *state == 0 {
                        break;
                    }
                } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                    i += 2;
                    *state += 1;
                } else {
                    i += 1;
                }
            }
            push(out, start, i, K::Comment);
            continue;
        }
        let b = bytes[i];
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            push(out, i, bytes.len(), K::Comment);
            return;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            *state += 1;
            let start = i;
            i += 2;
            while i < bytes.len() && *state > 0 {
                if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                    i += 2;
                    *state -= 1;
                } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                    i += 2;
                    *state += 1;
                } else {
                    i += 1;
                }
            }
            push(out, start, i, K::Comment);
            continue;
        }
        if b == b'"' {
            let end = string_end(bytes, i, b'"');
            push(out, i, end, K::String);
            i = end;
            continue;
        }
        if b == b'\'' {
            // A lifetime (`'a`) or a character literal (`'x'`, `'\n'`).
            let is_char = bytes.get(i + 1) == Some(&b'\\')
                || (bytes.get(i + 2) == Some(&b'\'') && bytes.get(i + 1) != Some(&b'\''));
            if is_char {
                let end = string_end(bytes, i, b'\'');
                push(out, i, end, K::String);
                i = end;
            } else {
                let mut j = i + 1;
                while j < bytes.len() && is_ident(bytes[j]) {
                    j += 1;
                }
                push(out, i, j, K::Attribute);
                i = j.max(i + 1);
            }
            continue;
        }
        if b == b'#' && matches!(bytes.get(i + 1), Some(b'[') | Some(b'!')) {
            let end = line[i..].find(']').map_or(bytes.len(), |e| i + e + 1);
            push(out, i, end, K::Attribute);
            i = end;
            continue;
        }
        if b.is_ascii_digit() {
            let end = number_end(bytes, i);
            push(out, i, end, K::Number);
            i = end;
            continue;
        }
        if is_ident_start(b) {
            let mut j = i;
            while j < bytes.len() && is_ident(bytes[j]) {
                j += 1;
            }
            let word = &line[i..j];
            let kind = if KEYWORDS.contains(&word) {
                K::Keyword
            } else if bytes.get(j) == Some(&b'!') && bytes.get(j + 1) != Some(&b'=') {
                K::Attribute
            } else if word.as_bytes()[0].is_ascii_uppercase() {
                K::Tag
            } else if bytes.get(j) == Some(&b'(') {
                K::Property
            } else {
                K::Plain
            };
            push(out, i, j, kind);
            i = j;
            continue;
        }
        if b.is_ascii_punctuation() {
            push(out, i, i + 1, K::Punctuation);
        }
        i += 1;
    }
}

fn markdown_line(line: &str, state: &mut u32, out: &mut Vec<Span>) {
    use TextAttributesKey as K;
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        *state ^= 1;
        push(out, indent, line.len(), K::Punctuation);
        return;
    }
    if *state == 1 {
        push(out, 0, line.len(), K::String);
        return;
    }
    if trimmed.starts_with('#') {
        push(out, indent, line.len(), K::Keyword);
        return;
    }
    if trimmed.starts_with('>') {
        push(out, indent, line.len(), K::Comment);
        return;
    }
    let bytes = line.as_bytes();
    let mut i = indent;
    // List bullets.
    let rest = &line[indent..];
    if rest.starts_with("- ") || rest.starts_with("* ") || rest.starts_with("+ ") {
        push(out, i, i + 1, K::Punctuation);
        i += 1;
    } else if let Some(dot) = rest.find(". ")
        && dot > 0
        && dot <= 3
        && rest[..dot].bytes().all(|b| b.is_ascii_digit())
    {
        push(out, i, i + dot + 1, K::Punctuation);
        i += dot + 1;
    }
    while i < bytes.len() {
        match bytes[i] {
            b'`' => {
                let end = line[i + 1..].find('`').map_or(bytes.len(), |e| i + e + 2);
                push(out, i, end, K::String);
                i = end;
            }
            b'[' => {
                if let Some(close) = line[i..].find("](")
                    && let Some(paren) = line[i + close..].find(')')
                {
                    push(out, i, i + close + 1, K::Property);
                    push(out, i + close + 1, i + close + paren + 1, K::Attribute);
                    i += close + paren + 1;
                } else {
                    i += 1;
                }
            }
            b'|' => {
                push(out, i, i + 1, K::Punctuation);
                i += 1;
            }
            b'*' | b'_' if bytes.get(i + 1) == Some(&bytes[i]) => {
                push(out, i, i + 2, K::Punctuation);
                i += 2;
            }
            _ => i += 1,
        }
    }
}

fn toml_line(line: &str, _state: &mut u32, out: &mut Vec<Span>) {
    use TextAttributesKey as K;
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if trimmed.starts_with('#') {
        push(out, indent, line.len(), K::Comment);
        return;
    }
    if trimmed.starts_with('[') {
        push(out, indent, line.len(), K::Tag);
        return;
    }
    let bytes = line.as_bytes();
    let eq = line.find('=');
    if let Some(eq) = eq {
        push(out, indent, line[..eq].trim_end().len(), K::Property);
        push(out, eq, eq + 1, K::Punctuation);
    }
    let mut i = eq.map_or(0, |e| e + 1);
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'#' {
            push(out, i, bytes.len(), K::Comment);
            return;
        }
        if b == b'"' || b == b'\'' {
            let end = string_end(bytes, i, b);
            push(out, i, end, K::String);
            i = end;
        } else if b.is_ascii_digit() {
            let end = number_end(bytes, i);
            push(out, i, end, K::Number);
            i = end;
        } else if is_ident_start(b) {
            let mut j = i;
            while j < bytes.len() && is_ident(bytes[j]) {
                j += 1;
            }
            if matches!(&line[i..j], "true" | "false") {
                push(out, i, j, K::Keyword);
            }
            i = j;
        } else {
            if matches!(b, b'{' | b'}' | b'[' | b']' | b',') {
                push(out, i, i + 1, K::Punctuation);
            }
            i += 1;
        }
    }
}

impl SyntaxHighlighter for CodeHighlighter {
    fn highlight_line(&self, line: &str, state: &mut u32, out: &mut Vec<Span>) {
        match self.language {
            CodeLanguage::Rust => rust_line(line, state, out),
            CodeLanguage::Markdown => markdown_line(line, state, out),
            CodeLanguage::Toml => toml_line(line, state, out),
            CodeLanguage::Plain => {}
        }
    }

    fn stateless(&self) -> bool {
        matches!(self.language, CodeLanguage::Toml | CodeLanguage::Plain)
    }

    fn name(&self) -> &'static str {
        match self.language {
            CodeLanguage::Rust => "archipel-tui-rust",
            CodeLanguage::Markdown => "archipel-tui-markdown",
            CodeLanguage::Toml => "archipel-tui-toml",
            CodeLanguage::Plain => "archipel-tui-plain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(language: CodeLanguage, line: &str) -> Vec<(String, TextAttributesKey)> {
        let mut out = Vec::new();
        CodeHighlighter::new(language).highlight_line(line, &mut 0, &mut out);
        out.iter()
            .map(|s| (line[s.start..s.end].to_owned(), s.kind))
            .collect()
    }

    #[test]
    fn rust_tokens_get_their_kinds() {
        let s = spans(CodeLanguage::Rust, "pub fn main() { let x = 42; // done");
        assert!(s.contains(&("pub".into(), TextAttributesKey::Keyword)));
        assert!(s.contains(&("main".into(), TextAttributesKey::Property)));
        assert!(s.contains(&("42".into(), TextAttributesKey::Number)));
        assert!(s.contains(&("// done".into(), TextAttributesKey::Comment)));
    }

    #[test]
    fn a_block_comment_carries_its_state_to_the_next_line() {
        let mut state = 0;
        let mut out = Vec::new();
        CodeHighlighter::default().highlight_line("a /* x", &mut state, &mut out);
        assert_eq!(state, 1);
        out.clear();
        CodeHighlighter::default().highlight_line("y */ b", &mut state, &mut out);
        assert_eq!(state, 0);
        assert_eq!(out[0].kind, TextAttributesKey::Comment);
    }

    #[test]
    fn markdown_headings_and_code_spans() {
        let s = spans(CodeLanguage::Markdown, "# Title");
        assert_eq!(s, [("# Title".into(), TextAttributesKey::Keyword)]);
        let s = spans(CodeLanguage::Markdown, "use `Editor` now");
        assert!(s.contains(&("`Editor`".into(), TextAttributesKey::String)));
    }

    #[test]
    fn toml_keys_and_tables() {
        let s = spans(CodeLanguage::Toml, "name = \"kit\"");
        assert!(s.contains(&("name".into(), TextAttributesKey::Property)));
        assert!(s.contains(&("\"kit\"".into(), TextAttributesKey::String)));
        assert_eq!(
            spans(CodeLanguage::Toml, "[package]"),
            [("[package]".into(), TextAttributesKey::Tag)]
        );
    }

    #[test]
    fn no_input_panics() {
        for line in ["", "'", "\"", "#[", "/*", "'\\", "`", "[](", "é\"é", "1..2"] {
            for language in [
                CodeLanguage::Rust,
                CodeLanguage::Markdown,
                CodeLanguage::Toml,
            ] {
                let _ = spans(language, line);
            }
        }
    }
}
