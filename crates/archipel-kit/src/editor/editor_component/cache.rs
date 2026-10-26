//! What the component knows about a document and highlighter, computed once and cached across
//! frames: the longest line, highlighter checkpoints, bracket fold regions, find results and
//! the occurrences of the selected text. It lives in the egui context under the component's id
//! and is rebuilt when the document changes.

use crate::editor::highlight::{Span, SyntaxHighlighter};
use crate::editor::model::{
    Document, FoldRegion, MAX_SEARCH_RESULTS, SearchOptions, SearchResults, find_all,
};
use crate::tokens::editor::MAX_SELECTION_OCCURRENCES;
use egui::{Context, Id};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// Lines longer than this have their spans cached instead of recomputed every frame.
const LONG_LINE: usize = 4096;
/// A highlighter state is remembered for every this many lines.
const CHECKPOINT: usize = 256;
type Ranges = Arc<Vec<Range<usize>>>;

/// What the cached find results were computed for.
type FindKey = (String, SearchOptions, Option<Range<usize>>);

#[derive(Default)]
struct Inner {
    checkpoints: Vec<u32>,
    long: HashMap<usize, (Arc<Vec<Span>>, u32)>,
    brackets: Option<Arc<Vec<BracketRegion>>>,
    widths: Option<Arc<Vec<u32>>>,
    found: Option<(FindKey, Arc<SearchResults>)>,
    occurrences: Option<(Range<usize>, Ranges)>,
}

/// A bracket pair spanning several lines: the range from the opener to just after the closer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BracketRegion {
    pub start: usize,
    pub end: usize,
    pub opener: char,
}

pub(crate) struct DocCache {
    key: u64,
    /// The longest line, in columns.
    pub max_cols: usize,
    inner: Mutex<Inner>,
}

/// A cheap identity of a document: its modification stamp, length, the tab size and a sample
/// of the content. Hashing all of a 10 MB text every frame would cost more than drawing it.
fn fingerprint(doc: &Document, tab_size: usize, highlighter: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let bytes = doc.text().as_bytes();
    (doc.modification_stamp(), bytes.len(), tab_size, highlighter).hash(&mut hasher);
    let edge = bytes.len().min(256);
    bytes[..edge].hash(&mut hasher);
    bytes[bytes.len() - edge..].hash(&mut hasher);
    let stride = (bytes.len() / 512).max(1);
    for chunk in bytes.chunks(8).step_by(stride.div_ceil(8)) {
        chunk.hash(&mut hasher);
    }
    hasher.finish()
}

impl DocCache {
    /// The cache of `doc` in `ctx` under `id`, rebuilt when the text or the highlighter
    /// changed.
    pub fn get(
        ctx: &Context,
        id: Id,
        doc: &Document,
        highlighter: &dyn SyntaxHighlighter,
        tab_size: usize,
    ) -> Arc<DocCache> {
        let key = fingerprint(doc, tab_size, highlighter.name());
        let slot = id.with("archipel-editor-cache");
        if let Some(cache) = ctx.data(|d| d.get_temp::<Arc<DocCache>>(slot))
            && cache.key == key
        {
            return cache;
        }
        let cache = Arc::new(DocCache::build(doc, key, tab_size));
        ctx.data_mut(|d| d.insert_temp(slot, cache.clone()));
        cache
    }

