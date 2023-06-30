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
        line_idx: usize,
    ) -> Self {
        if folded.contains(&line_idx) {
            return Self::Folded;
        }
        if let Some(&folding) = folding.get(&line_idx) {
            return Self::Folding(folding);
        }
        if let Some(&folding) = unfolding.get(&line_idx) {
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

    pub fn x(self, col_idx: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(folding) | Self::Unfolding(folding) => {
                let col_count_before = col_idx.min(folding.col_idx);
                let col_count_after = col_idx - col_count_before;
                col_count_before as f64 + folding.scale * col_count_after as f64
            }
            Self::Unfolded => col_idx as f64,
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
    pub col_idx: usize,
    pub scale: f64,
}
