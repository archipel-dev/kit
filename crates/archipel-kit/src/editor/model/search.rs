//! Find and replace without a screen: the matching rules of the editor's search bar (spec
//! section 7): options, results capped at 10 000, replacement templates, caret-relative
//! navigation, search history, and the multi-range edits find needs from the editor.

use super::document::Edit;
use super::editor::Editor;
use super::undo::UndoGroup;
use super::words::is_identifier_part;
use crate::host::PropertiesComponent;
use regex::{Captures, Regex, RegexBuilder};
use std::ops::Range;

/// Search results are collected up to this many (`SearchResults.cursorPositions` cap of the
/// IDE, spec 7 "Typing"); the status then reads `10000+`.
pub const MAX_SEARCH_RESULTS: usize = 10_000;

/// Recent queries and replacements kept by a [`SearchHistory`].
pub const MAX_SEARCH_HISTORY: usize = 20;

/// The toggles of the search bar (`FindModel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SearchOptions {
    /// Match Case (Alt+C). Off by default: matching folds case the Unicode way, so `é` finds
    /// `É`.
    pub match_case: bool,
    /// Words (Alt+W): a match must start and end at identifier boundaries.
    pub whole_words: bool,
    /// Regex (Alt+X): the query is a regular expression and the replacement may refer to
    /// groups with `$1` or `${name}`.
    pub regex: bool,
    /// Preserve Case: the replacement takes the case shape of the text it replaces.
    pub preserve_case: bool,
}

/// What is wrong with a query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternProblem {
    /// The regular expression does not compile: the status reads "Bad pattern".
    Bad,
    /// The regular expression matches the empty text (`|`, `a*`): "Vague pattern".
    Vague,
}

/// The matches of one query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchResults {
    /// Byte ranges, ascending, never empty ones, at most [`MAX_SEARCH_RESULTS`] of them.
    pub ranges: Vec<Range<usize>>,
    /// There were more matches than the cap.
    pub truncated: bool,
    /// Set when the pattern cannot be used; `ranges` is then empty.
    pub problem: Option<PatternProblem>,
}

impl SearchResults {
    /// The status text of the bar: `n/m` when `current` is one of the matches, `m results`,
    /// `1 result`, `0 results`, `Bad pattern`, `Vague pattern`; empty for an empty query.
    pub fn status(&self, query_is_empty: bool, current: Option<usize>) -> String {
        if let Some(problem) = self.problem {
            return match problem {
                PatternProblem::Bad => "Bad pattern",
                PatternProblem::Vague => "Vague pattern",
            }
            .to_owned();
        }
        if query_is_empty {
            return String::new();
        }
        let more = if self.truncated { "+" } else { "" };
        let total = self.ranges.len();
        match current {
            Some(index) => format!("{}/{total}{more}", index + 1),
            None if total == 1 && !self.truncated => "1 result".to_owned(),
            None => format!("{total}{more} results"),
        }
    }
}

/// A compiled query (IntelliJ `FindModel` + `FindManager.findString`).
#[derive(Clone, Debug)]
pub struct Searcher {
    regex: Regex,
    options: SearchOptions,
}

impl Searcher {
    /// Compiles `query`. An empty query has no matches and is not an error.
    pub fn new(query: &str, options: SearchOptions) -> Result<Self, PatternProblem> {
        let pattern = if options.regex {
            query.to_owned()
        } else {
            regex::escape(query)
        };
        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(!options.match_case)
            .multi_line(true)
            .build()
            .map_err(|_| PatternProblem::Bad)?;
        if options.regex && !query.is_empty() && regex.is_match("") {
            return Err(PatternProblem::Vague);
        }
        Ok(Self { regex, options })
    }

