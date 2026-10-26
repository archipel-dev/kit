//! Keys: every key goes through the GUI's keymap ([`editor_action_for`]) to an
//! [`EditorAction`] and into [`Editor::execute`]; typed text becomes [`EditorAction::Type`].
//! A legacy terminal cannot send some chords, so there are fallbacks: the `Ctrl+K` chords of
//! [`chords`] (which [`terminal_keymap`](crate::input::terminal_keymap) adds to the keymap, so
//! the which-key list shows them) and a few single strokes.

use super::{EditorOutput, EditorState, Opts, run_action};
use crate::input::{self, KeyCode, KeyEvent};
use crate::runtime::Runtime;
use archipel_kit::editor::model::{
    ClipboardContent, Editor, EditorAction, KeyModifiers as ModelMods, Os, editor_action_for,
};
use archipel_kit::editor::{NavigationKind, NavigationRequest};
use archipel_kit::egui::Key;

/// A fallback chord: `Ctrl+K`, then `keys`, runs `action`.
#[derive(Clone, Copy)]
pub struct EditorChord {
    /// The second stroke, as [`KeyStroke::parse`](archipel_kit::action_system::KeyStroke::parse)
    /// reads it (`"shift Up"`).
    pub keys: &'static str,
    /// The action it runs.
    pub action: fn() -> EditorAction,
    /// What which-key shows.
    pub label: &'static str,
}

macro_rules! chord {
    ($keys:expr, $action:expr, $label:expr) => {
        EditorChord {
            keys: $keys,
            action: || $action,
            label: $label,
        }
    };
}

/// The `Ctrl+K` chords that stand in for the editor combos a legacy terminal cannot send
/// (Ctrl+Shift+letter looks like Ctrl+letter, Ctrl+[ is Escape, Alt+letter types a symbol on
/// macOS, Ctrl+Enter is a line feed). They also work in kitty-protocol terminals.
pub fn chords() -> Vec<EditorChord> {
    use EditorAction as A;
    vec![
        chord!("Up", A::MoveLineUp, "Move Line Up"),
        chord!("Down", A::MoveLineDown, "Move Line Down"),
        chord!("shift Up", A::MoveStatementUp, "Move Statement Up"),
        chord!("shift Down", A::MoveStatementDown, "Move Statement Down"),
        chord!("E", A::SelectWord, "Extend Selection"),
        chord!("shift E", A::UnSelectWord, "Shrink Selection"),
        chord!("S", A::SelectNextOccurrence, "Select Next Occurrence"),
        chord!(
            "shift S",
            A::UnselectPreviousOccurrence,
            "Unselect Occurrence"
        ),
        chord!("L", A::SelectAllOccurrences, "Select All Occurrences"),
        chord!("ctrl Up", A::CloneCaretAbove, "Clone Caret Above"),
        chord!("ctrl Down", A::CloneCaretBelow, "Clone Caret Below"),
        chord!("G", A::AddCaretPerSelectedLine, "Carets per Selected Line"),
        chord!("C", A::ToggleColumnMode, "Column Selection Mode"),
        chord!("O", A::StartNewLineBefore, "Start New Line Before"),
        chord!("J", A::JoinLines, "Join Lines"),
        chord!("U", A::ToggleCase, "Toggle Case"),
        chord!("M", A::MatchBrace, "Move to Matching Brace"),
        chord!("[", A::CodeBlockStart, "Code Block Start"),
        chord!("]", A::CodeBlockEnd, "Code Block End"),
        chord!(
            "shift [",
            A::CodeBlockStartWithSelection,
            "Select to Block Start"
        ),
        chord!(
            "shift ]",
            A::CodeBlockEndWithSelection,
            "Select to Block End"
        ),
        chord!("/", A::CommentByBlockComment, "Block Comment"),
        chord!("=", A::ExpandRegion, "Expand Region"),
        chord!("-", A::CollapseRegion, "Collapse Region"),
        chord!(
            "alt =",
            A::ExpandRegionRecursively,
            "Expand Region Recursively"
        ),
        chord!(
            "alt -",
            A::CollapseRegionRecursively,
            "Collapse Region Recursively"
        ),
        chord!("shift =", A::ExpandAllRegions, "Expand All Regions"),
        chord!("shift -", A::CollapseAllRegions, "Collapse All Regions"),
        chord!("V", A::PasteSimple, "Paste without Formatting"),
    ]
}

/// The name of the editor action with id `id` (a chord of [`chords`]), for which-key.
pub fn label_of(id: &str) -> Option<&'static str> {
    chords()
        .into_iter()
        .find(|c| (c.action)().id() == id)
        .map(|c| c.label)
}

fn chord_action(id: &str) -> Option<EditorAction> {
    // The general chords of the terminal keymap that are editor actions too (`Ctrl+K, Z` is
    // Redo: Ctrl+Shift+Z looks like Ctrl+Z in a legacy terminal).
    let general = [
        EditorAction::Redo,
        EditorAction::Undo,
        EditorAction::SelectAll,
    ];
    chords()
        .into_iter()
        .map(|c| (c.action)())
        .chain(general)
        .find(|a| a.id() == id)
}

