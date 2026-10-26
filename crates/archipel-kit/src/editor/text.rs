//! What the code views know about a text, computed once and cached across frames: line
//! boundaries, highlighter checkpoints, wrap rows, fold regions and find results.

use super::highlight::{Highlighter, Span};
use egui::{Context, Id};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// Width of a tab stop, in columns.
pub(super) const TAB_WIDTH: usize = 4;
/// Lines longer than this have their spans cached instead of recomputed every frame.
const LONG_LINE: usize = 4096;
/// A highlighter state is remembered for every this many lines.
const CHECKPOINT: usize = 256;
/// Find stops collecting here; the count reads "100000+".
pub(super) const MAX_MATCHES: usize = 100_000;

pub(super) struct LineInfo {
    /// Byte offset of the first char.
    pub start: usize,
    /// Byte offset past the last char (before the line break).
    pub end: usize,
    /// Display columns: chars, with tabs advancing to the next tab stop.
    pub cols: usize,
    /// ASCII without tabs: bytes, chars and columns coincide.
    pub simple: bool,
}

#[derive(Default)]
struct Shared {
    checkpoints: Vec<u32>,
    long: HashMap<usize, (Arc<Vec<Span>>, u32)>,
    wrap: Option<(usize, Arc<Vec<usize>>)>,
    folds: Option<Arc<Vec<(usize, usize)>>>,
    found: Option<(String, Arc<Vec<Range<usize>>>)>,
}

/// The cached knowledge about one text and highlighter.
pub(super) struct Cache {
    pub lines: Vec<LineInfo>,
    pub max_cols: usize,
    key: (u64, &'static str),
    shared: Mutex<Shared>,
}

/// A cheap identity of `text`: length, address, the caller's revision and a sample of the
/// content. Hashing all of a 10 MB text every frame would cost more than drawing it.
fn fingerprint(text: &str, revision: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let bytes = text.as_bytes();
    (bytes.len(), bytes.as_ptr() as usize, revision).hash(&mut hasher);
    let edge = bytes.len().min(256);
    bytes[..edge].hash(&mut hasher);
    bytes[bytes.len() - edge..].hash(&mut hasher);
    let stride = (bytes.len() / 512).max(1);
    for chunk in bytes.chunks(8).step_by(stride.div_ceil(8)) {
        chunk.hash(&mut hasher);
    }
    hasher.finish()
}

impl Cache {
    /// The cache of `text` in `ctx` under `id`, rebuilt when the text or highlighter
    /// changed.
    pub fn get(
        ctx: &Context,
        id: Id,
        text: &str,
        highlighter: &dyn Highlighter,
        revision: u64,
    ) -> Arc<Cache> {
        let key = (fingerprint(text, revision), highlighter.name());
        let slot = id.with("archipel-code-cache");
        if let Some(cache) = ctx.data(|d| d.get_temp::<Arc<Cache>>(slot))
            && cache.key == key
        {
            return cache;
        }
        let cache = Arc::new(Cache::build(text, key));
        ctx.data_mut(|d| d.insert_temp(slot, cache.clone()));
        cache
    }

    fn build(text: &str, key: (u64, &'static str)) -> Cache {
        let mut lines = Vec::new();
        let mut max_cols = 0;
        let mut start = 0;
        for segment in text.split('\n') {
            let end = start + segment.len();
            let line = segment.strip_suffix('\r').unwrap_or(segment);
            let simple = line.is_ascii() && !line.as_bytes().contains(&b'\t');
            let cols = if simple {
                line.len()
            } else {
                display_cols(line)
            };
            max_cols = max_cols.max(cols);
            lines.push(LineInfo {
                start,
                end: start + line.len(),
                cols,
                simple,
            });
            start = end + 1;
        }
        // A final line break doesn't start another line.
        if lines.len() > 1 && text.ends_with('\n') {
            lines.pop();
        }
        Cache {
            lines,
            max_cols,
            key,
            shared: Mutex::new(Shared {
                checkpoints: vec![0],
                ..Shared::default()
            }),
        }
    }

    fn shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn line<'a>(&self, text: &'a str, line: usize) -> &'a str {
        let info = &self.lines[line];
        &text[info.start..info.end]
    }

