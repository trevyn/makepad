use {
    crate::{
        lines::{FoldState, Line},
        tokens::TokenInfo,
        Lines, Tokens,
    },
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line {
            text: &self.text,
            token_infos: &self.token_infos,
            inlays: &[],
            breaks: &self.breaks,
            fold_state: FoldState::Unfolded,
        }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
