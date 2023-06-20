use {
    super::{FoldState, Inlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, Inlay)>,
}

impl<'a> Blocks<'a> {
    pub(super) fn new(lines: Lines<'a>, inlays: Iter<'a, (usize, Inlay)>) -> Self {
        Self {
            line_index: 0,
            lines,
            inlays,
        }
    }
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    inlay: true,
                    line: Line {
                        text: &inlay.text,
                        token_infos: &inlay.token_infos,
                        inlays: &[],
                        breaks: &[],
                        fold_state: FoldState::Unfolded,
                    },
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line { inlay: false, line })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { inlay: bool, line: Line<'a> },
}
