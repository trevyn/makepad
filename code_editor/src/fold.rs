use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldState>,
        unfolding_lines: &HashMap<usize, FoldState>,
    ) -> Self {
        if folded.contains(&index) {
            Self {
                column_index: 0,
                scale: 1.0,
            }
        } else if let Some(state) = folding_lines.get(&index) {
            *state
        } else if let Some(state) = unfolding_lines.get(&index) {
            *state
        } else {
            FoldState::default()
        }
    }

    pub fn position_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