/// Whether `action` changes the text (and so does nothing in a viewer).
pub(crate) fn is_editing(action: &EditorAction) -> bool {
    use EditorAction::*;
    matches!(
        action,
        Type(_)
            | Enter
            | StartNewLine
            | StartNewLineBefore
            | CompleteStatement
            | SplitLine
            | Tab
            | UnindentSelection
            | Backspace
            | Delete
            | DeleteToWordStart
            | DeleteToWordEnd
            | DeleteLine
            | CutLineEnd
            | Duplicate
            | JoinLines
            | ToggleCase
            | MoveLineUp
            | MoveLineDown
            | MoveStatementUp
            | MoveStatementDown
            | CommentByLineComment
            | CommentByBlockComment
            | Cut
            | Paste
            | PasteSimple
            | Undo
            | Redo
    )
}

/// The single strokes a legacy terminal sends for combos it cannot send as themselves, and
/// the ones the GUI keymap leaves free on Windows and Linux.
fn fallback(key: Key, mods: ModelMods) -> Option<EditorAction> {
    let alt = ModelMods::ALT;
    let ctrl = ModelMods::CTRL;
    match (key, mods) {
        // Extend and shrink selection without Ctrl+W (the terminal's window leader).
        (Key::ArrowUp, m) if m == alt => Some(EditorAction::SelectWord),
        (Key::ArrowDown, m) if m == alt => Some(EditorAction::UnSelectWord),
        // Ctrl+Enter arrives as Ctrl+J, Ctrl+/ as Ctrl+7, Ctrl+Backspace as Ctrl+H.
        (Key::J, m) if m == ctrl => Some(EditorAction::SplitLine),
        (Key::Num7, m) if m == ctrl => Some(EditorAction::CommentByLineComment),
        (Key::H, m) if m == ctrl => Some(EditorAction::DeleteToWordStart),
        _ => None,
    }
}

/// The editor action of a key event, by the GUI keymap and then the fallbacks.
pub(crate) fn action_for(event: &KeyEvent, os: Os) -> Option<EditorAction> {
    let stroke = input::key_stroke(event)?;
    let (mut key, mut m) = (stroke.key, stroke.modifiers);
    // A typed `+` is Shift+= on a US keyboard; the keymap binds Plus.
    if event.code == KeyCode::Char('+') {
        key = Key::Plus;
        m.shift = false;
    }
    let mods = ModelMods {
        ctrl: m.ctrl,
        alt: m.alt,
        shift: m.shift,
        command: os == Os::MacOs && m.mac_cmd,
    };
    editor_action_for(key, mods, os).or_else(|| fallback(key, mods))
}

/// A key event for the editor.
pub(crate) fn key(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    state: &mut EditorState,
    event: &KeyEvent,
    out: &mut EditorOutput,
) {
    if !rt.focused(opts.id) {
        return;
    }
    // The first stroke of a chord is the runtime's, not ours.
    if rt.awaiting_second_stroke() && !rt.chord_completed() {
        return;
    }
    if rt.chord_completed() {
        let ids: Vec<String> = rt.actions().to_vec();
        for id in ids {
            if let Some(action) = chord_action(&id)
                && !(opts.viewer && is_editing(&action))
            {
                state.close_menu();
                run_action(opts, editor, rt, state, &action, None, out);
                super::find::after_user_move(editor, state);
                return;
            }
        }
        return;
    }
    state.hint = None;
    if state.find.open && super::find::key(opts, editor, rt, state, event, out) {
        out.consumed = true;
        return;
    }
    if state.menu.is_some() && super::completion::key(opts, editor, rt, state, event, out) {
        out.consumed = true;
        return;
    }
    // Tab belongs to the application when the editor does not indent with it.
    if !opts.tab_indents
        && matches!(event.code, KeyCode::Tab | KeyCode::BackTab)
        && !event
            .modifiers
            .intersects(input::KeyModifiers::CONTROL | input::KeyModifiers::ALT)
    {
        return;
    }
    if super::find::editor_key(opts, editor, rt, state, event, out) {
        out.consumed = true;
        return;
    }
    // Ctrl+Space asks for completions.
    if event.code == KeyCode::Char(' ')
        && event.modifiers.contains(input::KeyModifiers::CONTROL)
        && !opts.viewer
    {
        super::completion::request(opts, editor, rt, state, out);
        out.consumed = true;
        return;
    }
    // Go to Declaration (Ctrl+B, Cmd+B) and Go to Implementation (Ctrl+Alt+B): the editor
    // only reports the request, as it does for a Ctrl+click.
    if let Some(kind) = navigation_key(event, opts.os) {
        out.navigation = Some(NavigationRequest {
            offset: editor.carets.primary().offset,
            kind,
        });
        out.consumed = true;
        return;
    }
    // Typed text.
    if let Some(c) = input::typed_char(event) {
        if opts.viewer {
            return;
        }
        let action = EditorAction::Type(c);
        if run_action(opts, editor, rt, state, &action, None, out) {
            super::find::after_user_move(editor, state);
        }
        return;
    }
    let Some(action) = action_for(event, opts.os) else {
        return;
    };
    if opts.viewer && is_editing(&action) {
        return;
    }
    let clipboard = matches!(action, EditorAction::Paste | EditorAction::PasteSimple)
        .then(|| clipboard_content(rt, None));
    if let Some(content) = &clipboard
        && content.text.is_empty()
    {
        return;
    }
    if run_action(opts, editor, rt, state, &action, clipboard.as_ref(), out) {
        super::find::after_user_move(editor, state);
    }
}

