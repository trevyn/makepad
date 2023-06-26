use {
    crate::{line::Lines, token::TokenInfo, Fold, Line},
    std::slice::Iter,
};

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Inlay {
    text: String,
    token_infos: Vec<TokenInfo>,
    wraps: Vec<usize>,
}

impl Inlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::token;

        let text = text.into();
        let token_infos = token::tokenize(&text);
        Self {
            text,
            token_infos,
            wraps: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        crate::line(
            &self.text,
            &self.token_infos,
            &[],
            &self.wraps,
            Fold::default(),
            (self.wraps.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.wraps = Vec::new();
        self.wraps = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, Inlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.lines.line_index() {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: &'a [(usize, Inlay)]) -> Blocks<'a> {
    let mut inlays = inlays.iter();
    while let Some((index, _)) = inlays.as_slice().first() {
        if *index >= lines.line_index() {
            break;
        }
        inlays.next();
    }
    Blocks { lines, inlays }
}
