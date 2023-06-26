use crate::{
    inline::Inlay,
    token::{TokenInfo, Tokens},
    Fold, Inlines,
};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, Inlay)],
    breaks: &'a [usize],
    fold: Fold,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, Inlay)],
        breaks: &'a [usize],
        fold: Fold,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold,
            height,
        }
    }

    pub fn fold(&self) -> Fold {
        self.fold
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inline::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
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

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold.width(self.column_count())
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
        
        inline::inlines(self.text, self.token_infos, self.inlays, self.breaks)
    }
}