    /// The matches inside `scope` (the whole text when `None`), at most `cap`; the flag says
    /// whether there were more.
    pub fn find_all(
        &self,
        text: &str,
        scope: Option<Range<usize>>,
        cap: usize,
    ) -> (Vec<Range<usize>>, bool) {
        let scope = scope.unwrap_or(0..text.len());
        let end = floor_boundary(text, scope.end.min(text.len()));
        let mut pos = floor_boundary(text, scope.start.min(end));
        let hay = &text[..end];
        let mut found = Vec::new();
        while pos <= hay.len() {
            let Some(m) = self.regex.find_at(hay, pos) else {
                break;
            };
            let (start, stop) = (m.start(), m.end());
            if start == stop {
                match next_boundary(hay, stop) {
                    Some(next) => pos = next,
                    None => break,
                }
                continue;
            }
            if self.options.whole_words && !is_whole_word(text, start, stop) {
                match next_boundary(hay, start) {
                    Some(next) => pos = next,
                    None => break,
                }
                continue;
            }
            if found.len() >= cap {
                return (found, true);
            }
            found.push(start..stop);
            pos = stop;
        }
        (found, false)
    }

    /// The text that replaces the match `range` of `text` for the template `replacement`:
    /// literal, or with regex on `$1`, `${name}`, `\n`, `\t` and `\\` expanded; with
    /// Preserve Case the result takes the case shape of the match.
    pub fn replacement(&self, text: &str, range: Range<usize>, replacement: &str) -> String {
        let matched = &text[range.clone()];
        let mut out = if self.options.regex {
            match self.regex.captures_at(text, range.start) {
                Some(caps) if caps.get(0).is_some_and(|m| m.range() == range) => {
                    expand_template(replacement, &caps)
                }
                _ => replacement.to_owned(),
            }
        } else {
            replacement.to_owned()
        };
        if self.options.preserve_case {
            out = preserve_case(matched, &out);
        }
        out
    }
}

/// Finds `query` in `text` (compile and search in one go).
pub fn find_all(
    text: &str,
    query: &str,
    options: SearchOptions,
    scope: Option<Range<usize>>,
    cap: usize,
) -> SearchResults {
    if query.is_empty() {
        return SearchResults::default();
    }
    match Searcher::new(query, options) {
        Ok(searcher) => {
            let (ranges, truncated) = searcher.find_all(text, scope, cap);
            SearchResults {
                ranges,
                truncated,
                problem: None,
            }
        }
        Err(problem) => SearchResults {
            problem: Some(problem),
            ..SearchResults::default()
        },
    }
}

fn floor_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn next_boundary(text: &str, at: usize) -> Option<usize> {
    text[at..].chars().next().map(|c| at + c.len_utf8())
}

/// Whether `start..end` begins and ends at identifier boundaries.
fn is_whole_word(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(is_identifier_part) && !after.is_some_and(is_identifier_part)
}

/// Expands `$1`, `${name}`, `\n`, `\t`, `\\` and `\$` of a replacement template.
fn expand_template(template: &str, caps: &Captures<'_>) -> String {
    let mut out = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' if i + 1 < chars.len() => {
                out.push(match chars[i + 1] {
                    'n' => '\n',
                    't' => '\t',
                    other => other,
                });
                i += 2;
            }
            '$' if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() => {
                // As many digits as still name a group.
                let mut number = chars[i + 1].to_digit(10).unwrap_or(0) as usize;
                let mut used = 1;
                while let Some(d) = chars.get(i + 1 + used).and_then(|c| c.to_digit(10)) {
                    let next = number * 10 + d as usize;
                    if next >= caps.len() {
                        break;
                    }
                    number = next;
                    used += 1;
                }
                if let Some(group) = caps.get(number) {
                    out.push_str(group.as_str());
                }
                i += 1 + used;
            }
            '$' if chars.get(i + 1) == Some(&'{') => {
                if let Some(close) = chars[i + 2..].iter().position(|c| *c == '}') {
                    let name: String = chars[i + 2..i + 2 + close].iter().collect();
                    if let Some(group) = caps.name(&name) {
                        out.push_str(group.as_str());
                    }
                    i += close + 3;
                } else {
                    out.push('$');
                    i += 1;
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The case shape of `matched` applied to `replacement`: ALL CAPS, all lower, or Capitalized;
/// anything else leaves the replacement as written.
fn preserve_case(matched: &str, replacement: &str) -> String {
    let letters: Vec<char> = matched.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return replacement.to_owned();
    }
    if letters.iter().all(|c| c.is_lowercase()) {
        return replacement.to_lowercase();
    }
    if letters.len() > 1 && letters.iter().all(|c| c.is_uppercase()) {
        return replacement.to_uppercase();
    }
    let first_upper = letters[0].is_uppercase();
    if first_upper && letters[1..].iter().all(|c| c.is_lowercase()) {
        let mut chars = replacement.chars();
        return match chars.next() {
            Some(first) => first
                .to_uppercase()
                .chain(chars.flat_map(char::to_lowercase))
                .collect(),
            None => String::new(),
        };
    }
    replacement.to_owned()
}

/// The match to go to from `from`: the first one starting at or after it (`forward`), or the
/// last one ending at or before it. `None` when the search has to wrap.
pub fn next_match(ranges: &[Range<usize>], from: usize, forward: bool) -> Option<usize> {
    if forward {
        let i = ranges.partition_point(|r| r.start < from);
        (i < ranges.len()).then_some(i)
    } else {
        ranges.partition_point(|r| r.end <= from).checked_sub(1)
    }
}

/// The index of the match that is exactly `selection`, if any: the current match.
pub fn match_at(ranges: &[Range<usize>], selection: &Range<usize>) -> Option<usize> {
    let i = ranges.partition_point(|r| r.start < selection.start);
    (ranges.get(i) == Some(selection)).then_some(i)
}

/// Recent queries and replacements, newest first. The caller keeps one per kind of search
/// (every editor of a tool can share it) and may persist it with a
/// [`PropertiesComponent`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchHistory {
    queries: Vec<String>,
    replacements: Vec<String>,
}

