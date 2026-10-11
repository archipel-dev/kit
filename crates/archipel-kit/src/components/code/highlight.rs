//! Token highlighting: the [`Highlighter`] trait and the built-in tokenizers.

use crate::theme::EditorColors;
use egui::Color32;

/// What a piece of code is, which decides its color (from [`EditorColors`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TokenKind {
    /// Unstyled text.
    #[default]
    Plain,
    /// `if`, `function`, `true`, `null` …
    Keyword,
    /// String literals.
    String,
    /// Numbers.
    Number,
    /// Comments.
    Comment,
    /// Brackets, commas, colons and operators.
    Punctuation,
    /// Property names: JSON keys, member access, arguments.
    Property,
    /// Markup tags and type names.
    Tag,
    /// Markup attributes and directives.
    Attribute,
    /// Variables, entities.
    Variable,
    /// Invalid or unterminated input.
    Error,
    /// A `{{variable}}` the caller can resolve (see [`TemplateHighlighter`](super::TemplateHighlighter)).
    Resolved,
    /// A `{{variable}}` nothing defines: drawn in the error color with a dotted underline
    /// by [`TextField`](crate::components::TextField).
    Unresolved,
    /// A `{{variable}}` holding a secret, styled apart so it is not mistaken for plain data.
    Secret,
}

impl TokenKind {
    /// The color of this kind in `colors`.
    pub fn color(self, colors: &EditorColors) -> Color32 {
        match self {
            TokenKind::Plain => colors.text,
            TokenKind::Keyword => colors.keyword,
            TokenKind::String => colors.string,
            TokenKind::Number => colors.number,
            TokenKind::Comment => colors.comment,
            TokenKind::Punctuation => colors.punctuation,
            TokenKind::Property => colors.property,
            TokenKind::Tag => colors.tag,
            TokenKind::Attribute => colors.attribute,
            TokenKind::Variable => colors.variable,
            TokenKind::Error | TokenKind::Unresolved => colors.invalid,
            TokenKind::Resolved => colors.variable,
            TokenKind::Secret => colors.string,
        }
    }
}

/// A styled byte range of a line. Spans are sorted and don't overlap; the gaps are
/// [`TokenKind::Plain`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// First byte, relative to the start of the line (or text).
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
    /// What it is.
    pub kind: TokenKind,
}

/// Maps code to styled [`Span`]s. Implement it to plug your own tokenizer (a tree-sitter
/// query, a language server's semantic tokens) into [`CodeView`](super::CodeView) and
/// [`CodeEditor`](super::CodeEditor).
///
/// The views ask for one line at a time, so only visible lines are tokenized.
/// Constructs that span lines (block comments, template strings) carry a `state` from
/// each line to the next: `0` at the start of the text, and whatever the tokenizer wants
/// afterwards. A tokenizer without multi-line constructs overrides
/// [`stateless`](Highlighter::stateless) so the views never scan the lines above the
/// visible ones.
///
/// ```
/// use archipel_kit::components::code::{Highlighter, Span, TokenKind};
///
/// /// Marks `TODO` in comments.
/// struct Todo;
/// impl Highlighter for Todo {
///     fn highlight_line(&self, line: &str, _state: &mut u32, out: &mut Vec<Span>) {
///         for (start, word) in line.match_indices("TODO") {
///             out.push(Span { start, end: start + word.len(), kind: TokenKind::Keyword });
///         }
///     }
///     fn stateless(&self) -> bool { true }
/// }
/// let mut spans = Vec::new();
/// Todo.highlight_line("// TODO: later", &mut 0, &mut spans);
/// assert_eq!(spans.len(), 1);
/// ```
pub trait Highlighter {
    /// Appends the spans of `line` (without its line break) to `out`, in order.
    /// `state` is `0` for the first line and the value this method left for the previous
    /// one afterwards; set it when a construct continues on the next line.
    fn highlight_line(&self, line: &str, state: &mut u32, out: &mut Vec<Span>);

