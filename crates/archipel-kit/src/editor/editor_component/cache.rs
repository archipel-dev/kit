//! What the component knows about a document and highlighter, computed once and cached across
//! frames: the longest line, highlighter checkpoints, bracket fold regions, find results and
//! the occurrences of the selected text. It lives in the egui context under the component's id
//! and is rebuilt when the document changes.

use crate::editor::highlight::{Span, SyntaxHighlighter};
use crate::editor::logic::{BracketRegion, bracket_regions, occurrences_of};
use crate::editor::model::{Document, MAX_SEARCH_RESULTS, SearchOptions, SearchResults, find_all};
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

    /// Bracket pairs that span lines (see [`bracket_regions`]), cached with the document.
    pub fn brackets(&self, doc: &Document) -> Arc<Vec<BracketRegion>> {
        let mut inner = self.inner();
        if let Some(brackets) = &inner.brackets {
            return brackets.clone();
        }
        let out = Arc::new(bracket_regions(doc));
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
        let found = occurrences_of(doc, selection.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::PlainText;

    fn cache_of(doc: &Document) -> Arc<DocCache> {
        DocCache::get(&Context::default(), Id::new("t"), doc, &PlainText, 4)
    }

    #[test]
    fn the_longest_line_is_measured_in_columns() {
        let doc = Document::new("ab\n\tx\nabcd");
        assert_eq!(cache_of(&doc).max_cols, 5);
    }
}