/// Separates entries in the persisted form (queries may hold line breaks).
const HISTORY_SEPARATOR: char = '\u{1f}';

impl SearchHistory {
    /// An empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Recent queries, newest first.
    pub fn queries(&self) -> &[String] {
        &self.queries
    }

    /// Recent replacements, newest first.
    pub fn replacements(&self) -> &[String] {
        &self.replacements
    }

    /// Remembers a query (empty ones are not; an equal entry moves to the front).
    pub fn add_query(&mut self, query: &str) {
        push_recent(&mut self.queries, query);
    }

    /// Remembers a replacement.
    pub fn add_replacement(&mut self, replacement: &str) {
        push_recent(&mut self.replacements, replacement);
    }

    /// Forgets everything.
    pub fn clear(&mut self) {
        self.queries.clear();
        self.replacements.clear();
    }

    /// Reads the history stored under `key` (`<key>.find`, `<key>.replace`).
    pub fn load(props: &dyn PropertiesComponent, key: &str) -> Self {
        let read = |suffix: &str| {
            props
                .get(&format!("{key}.{suffix}"))
                .map(|text| {
                    text.split(HISTORY_SEPARATOR)
                        .filter(|entry| !entry.is_empty())
                        .take(MAX_SEARCH_HISTORY)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default()
        };
        Self {
            queries: read("find"),
            replacements: read("replace"),
        }
    }

    /// Stores the history under `key`.
    pub fn save(&self, props: &mut dyn PropertiesComponent, key: &str) {
        let sep = HISTORY_SEPARATOR.to_string();
        props.set(&format!("{key}.find"), &self.queries.join(&sep));
        props.set(&format!("{key}.replace"), &self.replacements.join(&sep));
    }
}

fn push_recent(list: &mut Vec<String>, text: &str) {
    if text.is_empty() {
        return;
    }
    list.retain(|entry| entry != text);
    list.insert(0, text.to_owned());
    list.truncate(MAX_SEARCH_HISTORY);
}

impl Editor {
    /// Replaces several ranges of the text as **one** undo step (Replace All). The ranges
    /// must be ascending and apart; overlapping ones are skipped. Carets keep their text
    /// position.
    pub fn replace_ranges(&mut self, replacements: &[(Range<usize>, String)]) {
        let before = self.carets.snapshot();
        let mut edits: Vec<Edit> = Vec::new();
        let mut limit = usize::MAX;
        // From the end, so the offsets of the earlier ranges stay valid.
        for (range, text) in replacements.iter().rev() {
            if range.end > limit {
                continue;
            }
            limit = range.start;
            if let Some(edit) = self.document.replace_string(range.start, range.end, text) {
                self.carets.apply_edit(&edit);
                self.folding.apply_edit(&edit);
                edits.push(edit);
            }
        }
        if !edits.is_empty() {
            self.carets.normalize();
            self.undo
                .record(edits, before, self.carets.snapshot(), UndoGroup::None);
        }
    }

    /// Selects `ranges` with one caret each (Select All Occurrences); the last range holds
    /// the primary caret. Returns false when the caret limit cut the list short.
    pub fn select_ranges(&mut self, ranges: &[Range<usize>]) -> bool {
        let Some(first) = ranges.first() else {
            return true;
        };
        self.carets
            .set_selection(&self.document, first.start, first.end);
        for range in &ranges[1..] {
            if !self
                .carets
                .add_caret_with_selection(&self.document, range.start, range.end)
            {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTS: SearchOptions = SearchOptions {
        match_case: false,
        whole_words: false,
        regex: false,
        preserve_case: false,
    };

    fn all(text: &str, query: &str, options: SearchOptions) -> Vec<Range<usize>> {
        find_all(text, query, options, None, MAX_SEARCH_RESULTS).ranges
    }

    #[test]
    fn matching_folds_case_the_unicode_way() {
        assert_eq!(all("Foo fOO bar", "foo", OPTS), vec![0..3, 4..7]);
        assert_eq!(all("café CAFÉ", "café", OPTS), vec![0..5, 6..11]);
        assert_eq!(all("ΣΑΣ", "σασ", OPTS), vec![0..6]);
    }

    #[test]
    fn match_case_makes_it_exact() {
        let opts = SearchOptions {
            match_case: true,
            ..OPTS
        };
        assert_eq!(all("Foo foo", "foo", opts), vec![4..7]);
        assert_eq!(all("é É", "É", opts), vec![3..5]);
    }

    #[test]
    fn words_need_identifier_boundaries() {
        let opts = SearchOptions {
            whole_words: true,
            ..OPTS
        };
        assert_eq!(
            all("foo foobar _foo foo.x", "foo", opts),
            vec![0..3, 16..19]
        );
    }

    #[test]
    fn a_rejected_candidate_does_not_hide_a_later_one() {
        let opts = SearchOptions {
            whole_words: true,
            ..OPTS
        };
        assert_eq!(all("foofoo foo", "foo", opts), vec![7..10]);
    }

    #[test]
    fn regex_groups_and_bad_patterns() {
        let opts = SearchOptions {
            regex: true,
            ..OPTS
        };
        assert_eq!(all("a1 b22", r"[a-z]\d+", opts), vec![0..2, 3..6]);
        let bad = find_all("x", "(", opts, None, 10);
        assert_eq!(bad.problem, Some(PatternProblem::Bad));
        assert_eq!(bad.status(false, None), "Bad pattern");
        let vague = find_all("x", "|", opts, None, 10);
        assert_eq!(vague.status(false, None), "Vague pattern");
    }

    #[test]
    fn caret_and_dollar_match_at_line_boundaries() {
        let opts = SearchOptions {
            regex: true,
            ..OPTS
        };
        assert_eq!(all("ab\ncd\n", "^.", opts), vec![0..1, 3..4]);
    }

    #[test]
    fn results_stop_at_the_cap() {
        let text = "a".repeat(30);
        let capped = find_all(&text, "a", OPTS, None, 10);
        assert_eq!(capped.ranges.len(), 10);
        assert!(capped.truncated);
        assert_eq!(capped.status(false, None), "10+ results");
        assert_eq!(capped.status(false, Some(2)), "3/10+");
        let exact = find_all(&text, "a", OPTS, None, 30);
        assert!(!exact.truncated);
        assert_eq!(MAX_SEARCH_RESULTS, 10_000);
    }

    #[test]
    fn status_texts() {
        let none = find_all("abc", "x", OPTS, None, 10);
        assert_eq!(none.status(false, None), "0 results");
        assert_eq!(none.status(true, None), "");
        let one = find_all("abc", "b", OPTS, None, 10);
        assert_eq!(one.status(false, None), "1 result");
        assert_eq!(one.status(false, Some(0)), "1/1");
        let two = find_all("abcb", "b", OPTS, None, 10);
        assert_eq!(two.status(false, None), "2 results");
        assert_eq!(two.status(false, Some(1)), "2/2");
    }

    #[test]
    fn a_scope_limits_the_matches() {
        let found = find_all("aa aa aa", "aa", OPTS, Some(3..8), 10);
        assert_eq!(found.ranges, vec![3..5, 6..8]);
    }

    #[test]
    fn regex_replacements_use_groups() {
        let opts = SearchOptions {
            regex: true,
            ..OPTS
        };
        let s = Searcher::new(r"(\w+)@(\w+)", opts).expect("compiles");
        let text = "ann@home";
        assert_eq!(s.replacement(text, 0..8, "$2:$1"), "home:ann");
        assert_eq!(s.replacement(text, 0..8, r"$1\n\$"), "ann\n$");
        assert_eq!(s.replacement(text, 0..8, "$11"), "ann1");
        let named = Searcher::new(r"(?P<a>\w+)-", opts).expect("compiles");
        assert_eq!(named.replacement("ab-", 0..3, "${a}!"), "ab!");
    }

    #[test]
    fn literal_replacements_are_not_expanded() {
        let s = Searcher::new("a", OPTS).expect("compiles");
        assert_eq!(s.replacement("a", 0..1, "$1 \\n"), "$1 \\n");
    }

    #[test]
    fn preserve_case_takes_the_shape_of_the_match() {
        let opts = SearchOptions {
            preserve_case: true,
            ..OPTS
        };
        let s = Searcher::new("hello", opts).expect("compiles");
        assert_eq!(s.replacement("hello", 0..5, "world"), "world");
        assert_eq!(s.replacement("Hello", 0..5, "world"), "World");
        assert_eq!(s.replacement("HELLO", 0..5, "world"), "WORLD");
        assert_eq!(s.replacement("hELLo", 0..5, "world"), "world");
    }

    #[test]
    fn next_and_previous_matches_are_caret_relative() {
        let ranges = vec![2..4, 6..8, 10..12];
        assert_eq!(next_match(&ranges, 0, true), Some(0));
        assert_eq!(next_match(&ranges, 2, true), Some(0));
        assert_eq!(next_match(&ranges, 4, true), Some(1));
        assert_eq!(next_match(&ranges, 12, true), None);
        assert_eq!(next_match(&ranges, 12, false), Some(2));
        assert_eq!(next_match(&ranges, 10, false), Some(1));
        assert_eq!(next_match(&ranges, 2, false), None);
        assert_eq!(match_at(&ranges, &(6..8)), Some(1));
        assert_eq!(match_at(&ranges, &(6..7)), None);
    }

    #[test]
    fn history_is_newest_first_without_duplicates_and_capped() {
        let mut h = SearchHistory::new();
        h.add_query("a");
        h.add_query("b");
        h.add_query("a");
        h.add_query("");
        assert_eq!(h.queries(), ["a", "b"]);
        for i in 0..40 {
            h.add_replacement(&i.to_string());
        }
        assert_eq!(h.replacements().len(), MAX_SEARCH_HISTORY);
        assert_eq!(h.replacements()[0], "39");
    }

    #[test]
    fn history_round_trips_through_properties() {
        let mut h = SearchHistory::new();
        h.add_query("one\ntwo");
        h.add_query("x");
        h.add_replacement("y");
        let mut props = crate::host::InMemoryProperties::new();
        h.save(&mut props, "editor.search");
        assert_eq!(SearchHistory::load(&props, "editor.search"), h);
        assert!(SearchHistory::load(&props, "other").queries().is_empty());
    }

    #[test]
    fn replace_ranges_is_one_undo_step() {
        let mut editor = Editor::new("a b a b a");
        editor.replace_ranges(&[(0..1, "xx".into()), (4..5, "yy".into()), (8..9, "".into())]);
        assert_eq!(editor.document.text(), "xx b yy b ");
        assert_eq!(editor.undo.undo_depth(), 1);
    }

    #[test]
    fn select_ranges_makes_one_caret_per_range() {
        let mut editor = Editor::new("ab ab ab");
        assert!(editor.select_ranges(&[0..2, 3..5, 6..8]));
        assert_eq!(editor.carets.caret_count(), 3);
        assert_eq!(
            editor.carets.selected_texts(&editor.document),
            ["ab", "ab", "ab"]
        );
    }
}
