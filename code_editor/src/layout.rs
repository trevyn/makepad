use {
    crate::{
        block::Blocks, inline::Inlines, state::View, str_ext::Graphemes, token::TokenKind, Fold,
        Vector,
    },
    std::ops::RangeBounds,
};

#[derive(Clone, Debug)]
pub struct Layout<'a> {
    state: Option<State<'a>>,
    column_index: usize,
    y: f64,
}

impl<'a> Layout<'a> {
    fn create_event(&self, fold: Fold, kind: EventKind<'a>) -> Event<'a> {
        Event {
            position: Vector {
                x: fold.x(self.column_index),
                y: self.y,
            },
            kind,
        }
    }
}

impl<'a> Iterator for Layout<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use crate::{Block, Inline, StrExt};

        loop {
            match self.state.take().unwrap() {
                State::Blocks { mut blocks } => match blocks.next() {
                    Some(Block::Line { is_inlay, line, .. }) => {
                        let event = self.create_event(
                            Fold::default(),
                            EventKind::LineStart {
                                scale: line.fold().scale(),
                            },
                        );
                        self.state = Some(State::Inlines {
                            blocks,
                            is_inlay_line: is_inlay,
                            fold: line.fold(),
                            inlines: line.inlines(),
                        });
                        break Some(event);
                    }
                    None => break None,
                },
                State::Inlines {
                    blocks,
                    is_inlay_line,
                    fold,
                    mut inlines,
                } => match inlines.next() {
                    Some(inline) => match inline {
                        Inline::Token {
                            is_inlay: is_inlay_token,
                            token,
                        } => {
                            self.state = Some(State::Graphemes {
                                blocks,
                                is_inlay_line,
                                fold,
                                inlines,
                                is_inlay_token,
                                token_kind: token.kind,
                                graphemes: token.text.graphemes(),
                            });
                            continue;
                        }
                        Inline::Wrap => {
                            let event = self.create_event(fold, EventKind::Wrap);
                            self.column_index = 0;
                            self.y += fold.scale();
                            self.state = Some(State::Inlines {
                                blocks,
                                is_inlay_line,
                                fold,
                                inlines,
                            });
                            break Some(event);
                        }
                    },
                    None => {
                        let event = self.create_event(fold, EventKind::LineEnd);
                        self.column_index = 0;
                        self.y += fold.scale();
                        self.state = Some(State::Blocks { blocks });
                        break Some(event);
                    }
                },
                State::Graphemes {
                    blocks,
                    is_inlay_line,
                    fold,
                    inlines,
                    is_inlay_token,
                    token_kind,
                    mut graphemes,
                } => match graphemes.next() {
                    Some(grapheme) => {
                        let column_count = grapheme.column_count();
                        let event = self.create_event(
                            fold,
                            EventKind::Grapheme {
                                is_inlay: is_inlay_line | is_inlay_token,
                                width: fold.width(self.column_index, column_count),
                                token_kind,
                                grapheme,
                            },
                        );
                        self.column_index += column_count;
                        self.state = Some(State::Graphemes {
                            blocks,
                            is_inlay_line,
                            fold,
                            inlines,
                            is_inlay_token,
                            token_kind,
                            graphemes,
                        });
                        break Some(event);
                    }
                    None => {
                        self.state = Some(State::Inlines {
                            blocks,
                            is_inlay_line,
                            fold,
                            inlines,
                        });
                        continue;
                    }
                },
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Event<'a> {
    pub position: Vector<f64>,
    pub kind: EventKind<'a>,
}

#[derive(Clone, Copy, Debug)]
pub enum EventKind<'a> {
    LineStart {
        scale: f64,
    },
    LineEnd,
    Grapheme {
        is_inlay: bool,
        width: f64,
        token_kind: TokenKind,
        grapheme: &'a str,
    },
    Wrap,
}

#[derive(Clone, Debug)]
enum State<'a> {
    Blocks {
        blocks: Blocks<'a>,
    },
    Inlines {
        blocks: Blocks<'a>,
        is_inlay_line: bool,
        fold: Fold,
        inlines: Inlines<'a>,
    },
    Graphemes {
        blocks: Blocks<'a>,
        is_inlay_line: bool,
        fold: Fold,
        inlines: Inlines<'a>,
        is_inlay_token: bool,
        token_kind: TokenKind,
        graphemes: Graphemes<'a>,
    },
}

pub fn layout<'a>(view: &View<'a>, line_range: impl RangeBounds<usize>) -> Layout<'a> {
    let blocks = view.blocks(line_range);
    let y = if blocks.line_index() == 0 {
        0.0
    } else {
        view.line_summed_height(blocks.line_index() - 1)
    };
    Layout {
        state: Some(State::Blocks { blocks }),
        column_index: 0,
        y,
    }
}
