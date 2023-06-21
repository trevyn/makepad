use {
    crate::{
        inlines::Inline,
        lines::{FoldingState, Line},
        state::{Block, SessionId},
        tokens::Token,
        state::ViewMut,
        State,
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
        let DVec2 {
            x: column_width,
            y: row_height,
        } = self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);
        
        let mut view = state.view_mut(session_id);
        view.set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));

        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();
   
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: 0.0,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks() {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks() {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if view.update_fold_state() {
            cx.redraw_all();
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
                        view.fold(line_index, 8);
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
                        view.unfold(line_index, 8);
                    }
                }
                cx.redraw_all();
            }
            _ => {}
        }
    }
}

struct DrawContext<'a> {
    draw_text: &'a mut DrawText,
    row_height: f64,
    column_width: f64,
    inlay_color: Vec4,
    token_color: Vec4,
    scroll_position: DVec2,
    row_y: f64,
    column_index: usize,
    inlay: bool,
    fold_state: FoldingState,
}

impl<'a> DrawContext<'a> {
    fn position(&self) -> DVec2 {
        DVec2 {
            x: self.fold_state.column_x(self.column_index) * self.column_width,
            y: self.row_y,
        } - self.scroll_position
    }

    fn draw_block(&mut self, cx: &mut Cx2d<'_>, block: Block<'_>) {
        match block {
            Block::Line {
                is_inlay: inlay,
                line,
            } => {
                self.inlay = inlay;
                self.draw_line(cx, line);
                self.inlay = false;
            }
        }
    }

    fn draw_line(&mut self, cx: &mut Cx2d<'_>, line: Line<'_>) {
        use crate::state::FoldState;

        match line.fold_state() {
            FoldState::Folded => return,
            FoldState::Folding(fold) | FoldState::Unfolding(fold) => self.fold_state = fold,
            FoldState::Unfolded => {}
        }
        for inline in line.inlines() {
            self.draw_inline(cx, inline);
        }
        self.column_index = 0;
        self.row_y += self.fold_state.scale * self.row_height;
        self.fold_state = FoldingState::default();
    }

    fn draw_inline(&mut self, cx: &mut Cx2d<'_>, inline: Inline) {
        match inline {
            Inline::Token {
                is_inlay: inlay,
                token,
            } => {
                let old_inlay = self.inlay;
                self.inlay |= inlay;
                self.draw_token(cx, token);
                self.inlay = old_inlay;
            }
            Inline::Break => {
                self.column_index = 0;
                self.row_y += self.fold_state.scale * self.row_height;
            }
        }
    }

    fn draw_token(&mut self, cx: &mut Cx2d<'_>, token: Token<'_>) {
        use crate::{state::TokenKind, StrExt};

        self.draw_text.font_scale = self.fold_state.scale;
        self.draw_text.color = if self.inlay {
            self.inlay_color
        } else {
            self.token_color
        };
        if token.kind != TokenKind::Whitespace {
            self.draw_text.draw_abs(cx, self.position(), token.text);
        }
        self.column_index += token.text.column_count();
    }
}
