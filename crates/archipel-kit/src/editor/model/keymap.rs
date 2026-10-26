//! The default JetBrains editor keymap (spec section 12), Windows/Linux and macOS.

use egui::Key;

use super::action::EditorAction;

/// The platform whose keymap applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// The default keymap.
    WindowsLinux,
    /// The "macOS 10.5+" keymap: Cmd where the default has Ctrl, plus Emacs-style Ctrl keys.
    MacOs,
}

impl Os {
    /// The platform this program runs on.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::WindowsLinux
        }
    }
}

/// Modifier keys of a shortcut. `command` is Cmd on macOS and unused elsewhere; on
/// Windows/Linux Ctrl is `ctrl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    /// Ctrl.
    pub ctrl: bool,
    /// Alt (Option on macOS).
    pub alt: bool,
    /// Shift.
    pub shift: bool,
    /// Cmd (macOS).
    pub command: bool,
}

impl KeyModifiers {
    /// No modifier.
    pub const NONE: Self = Self::new(false, false, false, false);
    /// Shift.
    pub const SHIFT: Self = Self::new(false, false, true, false);
    /// Ctrl.
    pub const CTRL: Self = Self::new(true, false, false, false);
    /// Alt (Option).
    pub const ALT: Self = Self::new(false, true, false, false);
    /// Cmd.
    pub const CMD: Self = Self::new(false, false, false, true);

    /// Builds a set.
    pub const fn new(ctrl: bool, alt: bool, shift: bool, command: bool) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            command,
        }
    }

    /// The same with Shift added.
    pub const fn plus_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// The same with Alt added.
    pub const fn plus_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// The same with Ctrl added.
    pub const fn plus_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// The same with Cmd added.
    pub const fn plus_cmd(mut self) -> Self {
        self.command = true;
        self
    }
}

impl From<egui::Modifiers> for KeyModifiers {
    fn from(modifiers: egui::Modifiers) -> Self {
        Self {
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            command: modifiers.mac_cmd,
        }
    }
}

/// One shortcut of the keymap.
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    /// The key.
    pub key: Key,
    /// The modifiers, exactly.
    pub modifiers: KeyModifiers,
    /// The action.
    pub action: fn() -> EditorAction,
}

macro_rules! bind {
    ($list:ident, $key:expr, $mods:expr, $action:expr) => {
        $list.push(Binding {
            key: $key,
            modifiers: $mods,
            action: || $action,
        });
    };
}

/// The actions bound to Shift-less / Shift pairs of a motion.
fn motion(
    list: &mut Vec<Binding>,
    key: Key,
    mods: KeyModifiers,
    plain: fn() -> EditorAction,
    selecting: fn() -> EditorAction,
) {
    list.push(Binding {
        key,
        modifiers: mods,
        action: plain,
    });
    list.push(Binding {
        key,
        modifiers: mods.plus_shift(),
        action: selecting,
    });
}

