use crate::{inlay::InlineInlay, tokenize::TokenInfo, Fold, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold: Fold,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
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
        self.fold.width(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
