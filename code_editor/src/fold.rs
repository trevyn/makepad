use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fold {
    Folded,
    Folding(Folding),
    Unfolding(Folding),
    Unfolded,
}

impl Fold {
    pub fn new(
        folded: &HashSet<usize>,
        folding: &HashMap<usize, Folding>,
        unfolding: &HashMap<usize, Folding>,
        line_index: usize,
    ) -> Self {
        if folded.contains(&line_index) {
            return Self::Folded;
        }
        if let Some(&folding) = folding.get(&line_index) {
            return Self::Folding(folding);
        }
        if let Some(&folding) = unfolding.get(&line_index) {
            return Self::Unfolding(folding);
        }
        Fold::default()
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(folding) | Self::Unfolding(folding) => folding.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(folding) | Self::Unfolding(folding) => {
                let column_count_before = column_index.min(folding.column_index);
                let column_count_after = column_index - column_count_before;
                column_count_before as f64 + folding.scale * column_count_after as f64
            }
            Self::Unfolded => column_index as f64,
        }
    }
}

impl Default for Fold {
    fn default() -> Self {
        Self::Unfolded
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Folding {
    pub column_index: usize,
    pub scale: f64,
}