fn common(list: &mut Vec<Binding>, os: Os) {
    use EditorAction as A;
    let none = KeyModifiers::NONE;
    // `primary` is the key the platform uses where the default keymap says Ctrl.
    let primary = match os {
        Os::WindowsLinux => KeyModifiers::CTRL,
        Os::MacOs => KeyModifiers::CMD,
    };
    motion(
        list,
        Key::ArrowLeft,
        none,
        || A::Left,
        || A::LeftWithSelection,
    );
    motion(
        list,
        Key::ArrowRight,
        none,
        || A::Right,
        || A::RightWithSelection,
    );
    motion(list, Key::ArrowUp, none, || A::Up, || A::UpWithSelection);
    motion(
        list,
        Key::ArrowDown,
        none,
        || A::Down,
        || A::DownWithSelection,
    );
    motion(
        list,
        Key::Home,
        none,
        || A::LineStart,
        || A::LineStartWithSelection,
    );
    motion(
        list,
        Key::End,
        none,
        || A::LineEnd,
        || A::LineEndWithSelection,
    );
    motion(
        list,
        Key::PageUp,
        none,
        || A::PageUp,
        || A::PageUpWithSelection,
    );
    motion(
        list,
        Key::PageDown,
        none,
        || A::PageDown,
        || A::PageDownWithSelection,
    );
    motion(
        list,
        Key::Home,
        primary,
        || A::TextStart,
        || A::TextStartWithSelection,
    );
    motion(
        list,
        Key::End,
        primary,
        || A::TextEnd,
        || A::TextEndWithSelection,
    );
    motion(
        list,
        Key::PageUp,
        primary,
        || A::MoveToPageTop,
        || A::MoveToPageTopWithSelection,
    );
    motion(
        list,
        Key::PageDown,
        primary,
        || A::MoveToPageBottom,
        || A::MoveToPageBottomWithSelection,
    );

    bind!(list, Key::Enter, none, A::Enter);
    bind!(list, Key::Enter, KeyModifiers::SHIFT, A::StartNewLine);
    bind!(list, Key::Tab, none, A::Tab);
    bind!(list, Key::Tab, KeyModifiers::SHIFT, A::UnindentSelection);
    bind!(list, Key::Backspace, none, A::Backspace);
    bind!(list, Key::Backspace, KeyModifiers::SHIFT, A::Backspace);
    bind!(list, Key::Delete, none, A::Delete);
    bind!(list, Key::Escape, none, A::Escape);
    bind!(list, Key::Insert, none, A::ToggleInsertState);
    bind!(list, Key::Delete, KeyModifiers::SHIFT, A::Cut);
    bind!(list, Key::Insert, KeyModifiers::SHIFT, A::Paste);
    bind!(list, Key::Insert, primary, A::Copy);
    bind!(list, Key::D, primary, A::Duplicate);
    bind!(list, Key::X, primary, A::Cut);
    bind!(list, Key::C, primary, A::Copy);
    bind!(list, Key::V, primary, A::Paste);
    bind!(
        list,
        Key::V,
        primary.plus_alt().plus_shift(),
        A::PasteSimple
    );
    bind!(list, Key::Z, primary, A::Undo);
    bind!(list, Key::Z, primary.plus_shift(), A::Redo);
    bind!(list, Key::A, primary, A::SelectAll);
    bind!(list, Key::Slash, primary, A::CommentByLineComment);
    bind!(list, Key::Enter, primary, A::SplitLine);
    bind!(list, Key::U, primary.plus_shift(), A::ToggleCase);
    bind!(
        list,
        Key::ArrowUp,
        KeyModifiers::ALT.plus_shift(),
        A::MoveLineUp
    );
    bind!(
        list,
        Key::ArrowDown,
        KeyModifiers::ALT.plus_shift(),
        A::MoveLineDown
    );
    bind!(list, Key::ArrowUp, primary.plus_shift(), A::MoveStatementUp);
    bind!(
        list,
        Key::ArrowDown,
        primary.plus_shift(),
        A::MoveStatementDown
    );
    bind!(
        list,
        Key::G,
        KeyModifiers::ALT.plus_shift(),
        A::AddCaretPerSelectedLine
    );
    // Folding: Ctrl/Cmd with + or =, and - for collapse; Alt = recursively, Shift = all.
    for key in [Key::Plus, Key::Equals] {
        bind!(list, key, primary, A::ExpandRegion);
        bind!(list, key, primary.plus_alt(), A::ExpandRegionRecursively);
        bind!(list, key, primary.plus_shift(), A::ExpandAllRegions);
    }
    bind!(list, Key::Minus, primary, A::CollapseRegion);
    bind!(
        list,
        Key::Minus,
        primary.plus_alt(),
        A::CollapseRegionRecursively
    );
    bind!(
        list,
        Key::Minus,
        primary.plus_shift(),
        A::CollapseAllRegions
    );
}

fn windows_linux(list: &mut Vec<Binding>) {
    use EditorAction as A;
    let ctrl = KeyModifiers::CTRL;
    motion(
        list,
        Key::ArrowLeft,
        ctrl,
        || A::PreviousWord,
        || A::PreviousWordWithSelection,
    );
    motion(
        list,
        Key::ArrowRight,
        ctrl,
        || A::NextWord,
        || A::NextWordWithSelection,
    );
    bind!(list, Key::ArrowUp, ctrl, A::ScrollUp);
    bind!(list, Key::ArrowDown, ctrl, A::ScrollDown);
    bind!(list, Key::M, ctrl, A::ScrollToCenter);
    motion(
        list,
        Key::OpenBracket,
        ctrl,
        || A::CodeBlockStart,
        || A::CodeBlockStartWithSelection,
    );
    motion(
        list,
        Key::CloseBracket,
        ctrl,
        || A::CodeBlockEnd,
        || A::CodeBlockEndWithSelection,
    );
    bind!(list, Key::M, ctrl.plus_shift(), A::MatchBrace);
    bind!(list, Key::W, ctrl, A::SelectWord);
    bind!(list, Key::W, ctrl.plus_shift(), A::UnSelectWord);
    bind!(list, Key::J, KeyModifiers::ALT, A::SelectNextOccurrence);
    bind!(
        list,
        Key::J,
        KeyModifiers::ALT.plus_shift(),
        A::UnselectPreviousOccurrence
    );
    bind!(
        list,
        Key::J,
        ctrl.plus_alt().plus_shift(),
        A::SelectAllOccurrences
    );
    bind!(
        list,
        Key::Insert,
        KeyModifiers::ALT.plus_shift(),
        A::ToggleColumnMode
    );
    bind!(list, Key::Enter, ctrl.plus_alt(), A::StartNewLineBefore);
    bind!(list, Key::Enter, ctrl.plus_shift(), A::CompleteStatement);
    bind!(list, Key::Backspace, ctrl, A::DeleteToWordStart);
    bind!(list, Key::Delete, ctrl, A::DeleteToWordEnd);
    bind!(list, Key::Y, ctrl, A::DeleteLine);
    bind!(list, Key::J, ctrl.plus_shift(), A::JoinLines);
    bind!(
        list,
        Key::Slash,
        ctrl.plus_shift(),
        A::CommentByBlockComment
    );
    bind!(list, Key::Backspace, KeyModifiers::ALT, A::Undo);
    bind!(
        list,
        Key::Backspace,
        KeyModifiers::ALT.plus_shift(),
        A::Redo
    );
    // Ctrl+Alt: recursive fold keys use Alt on top of Ctrl, already in `common`.
}

