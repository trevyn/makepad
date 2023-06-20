use {
    super::{Inlay, Inlines, TokenInfo, Tokens},
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, Inlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
}

impl<'a> Lines<'a> {
    pub(super) fn new(
        text: Iter<'a, String>,
        token_infos: Iter<'a, Vec<TokenInfo>>,
        inlays: Iter<'a, Vec<(usize, Inlay)>>,
        breaks: Iter<'a, Vec<usize>>,
        folded: &'a HashSet<usize>,
        folding: &'a HashMap<usize, FoldingState>,
        unfolding: &'a HashMap<usize, FoldingState>,
    ) -> Self {
        Self {
            line_index: 0,
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
    pub(super) text: &'a str,
    pub(super) token_infos: &'a [TokenInfo],
    pub(super) inlays: &'a [(usize, Inlay)],
    pub(super) breaks: &'a [usize],
    pub(super) fold_state: FoldState,
}

impl<'a> Line<'a> {
    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        Tokens::new(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        Inlines::new(self.tokens(), self.inlays.iter(), self.breaks.iter())
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
