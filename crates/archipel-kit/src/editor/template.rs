//! `{{name}}` template variables: highlighting and caret detection for single-line inputs.

use super::highlight::{Highlighter, PlainText, Span, TokenKind};
use std::ops::Range;

/// What a `{{name}}` resolves to, decided by the caller's resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateToken {
    /// A defined variable.
    Resolved,
    /// Not defined anywhere.
    Unresolved,
    /// Defined, and its value is a secret.
    Secret,
}

impl TemplateToken {
    fn kind(self) -> TokenKind {
        match self {
            TemplateToken::Resolved => TokenKind::Resolved,
            TemplateToken::Unresolved => TokenKind::Unresolved,
            TemplateToken::Secret => TokenKind::Secret,
        }
    }
}

/// Highlights `{{name}}` variables, and delegates the text around them to an inner
/// highlighter.
///
/// The resolver tells each name apart: [`Resolved`](TemplateToken::Resolved) variables use
/// the variable color, [`Unresolved`](TemplateToken::Unresolved) ones the invalid color (with
/// a dotted underline in a [`JBTextField`](crate::ui::JBTextField)),
/// [`Secret`](TemplateToken::Secret) ones the string color on a tint of it. These come from
/// [`TokenKind::color`](crate::editor::TokenKind::color), so a variable looks the same in a field, a
/// view and an editor. A variable still being typed (`{{ba`) is drawn as a plain variable. Names are
/// trimmed: `{{ name }}` resolves `"name"`.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::JBTextField;
/// use archipel_kit::editor::{TemplateHighlighter, TemplateToken};
///
/// let highlighter = TemplateHighlighter::new(|name: &str| match name {
///     "token" => TemplateToken::Secret,
///     "base_url" => TemplateToken::Resolved,
///     _ => TemplateToken::Unresolved,
/// });
/// let mut url = String::from("{{base_url}}/users?key={{token}}&q={{missing}}");
/// ui.add(JBTextField::new(&mut url).highlighter(&highlighter).monospace(true));
/// # });
/// ```
pub struct TemplateHighlighter<F, H = PlainText> {
    resolver: F,
    inner: H,
}

impl<F: Fn(&str) -> TemplateToken> TemplateHighlighter<F> {
    /// Highlights variables through `resolver`; the rest of the text stays plain.
    pub fn new(resolver: F) -> Self {
        Self {
            resolver,
            inner: PlainText,
        }
    }
}

impl<F: Fn(&str) -> TemplateToken, H: Highlighter> TemplateHighlighter<F, H> {
    /// Highlights the text between variables with `inner` (a URL, JSON …).
    pub fn inner<I: Highlighter>(self, inner: I) -> TemplateHighlighter<F, I> {
        TemplateHighlighter {
            resolver: self.resolver,
            inner,
        }
    }
}

impl<F: Fn(&str) -> TemplateToken, H: Highlighter> Highlighter for TemplateHighlighter<F, H> {
    fn highlight_line(&self, line: &str, state: &mut u32, out: &mut Vec<Span>) {
        let mut inner = Vec::new();
        self.inner.highlight_line(line, state, &mut inner);
        let found = scan(line);
        if found.is_empty() {
            out.extend(inner);
            return;
        }
        let mut spans: Vec<Span> = Vec::new();
        for span in inner {
            let mut start = span.start;
            for f in &found {
                if f.range.end <= start || f.range.start >= span.end {
                    continue;
                }
                if f.range.start > start {
                    spans.push(Span {
                        end: f.range.start,
                        start,
                        kind: span.kind,
                    });
                }
                start = start.max(f.range.end);
            }
            if start < span.end {
                spans.push(Span {
                    start,
                    end: span.end,
                    kind: span.kind,
                });
            }
        }
        for f in &found {
            let kind = if f.closed {
                (self.resolver)(&f.name).kind()
            } else {
                TokenKind::Variable
            };
            spans.push(Span {
                start: f.range.start,
                end: f.range.end,
                kind,
            });
        }
        spans.sort_by_key(|s| s.start);
        out.extend(spans);
    }

