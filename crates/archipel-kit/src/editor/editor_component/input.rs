//! Keyboard, text, IME and clipboard input: keys go through the model's keymap, typed text and
//! clipboard events become actions, and everything that was used is taken out of the frame's
//! input so nothing else reacts to it.

use super::state::ViewState;
use crate::editor::model::{
    ActionContext, ClipboardContent, Editor, EditorAction, KeyModifiers, Language, Os,
    ScrollRequest, editor_action_for,
};
use egui::{Event, Id, Key, Modifiers, Ui};

/// What the input of a frame did.
#[derive(Default)]
pub(crate) struct InputOut {
    pub text_changed: bool,
    pub scroll: Option<ScrollRequest>,
    pub hint: Option<String>,
    /// Anything was handled (so an Escape fell through to nobody).
    pub handled: bool,
}

/// What the input needs to know.
pub(crate) struct InputCfg<'a> {
    pub os: Os,
    pub viewer: bool,
    pub tab_indents: bool,
    pub language: &'a Language,
    pub first_visible_line: usize,
    pub visible_lines: usize,
}

/// The modifiers of `m` as the keymap sees them on `os`: Ctrl is Ctrl or Cmd-as-command on
/// Windows/Linux; Cmd is Cmd on macOS.
pub(crate) fn key_modifiers(m: Modifiers, os: Os) -> KeyModifiers {
    match os {
        Os::WindowsLinux => KeyModifiers {
            ctrl: m.ctrl || m.command,
            alt: m.alt,
            shift: m.shift,
            command: false,
        },
        Os::MacOs => KeyModifiers {
            ctrl: m.ctrl,
            alt: m.alt,
            shift: m.shift,
            command: m.mac_cmd || (m.command && !m.ctrl),
        },
    }
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

/// The last text the editor copied and whether it was whole lines, shared by all editors so a
/// line copied from one pastes above the caret line in another.
#[derive(Clone, Default)]
struct CopyMemo {
    text: String,
    whole_line: bool,
}

fn memo_id() -> Id {
    Id::new("archipel-editor-copy-memo")
}

/// Runs the input of this frame on `editor`.
pub(crate) fn process(
    ui: &Ui,
    editor: &mut Editor,
    cfg: &InputCfg<'_>,
    st: &mut ViewState,
) -> InputOut {
    let mut out = InputOut::default();
    let events = ui.input_mut(|i| std::mem::take(&mut i.events));
    let mut kept = Vec::with_capacity(events.len());
    let (mut copied, mut cut) = (false, false);
    for event in events {
        let consumed = match &event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => key_event(
                ui,
                editor,
                cfg,
                st,
                &mut out,
                *key,
                *modifiers,
                &mut copied,
                &mut cut,
            ),
            Event::Copy => {
                if copied {
                    true
                } else {
                    run(ui, editor, cfg, &mut out, &EditorAction::Copy, None);
                    true
                }
            }
            Event::Cut if !cfg.viewer => {
                if cut {
                    true
                } else {
                    run(ui, editor, cfg, &mut out, &EditorAction::Cut, None);
                    true
                }
            }
            Event::Paste(text) if !cfg.viewer => {
                let memo: CopyMemo = ui.data(|d| d.get_temp(memo_id())).unwrap_or_default();
                let content = if memo.whole_line && memo.text == *text {
                    ClipboardContent::lines(text.clone())
                } else {
                    ClipboardContent::text(text.clone())
                };
                run(
                    ui,
                    editor,
                    cfg,
                    &mut out,
                    &EditorAction::Paste,
                    Some(&content),
                );
                true
            }
            Event::Text(text) if !cfg.viewer => {
                for c in text.chars().filter(|c| !c.is_control()) {
                    run(ui, editor, cfg, &mut out, &EditorAction::Type(c), None);
                }
                true
            }
            Event::Ime(ime) if !cfg.viewer => {
                match ime {
                    egui::ImeEvent::Commit(text) => {
                        st.preedit = None;
                        for c in text.chars().filter(|c| !c.is_control()) {
                            run(ui, editor, cfg, &mut out, &EditorAction::Type(c), None);
                        }
                    }
                    egui::ImeEvent::Preedit(text) => {
                        st.preedit = (!text.is_empty()).then(|| text.clone());
                    }
                    _ => st.preedit = None,
                }
                true
            }
            _ => false,
        };
        if !consumed {
            kept.push(event);
        }
    }
    ui.input_mut(|i| i.events = kept);
    out
}