    /// `true` when `state` is always `0`, which makes jumping to any line free.
    fn stateless(&self) -> bool {
        false
    }

    /// Identifies the tokenizer: cached results are dropped when it changes.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// The spans of a whole text, with byte offsets into `text`. Defaults to running
    /// [`highlight_line`](Highlighter::highlight_line) over its lines; override it when
    /// the tokenizer wants the whole text (a syntax tree).
    fn highlight_text(&self, text: &str, out: &mut Vec<Span>) {
        let mut state = 0;
        let mut line_spans = Vec::new();
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let content = line.strip_suffix('\n').unwrap_or(line);
            let content = content.strip_suffix('\r').unwrap_or(content);
            line_spans.clear();
            self.highlight_line(content, &mut state, &mut line_spans);
            out.extend(line_spans.iter().map(|s| Span {
                start: s.start + offset,
                end: s.end + offset,
                kind: s.kind,
            }));
            offset += line.len();
        }
    }
}

fn push(out: &mut Vec<Span>, start: usize, end: usize, kind: TokenKind) {
    if kind == TokenKind::Plain || start >= end {
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
    b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 0x80
}

fn is_ident(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Length in bytes of the char at `i` (`i` is a char boundary).
fn char_len(line: &str, i: usize) -> usize {
    line[i..].chars().next().map_or(1, char::len_utf8)
}

/// End of a quoted string starting at `i` (on the quote). `Some(end)` past the closing
/// quote, `None` when the line ends first.
fn quoted_end(bytes: &[u8], i: usize, quote: u8) -> Option<usize> {
    let mut j = i + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            b if b == quote => return Some(j + 1),
            _ => j += 1,
        }
    }
    None
}

/// End of a number starting at `i`: digits, one dot, exponent, hex/underscore/suffix.
fn number_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            b if b.is_ascii_alphanumeric() || b == b'_' || b == b'.' => i += 1,
            b'+' | b'-' if matches!(bytes[i - 1], b'e' | b'E') => i += 1,
            _ => break,
        }
    }
    i
}

/// Text without highlighting.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainText;

impl Highlighter for PlainText {
    fn highlight_line(&self, _: &str, _: &mut u32, _: &mut Vec<Span>) {}
    fn stateless(&self) -> bool {
        true
    }
}

/// JSON: keys as properties, strings, numbers, `true`/`false`/`null`, punctuation, and
/// anything else as an error.
#[derive(Debug, Clone, Copy, Default)]
pub struct Json;

impl Highlighter for Json {
    fn highlight_line(&self, line: &str, _: &mut u32, out: &mut Vec<Span>) {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                b' ' | b'\t' | b'\r' => i += 1,
                b'"' => {
                    let (end, closed) = match quoted_end(bytes, i, b'"') {
                        Some(end) => (end, true),
                        None => (bytes.len(), false),
                    };
                    let next = skip_ws(bytes, end);
                    let kind = if !closed {
                        TokenKind::Error
                    } else if bytes.get(next) == Some(&b':') {
                        TokenKind::Property
                    } else {
                        TokenKind::String
                    };
                    push(out, i, end, kind);
                    i = end;
                }
                b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                    push(out, i, i + 1, TokenKind::Punctuation);
                    i += 1;
                }
                b'-' | b'0'..=b'9' => {
                    let end = number_end(bytes, i + 1);
                    let text = &line[i..end];
                    let valid = text.parse::<f64>().is_ok();
                    push(
                        out,
                        i,
                        end,
                        if valid {
                            TokenKind::Number
                        } else {
                            TokenKind::Error
                        },
                    );
                    i = end;
                }
                b if b.is_ascii_alphabetic() => {
                    let mut end = i;
                    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
                        end += 1;
                    }
                    let kind = if matches!(&line[i..end], "true" | "false" | "null") {
                        TokenKind::Keyword
                    } else {
                        TokenKind::Error
                    };
                    push(out, i, end, kind);
                    i = end;
                }
                _ => {
                    let len = char_len(line, i);
                    push(out, i, i + len, TokenKind::Error);
                    i += len;
                }
            }
        }
    }

    fn stateless(&self) -> bool {
        true
    }
}

