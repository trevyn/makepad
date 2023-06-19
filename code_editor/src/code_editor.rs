use {
    crate::{
        state::{Block, Fold, Inline, Line, SessionId, Token},
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
        state.set_max_column_index(
            session_id,
            Some((cx.turtle().rect().size.x / column_width as f64) as usize),
        );
        DrawContext {
            draw_text: &mut self.draw_text,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            row_y: 0.0,
            column_index: 0,
            inlay: false,
            fold: Fold::default(),
            row_height,
            column_width,
        }
        .draw(cx, state, session_id);
    }

    pub fn handle_event(
        &mut self,
        cx: &mut Cx,
        state: &mut State,
        session_id: SessionId,
        event: &Event,
    ) {
        match event {
            Event::KeyDown(KeyEvent {
                key_code: KeyCode::Alt,
                ..
            }) => {
                for line_index in 0..state.line_count(session_id) {
                    if state
                        .line(session_id, line_index)
                        .text()
                        .chars()
                        .take_while(|char| char.is_whitespace())
                        .count()
                        >= 8
                    {
                        state.fold_line(session_id, line_index, 8);
                    }
                }
                cx.redraw_all();
            }
            Event::KeyUp(KeyEvent {
                key_code: KeyCode::Alt,
                ..
            }) => {
                for line_index in 0..state.line_count(session_id) {
                    if state
                        .line(session_id, line_index)
                        .text()
                        .chars()
                        .take_while(|char| char.is_whitespace())
                        .count()
                        >= 8
                    {
                        state.unfold_line(session_id, line_index, 8);
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
    inlay_color: Vec4,
    token_color: Vec4,
    row_y: f64,
    column_index: usize,
    inlay: bool,
    fold: Fold,
    row_height: f64,
    column_width: f64,
}

impl<'a> DrawContext<'a> {
    fn position(&self) -> DVec2 {
        let column_count_before = self.column_index.min(self.fold.column_index);
        let column_count_after = self.column_index - column_count_before;
        let column_width_before = self.column_width;
        let column_width_after = self.fold.scale * self.column_width;
        DVec2 {
            x: column_count_before as f64 * column_width_before
                + column_count_after as f64 * column_width_after,
            y: self.row_y,
        }
    }

    fn draw(&mut self, cx: &mut Cx2d<'_>, state: &mut State, session_id: SessionId) {
        for block in state.blocks(session_id) {
            self.draw_block(cx, block);
        }
        if state.update_fold_state(session_id) {
            cx.redraw_all();
        }
    }

    fn draw_block(&mut self, cx: &mut Cx2d<'_>, block: Block<'_>) {
        match block {
            Block::Line { inlay, line } => {
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
            FoldState::Folding(fold) | FoldState::Unfolding(fold) => self.fold = fold,
            FoldState::Unfolded => {}
        }
        for inline in line.inlines() {
            self.draw_inline(cx, inline);
        }
        self.column_index = 0;
        self.row_y += self.fold.scale * self.row_height;
        self.fold = Fold::default();
    }

    fn draw_inline(&mut self, cx: &mut Cx2d<'_>, inline: Inline) {
        match inline {
            Inline::Token { inlay, token } => {
                let old_inlay = self.inlay;
                self.inlay |= inlay;
                self.draw_token(cx, token);
                self.inlay = old_inlay;
            }
            Inline::Break => {
                self.column_index = 0;
                self.row_y += self.fold.scale * self.row_height;
            }
        }
    }

    fn draw_token(&mut self, cx: &mut Cx2d<'_>, token: Token<'_>) {
        use crate::{state::TokenKind, StrExt};

        self.draw_text.font_scale = self.fold.scale;
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
