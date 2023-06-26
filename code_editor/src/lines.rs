use {
    crate::{fold::Folding, inlay::InlineInlay, tokenize::TokenInfo, Fold, Line},
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
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Folding>,
    unfolding: &'a HashMap<usize, Folding>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            Fold::new(
                &self.folded,
                &self.folding,
                &self.unfolding,
                self.line_index,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Folding>,
    unfolding: &'a HashMap<usize, Folding>,
    heights: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights,
    }
}
