//! What the kit needs from the application that embeds it: services as traits, with small
//! in-memory implementations for tests and small apps.
//!
//! IntelliJ reaches these through global `getInstance()` application services. The kit has no
//! globals, so the host passes the implementation to whatever needs it. Persistence, the
//! action registry and the clock stay on the host's side of this line (no filesystem or app
//! policy in the kit).

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::time::Instant;

use crate::action_system::{ActionGroup, AnAction, Keymap, Os};

/// The registry of actions and groups by id.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.ActionManager`, without the
/// toolbar and popup factories (the kit's components do that) and the timer listeners
/// (immediate mode updates every frame).
pub trait ActionManager {
    /// The action registered under `id` (IntelliJ `getAction`).
    fn action(&self, id: &str) -> Option<Rc<dyn AnAction>>;

    /// The group registered under `id` (IntelliJ `getAction` returning an `ActionGroup`).
    fn group(&self, id: &str) -> Option<Rc<dyn ActionGroup>>;

    /// Registers an action under its own [`AnAction::id`], replacing an earlier one
    /// (IntelliJ `registerAction`).
    fn register_action(&mut self, action: Rc<dyn AnAction>);

    /// Registers a group under `id`, replacing an earlier one.
    fn register_group(&mut self, id: &str, group: Rc<dyn ActionGroup>);

    /// Removes whatever is registered under `id` (IntelliJ `unregisterAction`). Returns
    /// whether something was.
    fn unregister(&mut self, id: &str) -> bool;

    /// The ids starting with `prefix`, sorted (IntelliJ `getActionIds(prefix)`).
    fn action_ids(&self, prefix: &str) -> Vec<String>;

    /// Whether `id` names a group (IntelliJ `isGroup`).
    fn is_group(&self, id: &str) -> bool {
        self.group(id).is_some()
    }
}

/// A [`HashMap`] backed [`ActionManager`].
#[derive(Default)]
pub struct SimpleActionManager {
    actions: HashMap<String, Rc<dyn AnAction>>,
    groups: HashMap<String, Rc<dyn ActionGroup>>,
}

impl SimpleActionManager {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ActionManager for SimpleActionManager {
    fn action(&self, id: &str) -> Option<Rc<dyn AnAction>> {
        self.actions.get(id).cloned()
    }

    fn group(&self, id: &str) -> Option<Rc<dyn ActionGroup>> {
        self.groups.get(id).cloned()
    }

    fn register_action(&mut self, action: Rc<dyn AnAction>) {
        let id = action.id().to_owned();
        self.groups.remove(&id);
        self.actions.insert(id, action);
    }

    fn register_group(&mut self, id: &str, group: Rc<dyn ActionGroup>) {
        self.actions.remove(id);
        self.groups.insert(id.to_owned(), group);
    }

    fn unregister(&mut self, id: &str) -> bool {
        self.actions.remove(id).is_some() | self.groups.remove(id).is_some()
    }

    fn action_ids(&self, prefix: &str) -> Vec<String> {
        let mut ids: Vec<String> = self
            .actions
            .keys()
            .chain(self.groups.keys())
            .filter(|id| id.starts_with(prefix))
            .cloned()
            .collect();
        ids.sort();
        ids
    }
}

/// The source of the active keymap.
///
/// IntelliJ counterpart: `com.intellij.openapi.keymap.KeymapManager` (`getActiveKeymap`).
pub trait KeymapManager {
    /// The keymap in effect (IntelliJ `getActiveKeymap`).
    fn active_keymap(&self) -> &Keymap;

    /// The operating system whose conventions shortcut texts follow.
    fn os(&self) -> Os;

    /// Whether shortcut texts use macOS symbols.
    fn is_mac(&self) -> bool {
        self.os().is_mac()
    }
}

/// A [`KeymapManager`] holding one keymap.
#[derive(Debug, Clone)]
pub struct SimpleKeymapManager {
    keymap: Keymap,
    os: Os,
}

impl SimpleKeymapManager {
    /// The default keymap of `os`.
    pub fn new(os: Os) -> Self {
        Self {
            keymap: Keymap::default_for(os),
            os,
        }
    }

    /// The default keymap of the running OS.
    pub fn for_current_os() -> Self {
        Self::new(Os::current())
    }

    /// Replaces the keymap.
    pub fn set_keymap(&mut self, keymap: Keymap) {
        self.keymap = keymap;
    }

    /// Mutable access, to add or remove shortcuts.
    pub fn keymap_mut(&mut self) -> &mut Keymap {
        &mut self.keymap
    }
}

impl KeymapManager for SimpleKeymapManager {
    fn active_keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn os(&self) -> Os {
        self.os
    }
}

/// String key-value storage for persisted UI state: split proportions, popup sizes, search
/// history, "do not show again" flags. The host decides where it lives (a file, the registry).
///
/// IntelliJ counterpart: `com.intellij.ide.util.PropertiesComponent` (`getValue`, `setValue`,
/// `isValueSet`, `unsetValue`, `getInt`, `getBoolean`).
pub trait PropertiesComponent {
    /// The stored value (IntelliJ `getValue`).
    fn get(&self, key: &str) -> Option<String>;

    /// Stores a value (IntelliJ `setValue`).
    fn set(&mut self, key: &str, value: &str);

    /// Forgets a value (IntelliJ `unsetValue`).
    fn unset(&mut self, key: &str);

    /// Whether a value is stored (IntelliJ `isValueSet`).
    fn is_set(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// A stored boolean, or `default` when unset or not `true`/`false` (IntelliJ
    /// `getBoolean`).
    fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.get(key).as_deref() {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        }
    }

    /// Stores a boolean (IntelliJ `setValue(String, boolean)`).
    fn set_bool(&mut self, key: &str, value: bool) {
        self.set(key, if value { "true" } else { "false" });
    }

    /// A stored integer, or `default` when unset or not a number (IntelliJ `getInt`).
    fn get_int(&self, key: &str, default: i64) -> i64 {
        self.get(key)
            .and_then(|text| text.parse().ok())
            .unwrap_or(default)
    }

    /// Stores an integer (IntelliJ `setValue(String, int)`).
    fn set_int(&mut self, key: &str, value: i64) {
        self.set(key, &value.to_string());
    }
}

/// A [`PropertiesComponent`] in memory; nothing survives the process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryProperties {
    values: BTreeMap<String, String>,
}

impl InMemoryProperties {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// All entries in key order, for the host to save.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

impl PropertiesComponent for InMemoryProperties {
    fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_owned(), value.to_owned());
    }

    fn unset(&mut self, key: &str) {
        self.values.remove(key);
    }
}

/// A monotonic time source, in seconds.
///
/// IntelliJ counterpart: the timing of `Alarm` and `javax.swing.Timer`. Immediate-mode
/// components take `now` per frame; a `Clock` lets a host and tests supply it (egui's
/// `InputState::time` is one too).
pub trait Clock {
    /// Seconds since an arbitrary start; never goes backwards.
    fn now(&self) -> f64;
}

/// The process clock, from [`Instant`].
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    start: Instant,
}

impl SystemClock {
    /// A clock that starts at zero now.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

/// A clock tests move by hand.
#[derive(Debug, Default)]
pub struct ManualClock {
    now: Cell<f64>,
}

impl ManualClock {
    /// A clock at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the clock forward by `seconds`; negative values are ignored.
    pub fn advance(&self, seconds: f64) {
        if seconds > 0.0 {
            self.now.set(self.now.get() + seconds);
        }
    }
}

impl Clock for ManualClock {
    fn now(&self) -> f64 {
        self.now.get()
    }
}
