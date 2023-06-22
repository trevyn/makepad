use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
