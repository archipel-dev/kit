//! The selection of a list, tree or table: IntelliJ's (and Swing's) `ListSelectionModel`.

use std::collections::BTreeSet;

/// How many rows a [`ListSelectionModel`] can hold (`ListSelectionModel.SINGLE_SELECTION`,
/// `SINGLE_INTERVAL_SELECTION`, `MULTIPLE_INTERVAL_SELECTION`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// At most one row.
    Single,
    /// One run of neighboring rows: <kbd>Shift</kbd>-click extends it, <kbd>Ctrl</kbd>-click
    /// starts a new one.
    SingleInterval,
    /// Any rows: <kbd>Ctrl</kbd>/<kbd>Cmd</kbd>-click toggles a row, <kbd>Shift</kbd>-click
    /// extends from the anchor.
    #[default]
    MultipleInterval,
}

/// The selected rows of a collection, owned by the caller and passed `&mut` to
/// [`JBList`](crate::ui::JBList), [`Tree`](crate::ui::Tree), [`JBTable`](crate::ui::JBTable)
/// or [`TreeTable`](crate::ui::TreeTable) with `with_selection`.
///
/// Like Swing's model it knows an **anchor** (where a <kbd>Shift</kbd> selection starts) and
/// a **lead** (the row the keyboard moves from), besides the selected indices. Indices are
/// the row indices you pass to the collection.
///
/// ```
/// use archipel_kit::ui::ListSelectionModel;
///
/// let mut selection = ListSelectionModel::multiple();
/// selection.select(2);
/// selection.set_selection_interval(2, 4);
/// selection.toggle(3);
/// assert_eq!(selection.iter().collect::<Vec<_>>(), vec![2, 4]);
/// assert_eq!(selection.lead(), Some(3));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListSelectionModel {
    mode: SelectionMode,
    selected: BTreeSet<usize>,
    anchor: Option<usize>,
    lead: Option<usize>,
}

impl ListSelectionModel {
    /// An empty selection in `mode`.
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    /// An empty selection of at most one row.
    pub fn single() -> Self {
        Self::new(SelectionMode::Single)
    }

    /// An empty selection of any rows.
    pub fn multiple() -> Self {
        Self::new(SelectionMode::MultipleInterval)
    }

    /// The selection mode.
    pub fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// Changes the mode; a selection that does not fit is cut down to the lead row (or to the
    /// first interval).
    pub fn set_mode(&mut self, mode: SelectionMode) {
        self.mode = mode;
        match mode {
            SelectionMode::Single => {
                if let Some(keep) = self.lead.filter(|lead| self.selected.contains(lead)) {
                    self.selected.clear();
                    self.selected.insert(keep);
                } else if let Some(first) = self.first() {
                    self.selected.clear();
                    self.selected.insert(first);
                }
            }
            SelectionMode::SingleInterval => {
                if let Some(first) = self.first() {
                    let end = self
                        .selected
                        .range(first..)
                        .zip(first..)
                        .take_while(|(index, expected)| *index == expected)
                        .count();
                    self.selected.retain(|index| *index < first + end);
                }
            }
            SelectionMode::MultipleInterval => {}
        }
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    /// How many rows are selected.
    pub fn len(&self) -> usize {
        self.selected.len()
    }

    /// Whether row `index` is selected.
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }

