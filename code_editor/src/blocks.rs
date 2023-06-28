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
    pub fn line_index(&self) -> usize {
        self.lines.line_index()
    }
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.lines.line_index() {
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
    while let Some((index, _)) = inlays.as_slice().first() {
        if *index >= lines.line_index() {
            break;
        }
        inlays.next();
    }
    Blocks { lines, inlays }
}
