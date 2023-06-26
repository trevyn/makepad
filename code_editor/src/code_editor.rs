use {
    crate::{
        block::Blocks, inline::Inlines, state::SessionId, token::Token, Block, Fold, Inline, State,
    },
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::theme::*;

    CodeEditor = {{CodeEditor}} {
        walk: {
            width: Fill,
            height: Fill,
            margin: 0,
        },
        draw_text: {
            draw_depth: 0.0,
            text_style: <FONT_CODE> {},
        },
        inlay_color: #C00000
        token_color: #C0C0C0
    }
}

#[derive(Live, LiveHook)]
pub struct CodeEditor {
    #[live]
    walk: Walk,
    #[live]
    scroll_bars: ScrollBars,
    #[live]
    draw_text: DrawText,
    #[live]
    inlay_color: Vec4,
    #[live]
    token_color: Vec4,
}

impl CodeEditor {
    pub fn draw(&mut self, cx: &mut Cx2d<'_>, state: &mut State, session_id: SessionId) {
        use crate::token::TokenKind;

        self.scroll_bars.begin(cx, self.walk, Layout::default());

        let viewport_origin = self.scroll_bars.get_scroll_pos();
        let viewport_size = cx.turtle().rect().size;
        let cell_size = self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);

        state
            .view_mut(session_id)
            .set_max_column_count(Some((viewport_size.x / cell_size.x) as usize));

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(viewport_origin.y / cell_size.y);
        let end_line_index = view
            .find_first_line_starting_after_y((viewport_origin.y + viewport_size.y) / cell_size.y);
        for event in (Draw {
            viewport_origin,
            cell_size,
            y: if start_line_index == 0 {
                0.0
            } else {
                view.line_summed_height(start_line_index - 1) * cell_size.y
            },
            column_index: 0,
            state: Some(DrawState::Blocks {
                blocks: state
                    .view(session_id)
                    .blocks(start_line_index..end_line_index),
            }),
        }) {
            match event.kind {
                DrawEventKind::Token { is_inlay, token } => {
                    if token.kind == TokenKind::Whitespace {
                        continue;
                    }
                    self.draw_text.font_scale = event.scale;
                    self.draw_text.color = if is_inlay {
                        self.inlay_color
                    } else {
                        self.token_color
                    };
                    self.draw_text.draw_abs(cx, event.position, token.text);
                }
                _ => {}
            }
        }

        let view = state.view(session_id);
        let mut max_width = 0.0;
        let mut height = 0.0;
        for block in view.blocks(..) {
            match block {
                Block::Line { line, .. } => {
                    max_width = max_width.max(line.width()) * cell_size.x;
                    height += line.height() * cell_size.y;
                }
            }
        }
        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_folds() {
            cx.cx.redraw_all();
        }
    }

    pub fn handle_event(
        &mut self,
        cx: &mut Cx,
        state: &mut State,
        session_id: SessionId,
        event: &Event,
    ) {
        self.scroll_bars.handle_event_with(cx, event, &mut |cx, _| {
            cx.redraw_all();
        });
        match event {
            Event::KeyDown(KeyEvent {
                key_code: KeyCode::Alt,
                ..
            }) => {
                let mut view = state.view_mut(session_id);
                for line_index in 0..view.line_count() {
                    if view
                        .line(line_index)
                        .text()
                        .chars()
                        .take_while(|char| char.is_whitespace())
                        .count()
                        >= 8
                    {
                        view.fold_line(line_index, 8);
                    }
                }
                cx.redraw_all();
            }
            Event::KeyUp(KeyEvent {
                key_code: KeyCode::Alt,
                ..
            }) => {
                let mut view = state.view_mut(session_id);
                for line_index in 0..view.line_count() {
                    if view
                        .line(line_index)
                        .text()
                        .chars()
                        .take_while(|char| char.is_whitespace())
                        .count()
                        >= 8
                    {
                        view.unfold_line(line_index, 8);
                    }
                }
                cx.redraw_all();
            }
            _ => {}
        }
    }
}

struct Draw<'a> {
    viewport_origin: DVec2,
    cell_size: DVec2,
    y: f64,
    column_index: usize,
    state: Option<DrawState<'a>>,
}

impl<'a> Draw<'a> {
    fn create_event(&self, fold: Fold, kind: DrawEventKind<'a>) -> DrawEvent<'a> {
        DrawEvent {
            position: DVec2 {
                x: fold.width(self.column_index) * self.cell_size.x,
                y: self.y,
            } - self.viewport_origin,
            scale: fold.scale(),
            kind,
        }
    }
}

impl<'a> Iterator for Draw<'a> {
    type Item = DrawEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use crate::StrExt;

        loop {
            break match self.state.take().unwrap() {
                DrawState::Blocks { mut blocks } => match blocks.next() {
                    Some(Block::Line { is_inlay, line, .. }) => {
                        self.state = Some(DrawState::Inlines {
                            blocks,
                            is_inlay,
                            fold: line.fold(),
                            inlines: line.inlines(),
                        });
                        continue;
                    }
                    None => None,
                },
                DrawState::Inlines {
                    blocks,
                    is_inlay: is_inlay_line,
                    fold,
                    mut inlines,
                } => Some(match inlines.next() {
                    Some(inline) => {
                        let event = match inline {
                            Inline::Token {
                                is_inlay: is_inlay_token,
                                token,
                            } => {
                                let event = self.create_event(
                                    fold,
                                    DrawEventKind::Token {
                                        is_inlay: is_inlay_line || is_inlay_token,
                                        token,
                                    },
                                );
                                self.column_index += token.text.column_count();
                                event
                            }
                            Inline::Wrap => {
                                let event = self.create_event(fold, DrawEventKind::NewRow);
                                self.column_index = 0;
                                self.y += fold.scale() * self.cell_size.y;
                                event
                            }
                        };
                        self.state = Some(DrawState::Inlines {
                            blocks,
                            is_inlay: is_inlay_line,
                            fold,
                            inlines,
                        });
                        event
                    }
                    None => {
                        let event = self.create_event(fold, DrawEventKind::NewRow);
                        self.column_index = 0;
                        self.y += fold.scale() * self.cell_size.y;
                        self.state = Some(DrawState::Blocks { blocks });
                        event
                    }
                }),
            };
        }
    }
}

enum DrawState<'a> {
    Blocks {
        blocks: Blocks<'a>,
    },
    Inlines {
        blocks: Blocks<'a>,
        is_inlay: bool,
        fold: Fold,
        inlines: Inlines<'a>,
    },
}

#[derive(Clone, Copy, Debug)]
struct DrawEvent<'a> {
    position: DVec2,
    scale: f64,
    kind: DrawEventKind<'a>,
}

#[derive(Clone, Copy, Debug)]
enum DrawEventKind<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    NewRow,
}
