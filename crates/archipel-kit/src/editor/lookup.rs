//! Inserting an accepted lookup item into the editor model, at every caret as IntelliJ does.

use super::model::{ActionContext, Edit, Editor, EditorAction, Language, UndoGroup};
use crate::ui::completion::Chosen;
use crate::ui::is_identifier_char;

/// Inserts the chosen item as one undo step.
///
/// The primary caret decides what is replaced: the typed prefix before it (from the
/// completer's start) and, for Tab, the rest of the identifier after it. Every other caret
/// whose text before it ends in the same prefix gets the same replacement; carets elsewhere
/// are left alone. The carets end where the item's caret offset says, and a context character
/// is typed at all carets afterwards.
pub(crate) fn apply(editor: &mut Editor, language: &Language, chosen: &Chosen) {
    let before = editor.carets.snapshot();
    let text = editor.document.text().to_owned();
    let byte = |chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(b, _)| b)
    };
    let primary = editor.carets.primary().offset.min(text.len());
    let from = byte(chosen.start).min(primary);
    let end = byte(chosen.end.max(chosen.start));
    let left = primary - from;
    let prefix = &text[from..primary];
    let right_of = |offset: usize| {
        let mut right = if offset == primary {
            end.saturating_sub(primary)
        } else {
            0
        };
        if chosen.replace_tail {
            let tail: usize = text[offset..]
                .chars()
                .take_while(|c| is_identifier_char(*c))
                .map(char::len_utf8)
                .sum();
            right = right.max(tail);
        }
        right
    };

    let inserted = &chosen.item.insert;
    let chars = inserted.chars().count();
    let into: usize = inserted
        .chars()
        .take(chosen.item.caret.unwrap_or(chars).min(chars))
        .map(char::len_utf8)
        .sum();

    let mut targets: Vec<(usize, usize, usize)> = Vec::new();
    let mut last_end = 0;
    let offsets: Vec<(usize, usize, bool)> = editor
        .carets
        .carets()
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.offset.min(text.len()), c.has_selection()))
        .collect();
    for (index, offset, selected) in offsets {
        let is_primary = offset == primary;
        if (selected && !is_primary) || offset < left {
            continue;
        }
        let start = offset - left;
        if !is_primary && text.get(start..offset) != Some(prefix) {
            continue;
        }
        let stop = (offset + right_of(offset)).min(text.len());
        if text.get(offset..stop).is_none() || start < last_end {
            continue;
        }
        last_end = stop;
        targets.push((index, start, stop));
    }

    let mut edits: Vec<Edit> = Vec::new();
    let mut moved: Vec<(usize, isize)> = Vec::new();
    let mut delta: isize = 0;
    for (index, start, stop) in targets {
        let (s, e) = (
            (start as isize + delta) as usize,
            (stop as isize + delta) as usize,
        );
        if let Some(edit) = editor.document.replace_string(s, e, inserted) {
            editor.carets.apply_edit(&edit);
            editor.folding.apply_edit(&edit);
            edits.push(edit);
        }
        moved.push((index, s as isize + into as isize));
        delta += inserted.len() as isize - (stop - start) as isize;
    }
    for (index, offset) in moved {
        if let Some(caret) = editor.carets.carets_mut().get_mut(index) {
            caret.move_to(offset as usize);
        }
    }
    editor.carets.normalize();
    editor
        .undo
        .record(edits, before, editor.carets.snapshot(), UndoGroup::None);
    if let Some(c) = chosen.then {
        editor.execute(&EditorAction::Type(c), &ActionContext::new(language));
    }
}