    /// The selected rows, in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.selected.iter().copied()
    }

    /// The first (smallest) selected row.
    pub fn first(&self) -> Option<usize> {
        self.selected.first().copied()
    }

    /// The last (largest) selected row.
    pub fn last(&self) -> Option<usize> {
        self.selected.last().copied()
    }

    /// The anchor: where a <kbd>Shift</kbd> selection starts.
    pub fn anchor(&self) -> Option<usize> {
        self.anchor
    }

    /// The lead: the row the keyboard moves from, usually the last one clicked or reached.
    pub fn lead(&self) -> Option<usize> {
        self.lead
    }

    /// Runs `change` and reports whether the set of selected rows changed.
    fn changing(&mut self, change: impl FnOnce(&mut Self)) -> bool {
        let before = self.selected.clone();
        change(self);
        self.selected != before
    }

    /// Unselects everything (the anchor and the lead stay, as in Swing). Returns whether the
    /// selection changed; so do the other methods that change it.
    pub fn clear(&mut self) -> bool {
        self.changing(|model| model.selected.clear())
    }

    /// Selects only `index`, and makes it the anchor and the lead.
    pub fn select(&mut self, index: usize) -> bool {
        self.changing(|model| {
            model.selected.clear();
            model.selected.insert(index);
            model.anchor = Some(index);
            model.lead = Some(index);
        })
    }

    /// Selects the rows from `anchor` to `lead` (either order) and nothing else; `anchor`
    /// becomes the anchor and `lead` the lead. In [`SelectionMode::Single`] only `lead` is
    /// selected.
    pub fn set_selection_interval(&mut self, anchor: usize, lead: usize) -> bool {
        self.changing(|model| {
            model.selected.clear();
            model.fill(anchor, lead);
            model.anchor = Some(anchor);
            model.lead = Some(lead);
        })
    }

    /// Adds the rows from `anchor` to `lead` to the selection. Outside
    /// [`SelectionMode::MultipleInterval`] this replaces the selection, as Swing does.
    pub fn add_selection_interval(&mut self, anchor: usize, lead: usize) -> bool {
        if self.mode == SelectionMode::MultipleInterval {
            self.changing(|model| {
                model.fill(anchor, lead);
                model.anchor = Some(anchor);
                model.lead = Some(lead);
            })
        } else {
            self.set_selection_interval(anchor, lead)
        }
    }

    /// Removes the rows from `from` to `to` (either order) from the selection.
    pub fn remove_selection_interval(&mut self, from: usize, to: usize) -> bool {
        self.changing(|model| {
            let (low, high) = (from.min(to), from.max(to));
            model.selected.retain(|index| *index < low || *index > high);
            model.anchor = Some(from);
            model.lead = Some(to);
        })
    }

    /// Toggles row `index`, and makes it the anchor and the lead.
    pub fn toggle(&mut self, index: usize) -> bool {
        if self.selected.remove(&index) {
            self.anchor = Some(index);
            self.lead = Some(index);
            true
        } else if self.mode == SelectionMode::MultipleInterval {
            self.selected.insert(index);
            self.anchor = Some(index);
            self.lead = Some(index);
            true
        } else {
            self.select(index)
        }
    }

    /// Selects rows `0..len` (only in [`SelectionMode::MultipleInterval`]; the lead
    /// becomes the last row, the anchor the first).
    pub fn select_all(&mut self, len: usize) -> bool {
        if len == 0 || self.mode != SelectionMode::MultipleInterval {
            return false;
        }
        self.changing(|model| {
            model.selected = (0..len).collect();
            model.anchor = Some(0);
            model.lead = Some(len - 1);
        })
    }

    /// Moves the lead to `index` without changing what is selected.
    pub fn set_lead(&mut self, index: usize) {
        self.lead = Some(index);
    }

    /// Sets the anchor and the lead without changing what is selected.
    pub fn set_anchor_and_lead(&mut self, anchor: usize, lead: usize) {
        self.anchor = Some(anchor);
        self.lead = Some(lead);
    }

    /// Drops every row at or after `len` (the collection got shorter). Returns whether the
    /// selection changed.
    pub fn clamp(&mut self, len: usize) -> bool {
        let before = self.selected.len();
        self.selected.retain(|index| *index < len);
        let mut changed = self.selected.len() != before;
        let last = len.checked_sub(1);
        for point in [&mut self.anchor, &mut self.lead] {
            if point.is_some_and(|index| index >= len) {
                *point = last;
                changed = true;
            }
        }
        changed
    }

    fn fill(&mut self, anchor: usize, lead: usize) {
        match self.mode {
            SelectionMode::Single => {
                self.selected.clear();
                self.selected.insert(lead);
            }
            _ => {
                let (low, high) = (anchor.min(lead), anchor.max(lead));
                self.selected.extend(low..=high);
            }
        }
    }
}