/// Ctrl+B (Cmd+B on macOS) asks for the declaration, with Alt for the implementation.
fn navigation_key(event: &KeyEvent, os: Os) -> Option<NavigationKind> {
    if !matches!(event.code, KeyCode::Char('b' | 'B')) {
        return None;
    }
    let mods = event.modifiers;
    let primary = match os {
        Os::MacOs => mods.contains(input::KeyModifiers::SUPER),
        Os::WindowsLinux => mods.contains(input::KeyModifiers::CONTROL),
    };
    if !primary || mods.contains(input::KeyModifiers::SHIFT) {
        return None;
    }
    Some(if mods.contains(input::KeyModifiers::ALT) {
        NavigationKind::Implementation
    } else {
        NavigationKind::Declaration
    })
}

/// The clipboard as paste content: whole lines when they were copied from an empty
/// selection and have not changed.
fn clipboard_content(rt: &Runtime, text: Option<&str>) -> ClipboardContent {
    let text = text.unwrap_or_else(|| rt.clipboard.get());
    if rt.clipboard.is_whole_line() && rt.clipboard.get() == text {
        ClipboardContent::lines(text)
    } else {
        ClipboardContent::text(text)
    }
}

/// A bracketed paste: into the find field that has the focus, else into the text.
pub(crate) fn paste(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    state: &mut EditorState,
    text: &str,
    out: &mut EditorOutput,
) {
    if !rt.focused(opts.id) {
        return;
    }
    if state.find.open && super::find::paste(opts, editor, state, rt.now, text, out) {
        out.consumed = true;
        return;
    }
    if opts.viewer {
        return;
    }
    let content = clipboard_content(rt, Some(text));
    state.close_menu();
    if run_action(
        opts,
        editor,
        rt,
        state,
        &EditorAction::Paste,
        Some(&content),
        out,
    ) {
        super::find::after_user_move(editor, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{KeyEvent, KeyModifiers};

    fn ev(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn keys_map_to_the_actions_of_the_gui_keymap() {
        let os = Os::WindowsLinux;
        assert_eq!(
            action_for(&ev(KeyCode::Right, KeyModifiers::CONTROL), os),
            Some(EditorAction::NextWord)
        );
        assert_eq!(
            action_for(
                &ev(KeyCode::Up, KeyModifiers::ALT | KeyModifiers::SHIFT),
                os
            ),
            Some(EditorAction::MoveLineUp)
        );
        assert_eq!(
            action_for(&ev(KeyCode::Char('j'), KeyModifiers::ALT), os),
            Some(EditorAction::SelectNextOccurrence)
        );
        assert_eq!(
            action_for(&ev(KeyCode::Char('z'), KeyModifiers::CONTROL), os),
            Some(EditorAction::Undo)
        );
        // Ctrl++ is expand, not expand all.
        assert_eq!(
            action_for(&ev(KeyCode::Char('+'), KeyModifiers::CONTROL), os),
            Some(EditorAction::ExpandRegion)
        );
    }

    #[test]
    fn the_fallbacks_fill_what_a_legacy_terminal_cannot_send() {
        let os = Os::WindowsLinux;
        assert_eq!(
            action_for(&ev(KeyCode::Up, KeyModifiers::ALT), os),
            Some(EditorAction::SelectWord)
        );
        assert_eq!(
            action_for(&ev(KeyCode::Char('7'), KeyModifiers::CONTROL), os),
            Some(EditorAction::CommentByLineComment)
        );
        // On macOS the keymap has Option+Up itself.
        assert_eq!(
            action_for(&ev(KeyCode::Up, KeyModifiers::ALT), Os::MacOs),
            Some(EditorAction::SelectWord)
        );
    }

    #[test]
    fn every_chord_parses_and_none_clashes_with_the_global_ones() {
        let taken = ["A", "F", "Z", "T", "W", "Q", "Enter"];
        let mut seen = Vec::new();
        for chord in chords() {
            assert!(
                archipel_kit::action_system::KeyStroke::parse(chord.keys).is_some(),
                "{}",
                chord.keys
            );
            assert!(!taken.contains(&chord.keys), "{}", chord.keys);
            assert!(!seen.contains(&chord.keys), "{}", chord.keys);
            seen.push(chord.keys);
        }
    }
}