const JS_KEYWORDS: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

const JS_BLOCK_COMMENT: u32 = 1;
const JS_TEMPLATE: u32 = 2;

/// JavaScript (and JSON-like scripts): keywords, strings, template literals, numbers,
/// comments, member names and punctuation. Regular expression literals are not tokenized.
#[derive(Debug, Clone, Copy, Default)]
pub struct JavaScript;

impl Highlighter for JavaScript {
    fn highlight_line(&self, line: &str, state: &mut u32, out: &mut Vec<Span>) {
        let bytes = line.as_bytes();
        let mut i = 0;
        // Continue a construct from the previous line.
        match *state {
            JS_BLOCK_COMMENT => match line.find("*/") {
                Some(at) => {
                    push(out, 0, at + 2, TokenKind::Comment);
                    i = at + 2;
                    *state = 0;
                }
                None => {
                    push(out, 0, line.len(), TokenKind::Comment);
                    return;
                }
            },
            JS_TEMPLATE => match template_end(bytes, 0) {
                Some(end) => {
                    push(out, 0, end, TokenKind::String);
                    i = end;
                    *state = 0;
                }
                None => {
                    push(out, 0, line.len(), TokenKind::String);
                    return;
                }
            },
            _ => {}
        }
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                b' ' | b'\t' | b'\r' => i += 1,
                b'/' if bytes.get(i + 1) == Some(&b'/') => {
                    push(out, i, bytes.len(), TokenKind::Comment);
                    return;
                }
                b'/' if bytes.get(i + 1) == Some(&b'*') => match line[i + 2..].find("*/") {
                    Some(at) => {
                        push(out, i, i + 2 + at + 2, TokenKind::Comment);
                        i += 2 + at + 2;
                    }
                    None => {
                        push(out, i, bytes.len(), TokenKind::Comment);
                        *state = JS_BLOCK_COMMENT;
                        return;
                    }
                },
                b'"' | b'\'' => match quoted_end(bytes, i, b) {
                    Some(end) => {
                        push(out, i, end, TokenKind::String);
                        i = end;
                    }
                    None => {
                        push(out, i, bytes.len(), TokenKind::Error);
                        return;
                    }
                },
                b'`' => match template_end(bytes, i + 1) {
                    Some(end) => {
                        push(out, i, end, TokenKind::String);
                        i = end;
                    }
                    None => {
                        push(out, i, bytes.len(), TokenKind::String);
                        *state = JS_TEMPLATE;
                        return;
                    }
                },
                b'0'..=b'9' => {
                    let end = number_end(bytes, i);
                    push(out, i, end, TokenKind::Number);
                    i = end;
                }
                b'.' if bytes.get(i + 1).is_some_and(u8::is_ascii_digit) => {
                    let end = number_end(bytes, i + 1);
                    push(out, i, end, TokenKind::Number);
                    i = end;
                }
                b if is_ident_start(b) => {
                    let mut end = i;
                    while end < bytes.len() && is_ident(bytes[end]) {
                        end += 1;
                    }
                    let word = &line[i..end];
                    let member = line[..i].trim_end().ends_with('.');
                    let kind = if member {
                        TokenKind::Property
                    } else if JS_KEYWORDS.contains(&word) {
                        TokenKind::Keyword
                    } else if bytes.get(skip_ws(bytes, end)) == Some(&b'(') {
                        TokenKind::Variable
                    } else {
                        TokenKind::Plain
                    };
                    push(out, i, end, kind);
                    i = end;
                }
                _ if b.is_ascii_punctuation() => {
                    push(out, i, i + 1, TokenKind::Punctuation);
                    i += 1;
                }
                _ => i += char_len(line, i),
            }
        }
    }
}

/// End (past the closing backtick) of a template literal body starting at `i`.
fn template_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'`' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