#[allow(clippy::too_many_arguments)]
fn key_event(
    ui: &Ui,
    editor: &mut Editor,
    cfg: &InputCfg<'_>,
    _st: &mut ViewState,
    out: &mut InputOut,
    key: Key,
    modifiers: Modifiers,
    copied: &mut bool,
    cut: &mut bool,
) -> bool {
    let Some(action) = editor_action_for(key, key_modifiers(modifiers, cfg.os), cfg.os) else {
        return false;
    };
    if cfg.viewer && is_editing(&action) {
        return false;
    }
    if !cfg.tab_indents && matches!(action, EditorAction::Tab | EditorAction::UnindentSelection) {
        return false;
    }
    match action {
        // The text arrives as `Event::Paste`.
        EditorAction::Paste | EditorAction::PasteSimple => false,
        EditorAction::Copy => {
            *copied = true;
            run(ui, editor, cfg, out, &action, None)
        }
        EditorAction::Cut => {
            *cut = true;
            run(ui, editor, cfg, out, &action, None)
        }
        _ => run(ui, editor, cfg, out, &action, None),
    }
}

/// Executes one action; returns whether it did something (so its key is consumed).
fn run(
    ui: &Ui,
    editor: &mut Editor,
    cfg: &InputCfg<'_>,
    out: &mut InputOut,
    action: &EditorAction,
    clipboard: Option<&ClipboardContent>,
) -> bool {
    let mut ctx = ActionContext::new(cfg.language)
        .with_viewport(cfg.first_visible_line, cfg.visible_lines.max(1));
    if let Some(clip) = clipboard {
        ctx = ctx.with_clipboard(clip);
    }
    let outcome = editor.execute(action, &ctx);
    if !outcome.handled {
        return false;
    }
    out.handled = true;
    out.text_changed |= outcome.text_changed;
    if outcome.scroll.is_some() {
        out.scroll = outcome.scroll;
    }
    if outcome.hint.is_some() {
        out.hint = outcome.hint.clone();
    }
    if let Some(content) = outcome.clipboard {
        ui.ctx().copy_text(content.text.clone());
        ui.data_mut(|d| {
            d.insert_temp(
                memo_id(),
                CopyMemo {
                    text: content.text,
                    whole_line: content.whole_line,
                },
            )
        });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_and_command_both_mean_ctrl_on_windows_and_linux() {
        assert!(key_modifiers(Modifiers::COMMAND, Os::WindowsLinux).ctrl);
        assert!(key_modifiers(Modifiers::CTRL, Os::WindowsLinux).ctrl);
        assert!(!key_modifiers(Modifiers::COMMAND, Os::WindowsLinux).command);
    }

    #[test]
    fn command_is_cmd_on_macos_and_ctrl_stays_ctrl() {
        assert!(key_modifiers(Modifiers::COMMAND, Os::MacOs).command);
        assert!(key_modifiers(Modifiers::MAC_CMD, Os::MacOs).command);
        let ctrl = key_modifiers(Modifiers::CTRL, Os::MacOs);
        assert!(ctrl.ctrl && !ctrl.command);
    }

    #[test]
    fn editing_actions_are_marked() {
        assert!(is_editing(&EditorAction::Type('a')));
        assert!(is_editing(&EditorAction::Undo));
        assert!(!is_editing(&EditorAction::Copy));
        assert!(!is_editing(&EditorAction::Left));
        assert!(!is_editing(&EditorAction::SelectAll));
    }
}
