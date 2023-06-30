use {
    crate::{fold::Folding, inlays::InlineInlay, tokenize::TokenInfo, Fold, Line},
    std::{
        collections::{HashMap, HashSet},
        ops::Range,
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    wraps: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Folding>,
    unfolding: &'a HashMap<usize, Folding>,
    heights: Iter<'a, f64>,
    line_idx: usize,
}

impl<'a> Lines<'a> {
    pub fn line_idx(&self) -> usize {
        self.line_idx
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = crate::line(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.wraps.next()?,
            Fold::new(&self.folded, &self.folding, &self.unfolding, self.line_idx),
            *self.heights.next()?,
        );
        self.line_idx += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inlays: &'a [Vec<(usize, InlineInlay)>],
    wraps: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Folding>,
    unfolding: &'a HashMap<usize, Folding>,
    heights: &'a [f64],
    line_range: Range<usize>,
) -> Lines<'a> {
    Lines {
        text: text[line_range.clone()].iter(),
        token_infos: token_infos[line_range.clone()].iter(),
        inlays: inlays[line_range.clone()].iter(),
        wraps: wraps[line_range.clone()].iter(),
        folded,
        folding,
        unfolding,
        heights: heights[line_range.clone()].iter(),
        line_idx: line_range.start,
    }
}
