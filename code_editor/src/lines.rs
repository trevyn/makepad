use {
    crate::{inlines::InlineInlay, tokens::TokenInfo, tokens::Tokens, Inlines},
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    height: Iter<'a, f64>,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
}

impl<'a> Lines<'a> {
    pub(super) fn new(
        height: Iter<'a, f64>,
        text: Iter<'a, String>,
        token_infos: Iter<'a, Vec<TokenInfo>>,
        inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
        breaks: Iter<'a, Vec<usize>>,
        folded: &'a HashSet<usize>,
        folding: &'a HashMap<usize, FoldingState>,
        unfolding: &'a HashMap<usize, FoldingState>,
    ) -> Self {
        Self {
            line_index: 0,
            height,
            text,
            token_infos,
            inlays,
            breaks,
            folded,
            folding,
            unfolding,
        }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line {
            height: *self.height.next()?,
            text: self.text.next()?,
            token_infos: self.token_infos.next()?,
            inlays: self.inlays.next()?,
            breaks: self.breaks.next()?,
            fold_state: FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
        };
        self.line_index += 1;
        Some(line)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    pub(super) height: f64,
    pub(super) text: &'a str,
    pub(super) token_infos: &'a [TokenInfo],
    pub(super) inlays: &'a [(usize, InlineInlay)],
    pub(super) breaks: &'a [usize],
    pub(super) fold_state: FoldState,
}

impl<'a> Line<'a> {
    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};
        
        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        let width = self.fold_state.column_x(self.column_count());
        width
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        use crate::tokens;

        tokens::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        use crate::inlines;

        inlines::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }
}

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

pub fn lines<'a>(
    height: Iter<'a, f64>,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        height,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
    }
}