use {
    crate::{inlay::BlockInlay, Line, Lines},
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

pub fn blocks<'a>(line_index: usize, lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index,
        lines,
        inlays,
    }
}