    /// The line containing byte offset `byte`.
    pub fn line_of_byte(&self, byte: usize) -> usize {
        self.lines
            .partition_point(|l| l.start <= byte)
            .saturating_sub(1)
    }

    /// Highlighter state at the start of `line`.
    pub fn state_at(&self, text: &str, highlighter: &dyn Highlighter, line: usize) -> u32 {
        if highlighter.stateless() {
            return 0;
        }
        let mut shared = self.shared();
        let mut scratch = Vec::new();
        while shared.checkpoints.len() <= line / CHECKPOINT {
            let first = (shared.checkpoints.len() - 1) * CHECKPOINT;
            let mut state = shared.checkpoints[shared.checkpoints.len() - 1];
            for l in first..first + CHECKPOINT {
                scratch.clear();
                self.tokenize(&mut shared, text, highlighter, l, &mut state, &mut scratch);
            }
            shared.checkpoints.push(state);
        }
        let first = line / CHECKPOINT * CHECKPOINT;
        let mut state = shared.checkpoints[line / CHECKPOINT];
        for l in first..line {
            scratch.clear();
            self.tokenize(&mut shared, text, highlighter, l, &mut state, &mut scratch);
        }
        state
    }

    /// The spans of `line`, and `state` advanced past it. Short lines are tokenized into
    /// `out`; the spans of long lines are cached and returned instead (`out` stays empty).
    pub fn spans(
        &self,
        text: &str,
        highlighter: &dyn Highlighter,
        line: usize,
        state: &mut u32,
        out: &mut Vec<Span>,
    ) -> Option<Arc<Vec<Span>>> {
        let mut shared = self.shared();
        self.tokenize(&mut shared, text, highlighter, line, state, out)
    }

    fn tokenize(
        &self,
        shared: &mut Shared,
        text: &str,
        highlighter: &dyn Highlighter,
        line: usize,
        state: &mut u32,
        out: &mut Vec<Span>,
    ) -> Option<Arc<Vec<Span>>> {
        let info = self.lines.get(line)?;
        let content = &text[info.start..info.end];
        if content.len() <= LONG_LINE {
            highlighter.highlight_line(content, state, out);
            return None;
        }
        if let Some((spans, end_state)) = shared.long.get(&line) {
            *state = *end_state;
            return Some(spans.clone());
        }
        let mut spans = Vec::new();
        highlighter.highlight_line(content, state, &mut spans);
        let spans = Arc::new(spans);
        if shared.long.len() >= 8 {
            shared.long.clear();
        }
        shared.long.insert(line, (spans.clone(), *state));
        Some(spans)
    }

    /// First row of every line when lines wrap at `cols` columns, plus the total at the end.
    pub fn wrap_prefix(&self, cols: usize) -> Arc<Vec<usize>> {
        let mut shared = self.shared();
        if let Some((c, prefix)) = &shared.wrap
            && *c == cols
        {
            return prefix.clone();
        }
        let mut prefix = Vec::with_capacity(self.lines.len() + 1);
        let mut rows = 0;
        for line in &self.lines {
            prefix.push(rows);
            rows += line.cols.div_ceil(cols.max(1)).max(1);
        }
        prefix.push(rows);
        let prefix = Arc::new(prefix);
        shared.wrap = Some((cols, prefix.clone()));
        prefix
    }