    fn stateless(&self) -> bool {
        self.inner.stateless()
    }
}

/// The template variable at the caret: where it is, so the caller can replace it, and what
/// has been typed of its name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TemplateAt {
    /// Byte range of the whole token in the text: `{{`, the name and the closing `}}` when
    /// present. [`insert_completion`](Self::insert_completion) replaces exactly this.
    pub range: Range<usize>,
    /// The name typed before the caret, without the braces and leading spaces: `ba` for
    /// `{{ba|se}}`. Filter the suggestions with it.
    pub partial: String,
    /// The whole name, trimmed.
    pub name: String,
    /// `true` when the token already has its closing `}}`.
    pub closed: bool,
}

impl TemplateAt {
    /// Replaces the token in `text` with `{{name}}`; returns the byte offset after it, where
    /// the caret belongs.
    pub fn insert_completion(&self, text: &mut String, name: &str) -> usize {
        let range = self.range.start.min(text.len())..self.range.end.min(text.len());
        let replacement = format!("{{{{{name}}}}}");
        let end = range.start + replacement.len();
        text.replace_range(range, &replacement);
        end
    }
}

/// The template variable the caret (a byte offset in `text`) is in or right after: the
/// caret must be after the opening `{{` and at or before the closing `}}`. `None` anywhere
/// else, including right before `{{`.
///
/// ```
/// use archipel_kit::editor::template_at;
///
/// let at = template_at("{{host}}/api", 4).expect("a value");
/// assert_eq!((at.partial.as_str(), at.name.as_str()), ("ho", "host"));
/// assert!(template_at("{{host}}/api", 9).is_none());
/// ```
pub fn template_at(text: &str, caret: usize) -> Option<TemplateAt> {
    let caret = caret.min(text.len());
    if !text.is_char_boundary(caret) {
        return None;
    }
    // Variables never span lines.
    let line_start = text[..caret].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
    scan(&text[line_start..line_end])
        .into_iter()
        .find(|f| {
            let (start, end) = (line_start + f.range.start, line_start + f.range.end);
            caret > start && caret <= end
        })
        .map(|f| {
            let name_start = line_start + f.name_start;
            let name_end = name_start + f.name.len();
            let typed = &text[name_start..caret.clamp(name_start, name_end)];
            TemplateAt {
                range: line_start + f.range.start..line_start + f.range.end,
                partial: typed.to_owned(),
                name: f.name,
                closed: f.closed,
            }
        })
}

struct Found {
    range: Range<usize>,
    name: String,
    /// Byte offset where the name begins (after the braces and spaces).
    name_start: usize,
    closed: bool,
}

