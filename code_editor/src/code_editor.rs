use {
    crate::{
        state::{Block, Inline, Line, SessionId, Token},
        State,
    },
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::theme::*;

    CodeEditor = {{CodeEditor}} {
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
    draw_text: DrawText,
    #[live]
    inlay_color: Vec4,
    #[live]
    token_color: Vec4,
}

impl CodeEditor {
    pub fn draw<'a>(&mut self, cx: &mut Cx2d<'_>, state: &'a State, session_id: SessionId) {
        let DVec2 {
            x: column_width,
            y: row_height,
        } = self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);
        DrawContext {
            draw_text: &mut self.draw_text,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            row_y: 0.0,
            column_index: 0,
            row_height,
            column_width,
        }
        .draw(cx, state, session_id);
    }
}

struct DrawContext<'a> {
    draw_text: &'a mut DrawText,
    inlay_color: Vec4,
    token_color: Vec4,
    row_y: f64,
    column_index: usize,
    row_height: f64,
    column_width: f64,
}

impl<'a> DrawContext<'a> {
    fn position(&self) -> DVec2 {
        DVec2 {
            x: self.column_index as f64 * self.column_width,
            y: self.row_y,
        }
    }

    fn draw(&mut self, cx: &mut Cx2d<'_>, state: &State, session_id: SessionId) {
        for block in state.blocks(session_id) {
            self.draw_block(cx, block);
        }
    }

    fn draw_block(&mut self, cx: &mut Cx2d<'_>, block: Block<'_>) {
        match block {
            Block::Inlay(inlay) => self.draw_block_inlay(cx, inlay),
            Block::Line(line) => self.draw_line(cx, line),
        }
    }

    fn draw_block_inlay(&mut self, cx: &mut Cx2d<'_>, inlay: &str) {
        self.draw_text.color = self.inlay_color;
        self.draw_text.draw_abs(cx, self.position(), inlay);
        self.row_y += self.row_height;
    }

    fn draw_line(&mut self, cx: &mut Cx2d<'_>, line: Line<'_>) {
        for inline in line.inlines() {
            self.draw_inline(cx, inline);
        }
        self.column_index = 0;
        self.row_y += self.row_height;
    }

    fn draw_inline(&mut self, cx: &mut Cx2d<'_>, inline: Inline) {
        match inline {
            Inline::Inlay(inlay) => self.draw_inline_inlay(cx, inlay),
            Inline::Token(token) => self.draw_token(cx, token),
        }
    }

    fn draw_inline_inlay(&mut self, cx: &mut Cx2d<'_>, inlay: &str) {
        use crate::StrExt;

        self.draw_text.color = self.inlay_color;
        self.draw_text.draw_abs(cx, self.position(), inlay);
        self.column_index += inlay.column_count();
    }

    fn draw_token(&mut self, cx: &mut Cx2d<'_>, token: Token<'_>) {
        use crate::StrExt;

        self.draw_text.color = self.token_color;
        self.draw_text.draw_abs(cx, self.position(), token.text);
        self.column_index += token.text.column_count();
    }
}
