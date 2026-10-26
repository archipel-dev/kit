//! Typed data for an action update: [`DataContext`] and [`DataKey`].

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

/// A named, typed key into a [`DataContext`].
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.DataKey<T>`. IntelliJ interns keys
/// in a global table; here a key is a `const` value (name plus type), so there is no global
/// registry. Two keys with the same name but different types are different keys.
///
/// ```
/// use archipel_kit::action_system::DataKey;
///
/// const ROW: DataKey<usize> = DataKey::new("example.row");
/// assert_eq!(ROW.name(), "example.row");
/// ```
pub struct DataKey<T: 'static> {
    name: &'static str,
    marker: PhantomData<fn() -> T>,
}

impl<T: 'static> DataKey<T> {
    /// Creates a key (IntelliJ `DataKey.create(name)`).
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }

    /// The key's name (IntelliJ `DataKey.getName`).
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl<T: 'static> Clone for DataKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for DataKey<T> {}

impl<T: 'static> std::fmt::Debug for DataKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DataKey({})", self.name)
    }
}

/// The data an action's `update` and `action_performed` can read: what is selected, which
/// file, which tool window.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.DataContext`. IntelliJ builds it
/// by walking the focused component's parents; immediate mode has no parent chain, so the
/// caller builds the context explicitly each frame and chains contexts with
/// [`with_parent`](Self::with_parent) to mimic that walk: a lookup that misses falls through
/// to the parent.
///
/// ```
/// use archipel_kit::action_system::{CommonDataKeys, DataContext};
/// use std::rc::Rc;
///
/// let window = Rc::new(
///     DataContext::builder()
///         .with(CommonDataKeys::TOOL_WINDOW_ID, "Run".to_owned())
///         .build(),
/// );
/// let list = DataContext::with_parent(window)
///     .with(CommonDataKeys::SELECTED_ITEMS, vec!["a.rs".to_owned()])
///     .build();
/// assert_eq!(list.get(CommonDataKeys::SELECTED_ITEMS), Some(&vec!["a.rs".to_owned()]));
/// assert_eq!(list.get(CommonDataKeys::TOOL_WINDOW_ID).map(String::as_str), Some("Run"));
/// ```
#[derive(Default)]
pub struct DataContext {
    values: HashMap<(&'static str, TypeId), Box<dyn Any>>,
    parent: Option<Rc<DataContext>>,
}

impl DataContext {
    /// An empty context with no parent (IntelliJ `DataContext.EMPTY_CONTEXT`).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Starts building a context.
    pub fn builder() -> DataContextBuilder {
        DataContextBuilder {
            context: Self::default(),
        }
    }

    /// Starts building a context whose misses fall through to `parent` (IntelliJ looks data
    /// up along the component's parents).
    pub fn with_parent(parent: Rc<DataContext>) -> DataContextBuilder {
        DataContextBuilder {
            context: Self {
                values: HashMap::new(),
                parent: Some(parent),
            },
        }
    }

    /// The value for `key`, here or in the nearest parent that has it (IntelliJ
    /// `DataContext.getData(DataKey)`).
    pub fn get<T: 'static>(&self, key: DataKey<T>) -> Option<&T> {
        let mut current = self;
        loop {
            if let Some(value) = current.values.get(&(key.name, TypeId::of::<T>())) {
                return value.downcast_ref::<T>();
            }
            current = current.parent.as_deref()?;
        }
    }

    /// Whether `key` has a value here or in a parent.
    pub fn contains<T: 'static>(&self, key: DataKey<T>) -> bool {
        self.get(key).is_some()
    }

    /// The parent context, if any.
    pub fn parent(&self) -> Option<&Rc<DataContext>> {
        self.parent.as_ref()
    }
}

impl std::fmt::Debug for DataContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut names: Vec<_> = self.values.keys().map(|(name, _)| *name).collect();
        names.sort_unstable();
        f.debug_struct("DataContext")
            .field("keys", &names)
            .field("parent", &self.parent)
            .finish()
    }
}

/// Builds a [`DataContext`] (IntelliJ fills one through `DataSink.set`).
pub struct DataContextBuilder {
    context: DataContext,
}

impl DataContextBuilder {
    /// Sets the value of `key`, replacing an earlier value (IntelliJ `DataSink.set`).
    #[must_use]
    pub fn with<T: 'static>(mut self, key: DataKey<T>, value: T) -> Self {
        self.context
            .values
            .insert((key.name, TypeId::of::<T>()), Box::new(value));
        self
    }

    /// Sets the parent context.
    #[must_use]
    pub fn parent(mut self, parent: Rc<DataContext>) -> Self {
        self.context.parent = Some(parent);
        self
    }

    /// Finishes the context.
    pub fn build(self) -> DataContext {
        self.context
    }
}

/// The data keys that make sense without a project model.
///
/// IntelliJ counterpart: `CommonDataKeys` and `PlatformDataKeys`. IntelliJ's `PROJECT`,
/// `EDITOR`, `PSI_*` and `NAVIGATABLE` keys have no equivalent in the kit; hosts define their
/// own [`DataKey`]s for those next to these.
pub struct CommonDataKeys;

impl CommonDataKeys {
    /// The selected rows of the focused list, tree or table, as display strings (IntelliJ
    /// `PlatformCoreDataKeys.SELECTED_ITEMS`).
    pub const SELECTED_ITEMS: DataKey<Vec<String>> = DataKey::new("selectedItems");
    /// The selected character range of the focused text, as byte offsets (IntelliJ
    /// `SelectionModel.getSelectionStart` and `getSelectionEnd`, reached through
    /// `CommonDataKeys.EDITOR`).
    pub const EDITOR_TEXT_RANGE: DataKey<Range<usize>> = DataKey::new("editor.textRange");
    /// The path of the file in front, as the host names it (IntelliJ
    /// `CommonDataKeys.VIRTUAL_FILE`). The kit never touches the filesystem with it.
    pub const FILE_PATH: DataKey<PathBuf> = DataKey::new("virtualFile");
    /// The id of the tool window that has focus (IntelliJ `PlatformDataKeys.TOOL_WINDOW`).
    pub const TOOL_WINDOW_ID: DataKey<String> = DataKey::new("toolWindow");
    /// The text typed into the focused list's speed search (IntelliJ
    /// `PlatformDataKeys.SPEED_SEARCH_TEXT`).
    pub const SPEED_SEARCH_TEXT: DataKey<String> = DataKey::new("speedSearchText");
    /// Where a context menu was requested, in screen points (IntelliJ
    /// `PlatformDataKeys.CONTEXT_MENU_POINT`).
    pub const CONTEXT_MENU_POINT: DataKey<egui::Pos2> = DataKey::new("contextMenuPoint");
    /// The widget that has focus (IntelliJ `PlatformCoreDataKeys.CONTEXT_COMPONENT`).
    pub const CONTEXT_COMPONENT: DataKey<egui::Id> = DataKey::new("contextComponent");
}