    /// Foldable regions `(first line, last line)`: bracket pairs spanning several lines,
    /// sorted by first line, one per first line (the longest).
    pub fn folds(&self, text: &str) -> Arc<Vec<(usize, usize)>> {
        let mut shared = self.shared();
        if let Some(folds) = &shared.folds {
            return folds.clone();
        }
        let bytes = text.as_bytes();
        let mut stack = Vec::new();
        let mut regions: Vec<(usize, usize)> = Vec::new();
        for (index, info) in self.lines.iter().enumerate() {
            let (mut in_string, mut escaped) = (false, false);
            for &b in &bytes[info.start..info.end] {
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
                        b'{' | b'[' => stack.push(index),
                        b'}' | b']' => {
                            if let Some(open) = stack.pop()
                                && open < index
                            {
                                regions.push((open, index));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        regions.sort_unstable_by_key(|&(start, end)| (start, std::cmp::Reverse(end)));
        regions.dedup_by_key(|r| r.0);
        let folds = Arc::new(regions);
        shared.folds = Some(folds.clone());
        folds
    }

    /// Byte ranges of `query` in `text`, ASCII case-insensitive (other characters match
    /// exactly), cached for the last query.
    pub fn find(&self, text: &str, query: &str) -> Arc<Vec<Range<usize>>> {
        let mut shared = self.shared();
        if let Some((q, found)) = &shared.found
            && q == query
        {
            return found.clone();
        }
        let found = Arc::new(search(text, query));
        shared.found = Some((query.to_owned(), found.clone()));
        found
    }
}

/// Byte ranges of `query` in `text`: ASCII case-insensitive for ASCII queries, exact
/// otherwise. At most [`MAX_MATCHES`].
pub(super) fn search(text: &str, query: &str) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    if query.is_empty() {
        return found;
    }
    if !query.is_ascii() {
        found.extend(
            text.match_indices(query)
                .take(MAX_MATCHES)
                .map(|(at, m)| at..at + m.len()),
        );
        return found;
    }
    let needle = query.to_ascii_lowercase().into_bytes();
    let hay = text.as_bytes();
    let (lower, upper) = (needle[0], needle[0].to_ascii_uppercase());
    let mut at = 0;
    while at + needle.len() <= hay.len() && found.len() < MAX_MATCHES {
        match hay[at..hay.len() + 1 - needle.len()]
            .iter()
            .position(|&b| b == lower || b == upper)
        {
            None => break,
            Some(offset) => {
                let start = at + offset;
                if hay[start..start + needle.len()].eq_ignore_ascii_case(&needle) {
                    found.push(start..start + needle.len());
                    at = start + needle.len();
                } else {
                    at = start + 1;
                }
            }
        }
    }
    found
}

fn char_width(col: usize, c: char) -> usize {
    if c == '\t' {
        TAB_WIDTH - col % TAB_WIDTH
    } else {
        1
    }
}

/// Display columns of `line`.
pub(super) fn display_cols(line: &str) -> usize {
    line.chars().fold(0, |col, c| col + char_width(col, c))
}

/// Byte offset of display column `col` in `line` (a column inside a tab rounds up).
pub(super) fn byte_of_col(line: &str, simple: bool, col: usize) -> usize {
    if simple {
        return col.min(line.len());
    }
    let mut at = 0;
    for (byte, c) in line.char_indices() {
        if at >= col {
            return byte;
        }
        at += char_width(at, c);
    }
    line.len()
}

/// Display column of byte offset `byte` in `line`.
pub(super) fn col_of_byte(line: &str, simple: bool, byte: usize) -> usize {
    if simple {
        return byte.min(line.len());
    }
    let mut col = 0;
    for (at, c) in line.char_indices() {
        if at >= byte {
            break;
        }
        col += char_width(col, c);
    }
    col
}

/// `line` with tabs replaced by spaces, and `spans` moved to match.
pub(super) fn expand(line: &str, spans: &[Span]) -> (String, Vec<Span>) {
    let mut out = String::with_capacity(line.len());
    let mut map = vec![0; line.len() + 1];
    let mut col = 0;
    for (byte, c) in line.char_indices() {
        map[byte] = out.len();
        let width = char_width(col, c);
        if c == '\t' {
            out.extend(std::iter::repeat_n(' ', width));
        } else {
            out.push(c);
        }
        col += width;
    }
    map[line.len()] = out.len();
    // Fill the continuation bytes of multi-byte chars with their char's end.
    for byte in (0..line.len()).rev() {
        if !line.is_char_boundary(byte) {
            map[byte] = map[byte + 1];
        }
    }
    let spans = spans
        .iter()
        .map(|s| Span {
            start: map[s.start.min(line.len())],
            end: map[s.end.min(line.len())],
            kind: s.kind,
        })
        .collect();
    (out, spans)
}