fn mac(list: &mut Vec<Binding>) {
    use EditorAction as A;
    let cmd = KeyModifiers::CMD;
    let ctrl = KeyModifiers::CTRL;
    let option = KeyModifiers::ALT;
    // Emacs-style keys.
    bind!(list, Key::B, ctrl, A::Left);
    bind!(list, Key::F, ctrl, A::Right);
    bind!(list, Key::P, ctrl, A::Up);
    bind!(list, Key::N, ctrl, A::Down);
    bind!(list, Key::A, ctrl, A::LineStart);
    bind!(list, Key::E, ctrl, A::LineEnd);
    // Words: Option+Arrow.
    motion(
        list,
        Key::ArrowLeft,
        option,
        || A::PreviousWord,
        || A::PreviousWordWithSelection,
    );
    motion(
        list,
        Key::ArrowRight,
        option,
        || A::NextWord,
        || A::NextWordWithSelection,
    );
    bind!(list, Key::B, ctrl.plus_alt(), A::PreviousWord);
    bind!(list, Key::F, ctrl.plus_alt(), A::NextWord);
    bind!(
        list,
        Key::B,
        ctrl.plus_alt().plus_shift(),
        A::PreviousWordWithSelection
    );
    bind!(
        list,
        Key::F,
        ctrl.plus_alt().plus_shift(),
        A::NextWordWithSelection
    );
    // Line start/end: Cmd+Left/Right.
    motion(
        list,
        Key::ArrowLeft,
        cmd,
        || A::LineStart,
        || A::LineStartWithSelection,
    );
    motion(
        list,
        Key::ArrowRight,
        cmd,
        || A::LineEnd,
        || A::LineEndWithSelection,
    );
    bind!(list, Key::L, ctrl, A::ScrollToCenter);
    motion(
        list,
        Key::OpenBracket,
        cmd.plus_alt(),
        || A::CodeBlockStart,
        || A::CodeBlockStartWithSelection,
    );
    motion(
        list,
        Key::CloseBracket,
        cmd.plus_alt(),
        || A::CodeBlockEnd,
        || A::CodeBlockEndWithSelection,
    );
    bind!(list, Key::M, ctrl, A::MatchBrace);
    bind!(list, Key::ArrowUp, option, A::SelectWord);
    bind!(list, Key::ArrowDown, option, A::UnSelectWord);
    bind!(list, Key::G, ctrl, A::SelectNextOccurrence);
    bind!(
        list,
        Key::G,
        ctrl.plus_shift(),
        A::UnselectPreviousOccurrence
    );
    bind!(list, Key::G, ctrl.plus_cmd(), A::SelectAllOccurrences);
    bind!(list, Key::Num8, cmd.plus_shift(), A::ToggleColumnMode);
    bind!(list, Key::Enter, cmd.plus_alt(), A::StartNewLineBefore);
    bind!(list, Key::Enter, cmd.plus_shift(), A::CompleteStatement);
    bind!(list, Key::Backspace, option, A::DeleteToWordStart);
    bind!(list, Key::Delete, option, A::DeleteToWordEnd);
    bind!(list, Key::Backspace, cmd, A::DeleteLine);
    bind!(list, Key::K, ctrl, A::CutLineEnd);
    bind!(list, Key::J, ctrl.plus_shift(), A::JoinLines);
    bind!(list, Key::Slash, cmd.plus_alt(), A::CommentByBlockComment);
    bind!(
        list,
        Key::Slash,
        ctrl.plus_shift(),
        A::CommentByBlockComment
    );
    bind!(list, Key::Slash, cmd.plus_shift(), A::CommentByBlockComment);
    bind!(list, Key::Backspace, option.plus_shift(), A::Redo);
}

/// All bindings of the default keymap for `os`.
pub fn default_bindings(os: Os) -> Vec<Binding> {
    let mut list = Vec::new();
    common(&mut list, os);
    match os {
        Os::WindowsLinux => windows_linux(&mut list),
        Os::MacOs => mac(&mut list),
    }
    list
}

/// The editor action bound to a key chord (IntelliJ `KeymapManager` lookup), or `None`.
///
/// Typed characters are not keys: feed them as [`EditorAction::Type`]. Chords of actions the
/// model does not implement (Find, Go to line, completion, ...) return `None`.
pub fn editor_action_for(key: Key, modifiers: KeyModifiers, os: Os) -> Option<EditorAction> {
    default_bindings(os)
        .into_iter()
        .find(|binding| binding.key == key && binding.modifiers == modifiers)
        .map(|binding| (binding.action)())
}
