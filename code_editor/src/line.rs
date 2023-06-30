use crate::{inlays::InlineInlay, tokenize::TokenInfo, Fold, Inlines, Tokens};

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
    pub fn fold(&self) -> Fold {
        self.fold
    }

    pub fn col_count(&self) -> usize {
        use crate::{inlines::Inline, str::StrExt};

        let mut max_col_count = 0;
        let mut col_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token(_, token) => {
                    col_count += token.text.col_count();
                    max_col_count = max_col_count.max(col_count);
                }
                Inline::Wrap => col_count = 0,
            }
        }
        max_col_count
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn width(&self) -> f64 {
        self.fold.x(self.col_count())
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos)
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays, self.breaks)
    }
}

pub fn line<'a>(
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold: Fold,
    height: f64,
) -> Line<'a> {
    Line {
        text,
        token_infos,
        inlays,
        breaks,
        fold,
        height,
    }
}