const GQL_KEYWORDS: &[&str] = &[
    "query",
    "mutation",
    "subscription",
    "fragment",
    "on",
    "type",
    "interface",
    "union",
    "enum",
    "input",
    "scalar",
    "extend",
    "schema",
    "directive",
    "implements",
    "repeatable",
    "true",
    "false",
    "null",
];

const GQL_BLOCK_STRING: u32 = 1;

/// GraphQL queries and schemas: keywords, `$variables`, `@directives`, argument and
/// alias names as properties, type names as tags, strings (including `"""` blocks),
/// numbers and `#` comments.
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphQl;

impl Highlighter for GraphQl {
    fn highlight_line(&self, line: &str, state: &mut u32, out: &mut Vec<Span>) {
        let bytes = line.as_bytes();
        let mut i = 0;
        if *state == GQL_BLOCK_STRING {
            match line.find("\"\"\"") {
                Some(at) => {
                    push(out, 0, at + 3, TokenKind::String);
                    i = at + 3;
                    *state = 0;
                }
                None => {
                    push(out, 0, line.len(), TokenKind::String);
                    return;
                }
            }
        }
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                b' ' | b'\t' | b'\r' => i += 1,
                b'#' => {
                    push(out, i, bytes.len(), TokenKind::Comment);
                    return;
                }
                b'"' if line[i..].starts_with("\"\"\"") => match line[i + 3..].find("\"\"\"") {
                    Some(at) => {
                        push(out, i, i + 3 + at + 3, TokenKind::String);
                        i += 3 + at + 3;
                    }
                    None => {
                        push(out, i, bytes.len(), TokenKind::String);
                        *state = GQL_BLOCK_STRING;
                        return;
                    }
                },
                b'"' => match quoted_end(bytes, i, b'"') {
                    Some(end) => {
                        push(out, i, end, TokenKind::String);
                        i = end;
                    }
                    None => {
                        push(out, i, bytes.len(), TokenKind::Error);
                        return;
                    }
                },
                b'-' | b'0'..=b'9' => {
                    let end = number_end(bytes, i + 1);
                    push(out, i, end, TokenKind::Number);
                    i = end;
                }
                b'$' | b'@' => {
                    let mut end = i + 1;
                    while end < bytes.len() && is_ident(bytes[end]) && bytes[end] != b'$' {
                        end += 1;
                    }
                    let kind = if b == b'$' {
                        TokenKind::Variable
                    } else {
                        TokenKind::Attribute
                    };
                    push(
                        out,
                        i,
                        end,
                        if end == i + 1 { TokenKind::Error } else { kind },
                    );
                    i = end;
                }
                b'.' if line[i..].starts_with("...") => {
                    push(out, i, i + 3, TokenKind::Punctuation);
                    i += 3;
                }
                b if is_ident_start(b) => {
                    let mut end = i;
                    while end < bytes.len() && is_ident(bytes[end]) && bytes[end] != b'$' {
                        end += 1;
                    }
                    let word = &line[i..end];
                    let kind = if GQL_KEYWORDS.contains(&word) {
                        TokenKind::Keyword
                    } else if bytes.get(skip_ws(bytes, end)) == Some(&b':') {
                        TokenKind::Property
                    } else if word.starts_with(|c: char| c.is_uppercase()) {
                        TokenKind::Tag
                    } else {
                        TokenKind::Plain
                    };
                    push(out, i, end, kind);
                    i = end;
                }
                _ if b.is_ascii_punctuation() => {
                    push(out, i, i + 1, TokenKind::Punctuation);
                    i += 1;
                }
                _ => i += char_len(line, i),
            }
        }
    }
}

const XML_COMMENT: u32 = 1;
const XML_TAG: u32 = 2;
const XML_DQ: u32 = 3;
const XML_SQ: u32 = 4;
const XML_CDATA: u32 = 5;

/// XML and HTML: tags, attributes, attribute values, entities, comments, CDATA and
/// `<!DOCTYPE>`/`<?xml?>` declarations. The content of `<script>` and `<style>` is plain.
#[derive(Debug, Clone, Copy, Default)]
pub struct Xml;

