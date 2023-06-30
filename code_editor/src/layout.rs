use {
    crate::{state::View, str::Graphemes, tokens::Token, Blocks, Inlines, Line},
    std::ops::Range,
};

#[derive(Clone, Debug)]
pub struct Layout<'a> {
    state: Option<State<'a>>,
}

#[derive(Clone, Debug)]
pub enum State<'a> {
    Blocks {
        blocks: Blocks<'a>,
    },
    Inlines {
        blocks: Blocks<'a>,
        is_inlay_line: bool,
        line: Line<'a>,
        inlines: Inlines<'a>,
    },
    Graphemes {
        blocks: Blocks<'a>,
        is_inlay_line: bool,
        line: Line<'a>,
        inlines: Inlines<'a>,
        is_inlay_token: bool,
        token: Token<'a>,
        graphemes: Graphemes<'a>,
    },
}

impl<'a> Iterator for Layout<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use crate::{blocks::Block, inlines::Inline, str::StrExt};

        loop {
            match self.state.take().unwrap() {
                State::Blocks { mut blocks } => match blocks.next() {
                    Some(Block::Line(is_inlay_line, line)) => {
                        let event = Event::LineStart {
                            is_inlay_line,
                            line,
                        };
                        self.state = Some(State::Inlines {
                            blocks,
                            is_inlay_line,
                            line,
                            inlines: line.inlines(),
                        });
                        break Some(event);
                    }
                    None => break None,
                },
                State::Inlines {
                    blocks,
                    is_inlay_line,
                    line,
                    mut inlines,
                } => match inlines.next() {
                    Some(Inline::Token(is_inlay_token, token)) => {
                        let event = Event::TokenStart {
                            is_inlay_line,
                            line,
                            is_inlay_token,
                            token,
                        };
                        self.state = Some(State::Graphemes {
                            blocks,
                            is_inlay_line,
                            line,
                            inlines,
                            is_inlay_token,
                            token,
                            graphemes: token.text.graphemes(),
                        });
                        break Some(event);
                    }
                    Some(Inline::Wrap) => {
                        let event = Event::Wrap {
                            is_inlay_line,
                            line,
                        };
                        self.state = Some(State::Inlines {
                            blocks,
                            is_inlay_line,
                            line,
                            inlines,
                        });
                        break Some(event);
                    }
                    None => {
                        let event = Event::LineEnd {
                            is_inlay_line,
                            line,
                        };
                        self.state = Some(State::Blocks { blocks });
                        break Some(event);
                    }
                },
                State::Graphemes {
                    blocks,
                    is_inlay_line,
                    line,
                    inlines,
                    is_inlay_token,
                    token,
                    mut graphemes,
                } => match graphemes.next() {
                    Some(grapheme) => {
                        let event = Event::Grapheme {
                            is_inlay_line,
                            line,
                            is_inlay_token,
                            token,
                            grapheme,
                        };
                        self.state = Some(State::Graphemes {
                            blocks,
                            is_inlay_line,
                            line,
                            inlines,
                            is_inlay_token,
                            token,
                            graphemes,
                        });
                        break Some(event);
                    }
                    None => {
                        let event = Event::TokenEnd {
                            is_inlay_line,
                            line,
                            is_inlay_token,
                            token,
                        };
                        self.state = Some(State::Inlines {
                            blocks,
                            is_inlay_line,
                            line,
                            inlines,
                        });
                        break Some(event);
                    }
                },
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Event<'a> {
    LineStart {
        is_inlay_line: bool,
        line: Line<'a>,
    },
    LineEnd {
        is_inlay_line: bool,
        line: Line<'a>,
    },
    TokenStart {
        is_inlay_line: bool,
        is_inlay_token: bool,
        line: Line<'a>,
        token: Token<'a>,
    },
    TokenEnd {
        is_inlay_line: bool,
        is_inlay_token: bool,
        line: Line<'a>,
        token: Token<'a>,
    },
    Grapheme {
        is_inlay_line: bool,
        is_inlay_token: bool,
        line: Line<'a>,
        token: Token<'a>,
        grapheme: &'a str,
    },
    Wrap {
        is_inlay_line: bool,
        line: Line<'a>,
    },
}

pub fn layout<'a>(view: &View<'a>, line_index_range: Range<usize>) -> Layout<'a> {
    Layout {
        state: Some(State::Blocks {
            blocks: view.blocks(line_index_range),
        }),
    }
}
