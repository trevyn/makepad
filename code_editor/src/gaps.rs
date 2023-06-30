use {
    crate::{
        position::{Affinity, PositionWithAffinity},
        state::View,
        tokens::Token,
        visit::Visitor,
        Fold, Line, Position, Vector,
    },
    std::ops::Range,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gap {
    pub is_at_end_of_row: bool,
    pub logical_position: PositionWithAffinity,
    pub physical_position: Vector<f64>,
    pub scale: f64,
}

#[derive(Clone, Debug)]
struct GapsVisitor<F> {
    logical_position: PositionWithAffinity,
    fold: Fold,
    y: f64,
    column_index: usize,
    f: F,
}

impl<F> GapsVisitor<F>
where
    F: FnMut(Gap),
{
    fn emit_gap(&mut self, is_at_end_of_row: bool) {
        (self.f)(Gap {
            is_at_end_of_row,
            logical_position: self.logical_position,
            physical_position: Vector {
                x: self.fold.x(self.column_index),
                y: self.y,
            },
            scale: self.fold.scale(),
        })
    }
}

impl<F> Visitor for GapsVisitor<F>
where
    F: FnMut(Gap),
{
    fn visit_line(&mut self, is_inlay: bool, line: Line<'_>) {
        use crate::visit;

        if is_inlay {
            self.emit_gap(true);
            self.column_index = 0;
            self.y += line.height();
        } else {
            self.fold = line.fold();
            self.emit_gap(false);
            self.logical_position.affinity = Affinity::After;
            visit::walk_line(self, line);
            self.emit_gap(false);
            self.column_index += 1;
            self.emit_gap(true);
            self.logical_position.position.line_index += 1;
            self.logical_position.position.byte_index = 0;
            self.logical_position.affinity = Affinity::Before;
            self.column_index = 0;
            self.y += self.fold.scale();
            self.fold = Fold::default();
        }
    }

    fn visit_token(&mut self, is_inlay: bool, token: Token<'_>) {
        use crate::{str::StrExt, visit};

        if is_inlay {
            self.column_index += token.text.column_count();
        } else {
            visit::walk_token(self, token);
        }
    }

    fn visit_grapheme(&mut self, grapheme: &str) {
        use crate::str::StrExt;

        self.emit_gap(false);
        self.logical_position.position.byte_index += grapheme.len();
        self.logical_position.affinity = Affinity::Before;
        self.column_index += grapheme.column_count();
        self.emit_gap(false);
        self.logical_position.affinity = Affinity::After;
    }

    fn visit_wrap(&mut self) {
        self.column_index += 1;
        self.emit_gap(true);
        self.column_index = 0;
        self.y += self.fold.scale();
    }
}

pub fn gaps<F>(view: &View<'_>, line_index_range: Range<usize>, f: F)
where
    F: FnMut(Gap),
{
    let mut visitor = GapsVisitor {
        logical_position: PositionWithAffinity {
            position: Position {
                line_index: line_index_range.start,
                byte_index: 0,
            },
            affinity: Affinity::Before,
        },
        fold: Fold::default(),
        y: if line_index_range.start == 0 {
            0.0
        } else {
            view.line_summed_height(line_index_range.start - 1)
        },
        column_index: 0,
        f,
    };
    for block in view.blocks(line_index_range) {
        visitor.visit_block(block);
    }
}