impl Highlighter for Xml {
    fn highlight_line(&self, line: &str, state: &mut u32, out: &mut Vec<Span>) {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match *state {
                XML_COMMENT | XML_CDATA => {
                    let (close, kind) = if *state == XML_COMMENT {
                        ("-->", TokenKind::Comment)
                    } else {
                        ("]]>", TokenKind::String)
                    };
                    match line[i..].find(close) {
                        Some(at) => {
                            push(out, i, i + at + 3, kind);
                            i += at + 3;
                            *state = 0;
                        }
                        None => {
                            push(out, i, bytes.len(), kind);
                            return;
                        }
                    }
                }
                XML_DQ | XML_SQ => {
                    let quote = if *state == XML_DQ { b'"' } else { b'\'' };
                    match bytes[i..].iter().position(|&b| b == quote) {
                        Some(at) => {
                            push(out, i, i + at + 1, TokenKind::String);
                            i += at + 1;
                            *state = XML_TAG;
                        }
                        None => {
                            push(out, i, bytes.len(), TokenKind::String);
                            return;
                        }
                    }
                }
                XML_TAG => {
                    let b = bytes[i];
                    match b {
                        b' ' | b'\t' | b'\r' => i += 1,
                        b'>' => {
                            push(out, i, i + 1, TokenKind::Tag);
                            i += 1;
                            *state = 0;
                        }
                        b'/' | b'?' if bytes.get(i + 1) == Some(&b'>') => {
                            push(out, i, i + 2, TokenKind::Tag);
                            i += 2;
                            *state = 0;
                        }
                        b'=' => {
                            push(out, i, i + 1, TokenKind::Punctuation);
                            i += 1;
                        }
                        b'"' | b'\'' => {
                            push(out, i, i + 1, TokenKind::String);
                            i += 1;
                            *state = if b == b'"' { XML_DQ } else { XML_SQ };
                        }
                        _ => {
                            let mut end = i;
                            while end < bytes.len()
                                && !matches!(
                                    bytes[end],
                                    b' ' | b'\t' | b'\r' | b'=' | b'>' | b'/' | b'"' | b'\''
                                )
                            {
                                end += 1;
                            }
                            if end == i {
                                end += char_len(line, i);
                                push(out, i, end, TokenKind::Error);
                            } else {
                                push(out, i, end, TokenKind::Attribute);
                            }
                            i = end;
                        }
                    }
                }
                _ => {
                    // Text content.
                    let rest = &line[i..];
                    if rest.starts_with("<!--") {
                        *state = XML_COMMENT;
                        push(out, i, i + 4, TokenKind::Comment);
                        i += 4;
                    } else if rest.starts_with("<![CDATA[") {
                        *state = XML_CDATA;
                        push(out, i, i + 9, TokenKind::String);
                        i += 9;
                    } else if rest.starts_with('<') {
                        let open_len = if rest.starts_with("</") { 2 } else { 1 };
                        let special = rest[open_len..].starts_with(['!', '?']);
                        let name_start = i + open_len + usize::from(special);
                        let mut end = name_start;
                        while end < bytes.len()
                            && !matches!(bytes[end], b' ' | b'\t' | b'\r' | b'>' | b'/')
                        {
                            end += 1;
                        }
                        let kind = if special {
                            TokenKind::Keyword
                        } else {
                            TokenKind::Tag
                        };
                        push(out, i, name_start, TokenKind::Tag);
                        push(out, name_start, end, kind);
                        i = end;
                        *state = XML_TAG;
                    } else if rest.starts_with('&')
                        && let Some(semi) = rest.find(';')
                        && semi <= 10
                        && !rest[1..semi].contains(char::is_whitespace)
                    {
                        push(out, i, i + semi + 1, TokenKind::Variable);
                        i += semi + 1;
                    } else {
                        i += char_len(line, i);
                    }
                }
            }
        }
    }
}
