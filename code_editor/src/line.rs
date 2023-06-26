use {
    crate::{
        fold::Folding,
        inline,
        inline::{Inlines, Inlay},
        token::{TokenInfo, Tokens},
        Fold,
    },
    std::{
        collections::{HashMap, HashSet},
        ops::RangeBounds,
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, inline::Inlay)>>,
    wraps: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Folding>,
    unfolding: &'a HashMap<usize, Folding>,
    heights: Iter<'a, f64>,
    index: usize,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.wraps.next()?,
            Fold::new(
                &self.folded,
                &self.folding,
                &self.unfolding,
                self.index,
            ),
            *self.heights.next()?,
        );
        self.index += 1;
        Some(line)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, Inlay)],
    wraps: &'a [usize],
    fold: Fold,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, Inlay)],
        wraps: &'a [usize],
        fold: Fold,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            wraps,
            fold,
            height,
        }
    }

    pub fn fold(&self) -> Fold {
        self.fold
    }

    pub fn column_count(&self) -> usize {
        use crate::{Inline, StrExt};

        let mut max_column_count = 0;
        let mut column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Wrap => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn row_count(&self) -> usize {
        self.wraps.len() + 1
    }

    pub fn width(&self) -> f64 {
        self.fold.width(self.column_count())
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        use crate::token;

        token::tokens(self.text, self.token_infos)
    }

    pub fn inlines(&self) -> Inlines<'a> {
        use crate::inline;
        
        inline::inlines(self.tokens(), self.inlays, self.wraps)
    }
}

pub fn lines<'a>(
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inlays: &'a [Vec<(usize, inline::Inlay)>],
    wraps: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Folding>,
    unfolding: &'a HashMap<usize, Folding>,
    heights: &'a [f64],
    range: impl RangeBounds<usize>,
) -> Lines<'a> {
    use std::ops::Bound;

    let start = match range.start_bound() {
        Bound::Included(&start) => start,
        Bound::Excluded(&start) => start + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&end) => end + 1,
        Bound::Excluded(&end) => end,
        Bound::Unbounded => text.len(),
    };
    Lines {
        text: text[start..end].iter(),
        token_infos: token_infos[start..end].iter(),
        inlays: inlays[start..end].iter(),
        wraps: wraps[start..end].iter(),
        folded,
        folding,
        unfolding,
        heights: heights[start..end].iter(),
        index: start,
    }
}