    fn build(doc: &Document, key: u64, tab_size: usize) -> DocCache {
        let tab = tab_size.max(1);
        let mut max_cols = 0;
        for line in doc.text().split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let cols = if line.is_ascii() && !line.as_bytes().contains(&b'\t') {
                line.len()
            } else {
                line.chars().fold(0, |col, c| {
                    if c == '\t' {
                        (col / tab + 1) * tab
                    } else {
                        col + 1
                    }
                })
            };
            max_cols = max_cols.max(cols);
        }
        DocCache {
            key,
            max_cols,
            inner: Mutex::new(Inner {
                checkpoints: vec![0],
                ..Inner::default()
            }),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// SyntaxHighlighter state at the start of `line`.
    pub fn state_at(
        &self,
        doc: &Document,
        highlighter: &dyn SyntaxHighlighter,
        line: usize,
    ) -> u32 {
        if highlighter.stateless() {
            return 0;
        }
        let mut inner = self.inner();
        let mut scratch = Vec::new();
        while inner.checkpoints.len() <= line / CHECKPOINT {
            let first = (inner.checkpoints.len() - 1) * CHECKPOINT;
            let mut state = inner.checkpoints[inner.checkpoints.len() - 1];
            for l in first..first + CHECKPOINT {
                scratch.clear();
                tokenize(&mut inner, doc, highlighter, l, &mut state, &mut scratch);
            }
            inner.checkpoints.push(state);
        }
        let first = line / CHECKPOINT * CHECKPOINT;
        let mut state = inner.checkpoints[line / CHECKPOINT];
        for l in first..line {
            scratch.clear();
            tokenize(&mut inner, doc, highlighter, l, &mut state, &mut scratch);
        }
        state
    }

    /// The spans of `line`, and `state` advanced past it. Short lines are tokenized into
    /// `out`; the spans of long lines are cached and returned instead (`out` stays empty).
    pub fn spans(
        &self,
        doc: &Document,
        highlighter: &dyn SyntaxHighlighter,
        line: usize,
        state: &mut u32,
        out: &mut Vec<Span>,
    ) -> Option<Arc<Vec<Span>>> {
        let mut inner = self.inner();
        tokenize(&mut inner, doc, highlighter, line, state, out)
    }

    /// The width in columns of every line (for soft wrap).
    pub fn widths(&self, doc: &Document, tab_size: usize) -> Arc<Vec<u32>> {
        let mut inner = self.inner();
        if let Some(widths) = &inner.widths {
            return widths.clone();
        }
        let tab = tab_size.max(1);
        let widths: Vec<u32> = doc
            .text()
            .split('\n')
            .map(|line| {
                let line = line.strip_suffix('\r').unwrap_or(line);
                if line.is_ascii() && !line.as_bytes().contains(&b'\t') {
                    line.len() as u32
                } else {
                    line.chars().fold(0u32, |col, c| {
                        if c == '\t' {
                            (col / tab as u32 + 1) * tab as u32
                        } else {
                            col + 1
                        }
                    })
                }
            })
            .collect();
        let widths = Arc::new(widths);
        inner.widths = Some(widths.clone());
        widths
    }

    /// Bracket pairs (`{}` and `[]`) whose closer is on a later line than the opener, sorted by
    /// start, one per opener line (the longest). Strings in double quotes are skipped.
    pub fn brackets(&self, doc: &Document) -> Arc<Vec<BracketRegion>> {
        let mut inner = self.inner();
        if let Some(brackets) = &inner.brackets {
            return brackets.clone();
        }
        let text = doc.text();
        let bytes = text.as_bytes();
        let mut stack: Vec<(usize, usize, u8)> = Vec::new();
        let mut regions: Vec<(usize, usize, usize, u8)> = Vec::new();
        let mut line_start = 0;
        for (line, piece) in text.split('\n').enumerate() {
            let (mut in_string, mut escaped) = (false, false);
            for (i, &b) in piece.as_bytes().iter().enumerate() {
                if in_string {
                    match b {
                        _ if escaped => escaped = false,
                        b'\\' => escaped = true,
                        b'"' => in_string = false,
                        _ => {}
                    }
                } else {
                    match b {
                        b'"' => in_string = true,
                        b'{' | b'[' => stack.push((line_start + i, line, b)),
                        b'}' | b']' => {
                            if let Some((open, open_line, opener)) = stack.pop()
                                && open_line < line
                            {
                                regions.push((open, line_start + i + 1, open_line, opener));
                            }
                        }
                        _ => {}
                    }
                }
            }
            line_start += piece.len() + 1;
        }
        let _ = bytes;
        regions.sort_unstable_by_key(|&(start, end, _, _)| (start, std::cmp::Reverse(end)));
        let mut out: Vec<BracketRegion> = Vec::new();
        let mut last_line = usize::MAX;
        for (start, end, open_line, opener) in regions {
            if open_line == last_line {
                continue;
            }
            last_line = open_line;
            out.push(BracketRegion {
                start,
                end,
                opener: opener as char,
            });
        }
        let out = Arc::new(out);
        inner.brackets = Some(out.clone());
        out
    }

    /// The matches of `query` in the document (at most `MAX_SEARCH_RESULTS`), cached for the
    /// last query, options and scope.
    pub fn find(
        &self,
        doc: &Document,
        query: &str,
        options: SearchOptions,
        scope: Option<Range<usize>>,
    ) -> Arc<SearchResults> {
        let mut inner = self.inner();
        let key = (query.to_owned(), options, scope);
        if let Some((k, found)) = &inner.found
            && *k == key
        {
            return found.clone();
        }
        let found = Arc::new(find_all(
            doc.text(),
            query,
            options,
            key.2.clone(),
            MAX_SEARCH_RESULTS,
        ));
        inner.found = Some((key, found.clone()));
        found
    }

    /// The other occurrences of the text at `selection` (exact match), or none when there are
    /// more than [`MAX_SELECTION_OCCURRENCES`]. The selection itself is not in the list.
    pub fn occurrences(&self, doc: &Document, selection: Range<usize>) -> Arc<Vec<Range<usize>>> {
        let mut inner = self.inner();
        if let Some((s, found)) = &inner.occurrences
            && *s == selection
        {
            return found.clone();
        }
        let needle = doc.slice(selection.start, selection.end);
        let mut found = Vec::new();
        if !needle.is_empty() {
            for (at, m) in doc.text().match_indices(needle) {
                if at != selection.start {
                    found.push(at..at + m.len());
                }
                if found.len() > MAX_SELECTION_OCCURRENCES {
                    found.clear();
                    break;
                }
            }
        }
        let found = Arc::new(found);
        inner.occurrences = Some((selection, found.clone()));
        found
    }
}

fn tokenize(
    inner: &mut Inner,
    doc: &Document,
    highlighter: &dyn SyntaxHighlighter,
    line: usize,
    state: &mut u32,
    out: &mut Vec<Span>,
) -> Option<Arc<Vec<Span>>> {
    if line >= doc.line_count() {
        return None;
    }
    let content = doc.line_text(line);
    if content.len() <= LONG_LINE {
        highlighter.highlight_line(content, state, out);
        return None;
    }
    if let Some((spans, end_state)) = inner.long.get(&line) {
        *state = *end_state;
        return Some(spans.clone());
    }
    let mut spans = Vec::new();
    highlighter.highlight_line(content, state, &mut spans);
    let spans = Arc::new(spans);
    if inner.long.len() >= 8 {
        inner.long.clear();
    }
    inner.long.insert(line, (spans.clone(), *state));
    Some(spans)
}

/// The fold regions for `brackets`, keeping the collapsed state of regions in `old` that
/// start at the same offset.
pub(crate) fn fold_regions(brackets: &[BracketRegion], old: &[FoldRegion]) -> Vec<FoldRegion> {
    brackets
        .iter()
        .map(|b| {
            let expanded = old
                .iter()
                .find(|r| r.start == b.start)
                .is_none_or(|r| r.expanded);
            FoldRegion {
                start: b.start,
                end: b.end,
                placeholder: if b.opener == '{' { "{...}" } else { "[...]" }.to_owned(),
                expanded,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::PlainText;

    fn cache_of(doc: &Document) -> Arc<DocCache> {
        DocCache::get(&Context::default(), Id::new("t"), doc, &PlainText, 4)
    }

    #[test]
    fn occurrences_skip_the_selection_itself() {
        let doc = Document::new("foo bar foo baz foo");
        let found = cache_of(&doc).occurrences(&doc, 0..3);
        assert_eq!(*found, vec![8..11, 16..19]);
    }

    #[test]
    fn more_than_fifty_occurrences_highlight_nothing() {
        let text = "ab ".repeat(60);
        let doc = Document::new(&text);
        assert!(cache_of(&doc).occurrences(&doc, 0..2).is_empty());
        let text = "ab ".repeat(51);
        let doc = Document::new(&text);
        assert_eq!(cache_of(&doc).occurrences(&doc, 0..2).len(), 50);
    }

    #[test]
    fn bracket_regions_span_lines_one_per_opener_line() {
        let doc = Document::new("{\n  \"a\": [1,\n  2],\n  \"b\": \"}\"\n}\n[ 1 ]");
        let brackets = cache_of(&doc).brackets(&doc);
        let spans: Vec<_> = brackets
            .iter()
            .map(|b| (b.start, b.end, b.opener))
            .collect();
        // The object spans lines 0..=4; the array lines 1..=2; `[ 1 ]` is on one line and the
        // `}` inside the string does not close anything.
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert_eq!(spans[0].2, '{');
        assert_eq!(doc.line_number(spans[0].1), 4);
        assert_eq!(spans[1].2, '[');
    }

    #[test]
    fn fold_regions_keep_what_was_collapsed() {
        let doc = Document::new("{\n  [\n  1\n  ]\n}");
        let brackets = cache_of(&doc).brackets(&doc);
        let first = fold_regions(&brackets, &[]);
        assert!(first.iter().all(|r| r.expanded));
        let mut old = first.clone();
        old[1].expanded = false;
        let again = fold_regions(&brackets, &old);
        assert_eq!(
            again.iter().map(|r| r.expanded).collect::<Vec<_>>(),
            [true, false]
        );
        assert_eq!(again[0].placeholder, "{...}");
        assert_eq!(again[1].placeholder, "[...]");
    }

    #[test]
    fn the_longest_line_is_measured_in_columns() {
        let doc = Document::new("ab\n\tx\nabcd");
        assert_eq!(cache_of(&doc).max_cols, 5);
    }
}