/// Finds the `{{…}}` tokens of one line. A closed token holds anything but braces; one
/// without `}}` is the name characters typed so far.
fn scan(line: &str) -> Vec<Found> {
    let mut found = Vec::new();
    let mut i = 0;
    while let Some(at) = line[i..].find("{{") {
        let start = i + at;
        let body = start + 2;
        let rest = &line[body..];
        let stop = rest.find(['{', '}']).unwrap_or(rest.len());
        if rest[stop..].starts_with("}}") {
            let content = &rest[..stop];
            let lead = content.len() - content.trim_start().len();
            found.push(Found {
                range: start..body + stop + 2,
                name: content.trim().to_owned(),
                name_start: body + lead,
                closed: true,
            });
            i = body + stop + 2;
        } else {
            let content = &rest[..stop];
            let lead = content.len() - content.trim_start().len();
            let len = content[lead..]
                .find(char::is_whitespace)
                .unwrap_or(content.len() - lead);
            let end = body + lead + len;
            found.push(Found {
                range: start..end,
                name: line[body + lead..end].to_owned(),
                name_start: body + lead,
                closed: false,
            });
            i = end.max(body);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, caret: usize) -> Option<(String, String, Range<usize>, bool)> {
        template_at(text, caret).map(|t| (t.partial, t.name, t.range, t.closed))
    }

    #[test]
    fn caret_positions_in_a_closed_token() {
        let text = "a/{{host}}/b";
        assert_eq!(at(text, 2), None, "right before the braces");
        assert_eq!(at(text, 3).expect("a value").0, "", "between the braces");
        assert_eq!(
            at(text, 4).expect("a value").0,
            "",
            "after the opening braces"
        );
        assert_eq!(at(text, 6).expect("a value").0, "ho");
        assert_eq!(
            at(text, 8).expect("a value").0,
            "host",
            "before the closing braces"
        );
        assert_eq!(
            at(text, 9).expect("a value").0,
            "host",
            "inside the closing braces"
        );
        assert_eq!(
            at(text, 10),
            Some(("host".into(), "host".into(), 2..10, true))
        );
        assert_eq!(at(text, 11), None, "after the token");
    }

    #[test]
    fn unclosed_token_while_typing() {
        assert_eq!(
            at("x{{ba", 5),
            Some(("ba".into(), "ba".into(), 1..5, false))
        );
        assert_eq!(
            at("x{{", 3),
            Some((String::new(), String::new(), 1..3, false))
        );
        // The rest of the name after the caret is part of the token.
        assert_eq!(
            at("{{base url", 3),
            Some(("b".into(), "base".into(), 0..6, false))
        );
        assert_eq!(at("{{base url", 8), None);
    }

    #[test]
    fn spaces_around_names_and_several_tokens() {
        let text = "{{ a }}{{b}}";
        assert_eq!(at(text, 5), Some(("a".into(), "a".into(), 0..7, true)));
        assert_eq!(at(text, 7).expect("a value").1, "a", "end of the first");
        assert_eq!(at(text, 10).expect("a value").1, "b");
        assert_eq!(
            at("{{ }}", 3),
            Some((String::new(), String::new(), 0..5, true))
        );
    }

    #[test]
    fn plain_text_and_edge_cases() {
        assert_eq!(at("", 0), None);
        assert_eq!(at("no braces", 4), None);
        assert_eq!(at("{single}", 3), None);
        assert_eq!(at("{{a}} and }} {{", 11), None);
        assert_eq!(at("é{{ü}}", 3).expect("a value").1, "ü", "multi-byte names");
        assert_eq!(at("é{{ü}}", 2), None, "inside a char");
        assert_eq!(
            at("{{a\n{{b}}", 3).expect("a value").1,
            "a",
            "tokens stop at line breaks"
        );
        assert_eq!(
            at("{{a}}", 99).expect("a value").1,
            "a",
            "caret past the end is clamped"
        );
    }

    #[test]
    fn completion_replaces_the_token() {
        let mut text = String::from("go {{ba}}/x");
        let token = template_at(&text, 7).expect("a value");
        let caret = token.insert_completion(&mut text, "base_url");
        assert_eq!(text, "go {{base_url}}/x");
        assert_eq!(caret, 15);
        let mut open = String::from("{{b");
        let token = template_at(&open, 3).expect("a value");
        assert_eq!(token.insert_completion(&mut open, "base"), 8);
        assert_eq!(open, "{{base}}");
    }

    #[test]
    fn highlights_around_an_inner_highlighter() {
        let h = TemplateHighlighter::new(|n: &str| match n {
            "a" => TemplateToken::Resolved,
            "s" => TemplateToken::Secret,
            _ => TemplateToken::Unresolved,
        })
        .inner(crate::editor::Json);
        let mut out = Vec::new();
        h.highlight_line("\"x{{a}}\" {{s}} {{z}} {{", &mut 0, &mut out);
        let kinds: Vec<_> = out.iter().map(|s| (s.start, s.end, s.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                (0, 2, TokenKind::String),
                (2, 7, TokenKind::Resolved),
                (7, 8, TokenKind::String),
                (9, 14, TokenKind::Secret),
                (15, 20, TokenKind::Unresolved),
                (21, 23, TokenKind::Variable),
            ]
        );
        assert!(out.windows(2).all(|w| w[0].end <= w[1].start));
    }
}
