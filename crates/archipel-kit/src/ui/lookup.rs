//! The model of a completion popup (IntelliJ `Lookup`): what a row shows
//! ([`LookupElementPresentation`]), how typed text matches an item ([`PrefixMatcher`], with
//! camel-hump matching) and what the caller remembers between lookups ([`LookupStats`]).
//!
//! Nothing here draws or reads input: [`Completer`](super::Completer) implementations may use
//! the matcher to rank their own items, and the list in `completion.rs` uses it to filter and
//! sort what a completer returns.

use crate::icons::Icon;
use std::collections::HashMap;
use std::ops::Range;

/// Whether `c` belongs to an identifier: the characters that open the popup when typed and
/// narrow it while it is open.
pub(crate) fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// How well the typed text matches an item. Best first: the derived order is the sort order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatchKind {
    /// The item starts with the typed text (ignoring case).
    Prefix,
    /// Camel humps from the item's first letter: `gS` matches `getString`.
    CamelHump,
    /// Humps (or words after `_`, `-`, `.`) starting anywhere in the item: `S` matches
    /// `getString` (IntelliJ "middle matching").
    Word,
    /// The typed text occurs inside a word of the item.
    Substring,
}

/// The result of [`PrefixMatcher::matches`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixMatch {
    /// How the text matched.
    pub kind: MatchKind,
    /// The matched fragments of the candidate, as char ranges (drawn in the match color).
    pub fragments: Vec<Range<usize>>,
    /// Whether every typed char equals the matched char exactly, case included.
    pub exact_case: bool,
}

/// Matches the text typed so far against item texts (IntelliJ `PrefixMatcher`), with
/// camel-hump and middle matching. Matching ignores case; [`PrefixMatch::exact_case`] tells
/// the sort whether the case agreed too.
///
/// ```
/// use archipel_kit::ui::{MatchKind, PrefixMatcher};
///
/// let matcher = PrefixMatcher::new("gS");
/// let found = matcher.matches("getString").expect("matches");
/// assert_eq!(found.kind, MatchKind::CamelHump);
/// assert_eq!(found.fragments, vec![0..1, 3..4]);
/// assert!(matcher.matches("size").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixMatcher {
    prefix: Vec<char>,
}

impl PrefixMatcher {
    /// A matcher for the text typed so far.
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.chars().collect(),
        }
    }

    /// The typed text.
    pub fn prefix(&self) -> String {
        self.prefix.iter().collect()
    }

    /// Whether nothing was typed (every item matches).
    pub fn is_empty(&self) -> bool {
        self.prefix.is_empty()
    }

    /// How `candidate` matches, or `None` when it does not.
    pub fn matches(&self, candidate: &str) -> Option<PrefixMatch> {
        let typed = &self.prefix;
        if typed.is_empty() {
            return Some(PrefixMatch {
                kind: MatchKind::Prefix,
                fragments: Vec::new(),
                exact_case: true,
            });
        }
        let cand: Vec<char> = candidate.chars().collect();
        if cand.len() < typed.len() {
            return None;
        }
        if typed.iter().zip(&cand).all(|(t, c)| same(*t, *c)) {
            let positions: Vec<usize> = (0..typed.len()).collect();
            return Some(build(MatchKind::Prefix, typed, &cand, &positions));
        }
        let humps = hump_starts(&cand);
        let mut positions = Vec::new();
        if search(typed, &cand, &humps, 0, None, true, &mut positions) {
            return Some(build(MatchKind::CamelHump, typed, &cand, &positions));
        }
        positions.clear();
        if search(typed, &cand, &humps, 0, None, false, &mut positions) {
            return Some(build(MatchKind::Word, typed, &cand, &positions));
        }
        (0..=cand.len() - typed.len())
            .find(|start| typed.iter().zip(&cand[*start..]).all(|(t, c)| same(*t, *c)))
            .map(|start| {
                let positions: Vec<usize> = (start..start + typed.len()).collect();
                build(MatchKind::Substring, typed, &cand, &positions)
            })
    }
}

