use {
    crate::{inlays::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Blocks<'a> {
    pub fn line_idx(&self) -> usize {
        self.lines.line_idx()
    }
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((idx, _)) = self.inlays.as_slice().first() {
            if *idx == self.lines.line_idx() {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line(true, inlay.as_line()));
            }
        }
        let line = self.lines.next()?;
        Some(Block::Line(false, line))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line(bool, Line<'a>),
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: &'a [(usize, BlockInlay)]) -> Blocks<'a> {
    let mut inlays = inlays.iter();
    while let Some((idx, _)) = inlays.as_slice().first() {
        if *idx >= lines.line_idx() {
            break;
        }
        inlays.next();
    }
    Blocks { lines, inlays }
}
