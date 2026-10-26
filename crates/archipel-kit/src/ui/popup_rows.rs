//! A plain list of rows for the popup engine, shared by the combo box and the search
//! history: one level, one final choice.

use super::popup::{Chosen, LevelSpec, PopupSource, RowSpec};

/// The rows of a one-level list popup and the row chosen from it.
pub(crate) struct ListRows {
    pub rows: Vec<RowSpec>,
    /// The row highlighted when the list opens.
    pub default: usize,
    pub speed_search: bool,
    /// The row chosen this frame.
    pub chosen: Option<usize>,
}

impl PopupSource for ListRows {
    fn level(&mut self, depth: usize, _path: &[usize]) -> Option<LevelSpec> {
        (depth == 0).then(|| LevelSpec {
            rows: self.rows.clone(),
            speed_search: self.speed_search,
            default_index: Some(self.default),
            ..LevelSpec::default()
        })
    }

    fn choose(&mut self, _depth: usize, _path: &[usize], row: usize, final_choice: bool) -> Chosen {
        if final_choice {
            self.chosen = Some(row);
            Chosen::Final
        } else {
            Chosen::Stay
        }
    }
}