/// Equal, ignoring case.
fn same(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// The char positions where a word or camel hump starts.
fn hump_starts(cand: &[char]) -> Vec<bool> {
    (0..cand.len())
        .map(|i| {
            let c = cand[i];
            if i == 0 {
                return true;
            }
            let prev = cand[i - 1];
            if !c.is_alphanumeric() {
                return false;
            }
            !prev.is_alphanumeric()
                || (c.is_uppercase() && prev.is_lowercase())
                || (c.is_uppercase()
                    && prev.is_uppercase()
                    && cand.get(i + 1).is_some_and(|n| n.is_lowercase()))
                || (c.is_ascii_digit() != prev.is_ascii_digit())
        })
        .collect()
}

/// Matches `typed[ti..]` after `prev`: each char continues the run or jumps to a hump.
fn search(
    typed: &[char],
    cand: &[char],
    humps: &[bool],
    ti: usize,
    prev: Option<usize>,
    from_start: bool,
    out: &mut Vec<usize>,
) -> bool {
    if ti == typed.len() {
        return true;
    }
    let t = typed[ti];
    // A typed capital must land on a capital hump start.
    let fits = |pos: usize| same(t, cand[pos]) && (!t.is_uppercase() || cand[pos].is_uppercase());
    let try_pos = |pos: usize, out: &mut Vec<usize>| {
        out.push(pos);
        if search(typed, cand, humps, ti + 1, Some(pos), from_start, out) {
            return true;
        }
        out.pop();
        false
    };
    match prev {
        None => {
            if from_start {
                return !cand.is_empty() && fits(0) && try_pos(0, out);
            }
            humps
                .iter()
                .enumerate()
                .any(|(pos, hump)| *hump && fits(pos) && try_pos(pos, out))
        }
        Some(p) => {
            if p + 1 < cand.len() && same(t, cand[p + 1]) && try_pos(p + 1, out) {
                return true;
            }
            humps
                .iter()
                .enumerate()
                .skip(p + 2)
                .any(|(pos, hump)| *hump && fits(pos) && try_pos(pos, out))
        }
    }
}

fn build(kind: MatchKind, typed: &[char], cand: &[char], positions: &[usize]) -> PrefixMatch {
    let mut fragments: Vec<Range<usize>> = Vec::new();
    for &p in positions {
        match fragments.last_mut() {
            Some(last) if last.end == p => last.end = p + 1,
            _ => fragments.push(p..p + 1),
        }
    }
    let exact_case = typed.iter().zip(positions).all(|(t, p)| *t == cand[*p]);
    PrefixMatch {
        kind,
        fragments,
        exact_case,
    }
}

/// What a row of the popup shows (IntelliJ `LookupElementPresentation`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LookupElementPresentation {
    /// The item text, drawn first.
    pub item_text: String,
    /// The gray text right after the item text (a signature's parameters).
    pub tail_text: String,
    /// The gray text at the row's right edge (a return type, a container).
    pub type_text: String,
    /// The icon before the item text.
    pub icon: Option<Icon>,
    /// The item text is drawn bold (a preferred item).
    pub bold: bool,
    /// The item text is struck out (a deprecated item).
    pub strikeout: bool,
    /// The parts of the item text that match what was typed, as char ranges: drawn in the
    /// match color (`CompletionPopup.matchForeground`).
    pub fragments: Vec<Range<usize>>,
}

/// What the caller remembers about earlier completions, to rank recently chosen items first
/// (the kit keeps no state: the caller owns this, records [`EditorComponentResponse::completed`]
/// and answers [`Completer::recency`](super::Completer::recency) from it).
///
/// [`EditorComponentResponse::completed`]: crate::editor::EditorComponentResponse::completed
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LookupStats {
    tick: u64,
    last: HashMap<String, u64>,
}

impl LookupStats {
    /// Remembers that the item `lookup_string` was chosen now.
    pub fn record(&mut self, lookup_string: &str) {
        self.tick += 1;
        self.last.insert(lookup_string.to_owned(), self.tick);
    }

    /// How recently the item was chosen: higher is more recent, 0 means never.
    pub fn recency(&self, lookup_string: &str) -> u64 {
        self.last.get(lookup_string).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(typed: &str, candidate: &str) -> Option<MatchKind> {
        PrefixMatcher::new(typed).matches(candidate).map(|m| m.kind)
    }

    #[test]
    fn prefix_beats_hump_beats_word_beats_substring() {
        assert_eq!(kind("get", "getString"), Some(MatchKind::Prefix));
        assert_eq!(kind("gS", "getString"), Some(MatchKind::CamelHump));
        assert_eq!(kind("Str", "getString"), Some(MatchKind::Word));
        assert_eq!(kind("tri", "getString"), Some(MatchKind::Substring));
        assert_eq!(kind("xyz", "getString"), None);
        assert!(MatchKind::Prefix < MatchKind::CamelHump);
        assert!(MatchKind::Word < MatchKind::Substring);
    }

    #[test]
    fn humps_follow_separators_and_capital_runs() {
        assert_eq!(kind("gu", "get_user"), Some(MatchKind::CamelHump));
        assert_eq!(kind("xp", "XMLParser"), Some(MatchKind::CamelHump));
        assert_eq!(kind("sb", "StringBuilder"), Some(MatchKind::CamelHump));
        // A typed capital must land on a capital.
        assert_eq!(kind("gS", "getstring"), None);
    }

    #[test]
    fn fragments_merge_runs() {
        let found = PrefixMatcher::new("stB")
            .matches("StringBuilder")
            .expect("matches");
        assert_eq!(found.fragments, vec![0..2, 6..7]);
        assert!(!found.exact_case);
        let found = PrefixMatcher::new("St").matches("String").expect("matches");
        assert!(found.exact_case);
    }
}
