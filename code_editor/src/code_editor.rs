use {
    crate::{
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Fold, Line, State,
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
        let mut viewport_origin = self.scroll_bars.get_scroll_pos();
        let viewport_size = cx.turtle().rect().size;
        let cell_size = self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);

        let view = state.view(session_id);
        // Find index of the first line that is in the viewport.
        let start_line_index = view.find_first_line_ending_after_y(viewport_origin.y / cell_size.y);
        // Find index of one past the last line that is in the viewport.
        let end_line_index = view
            .find_first_line_starting_after_y((viewport_origin.y + viewport_size.y) / cell_size.y);
        let start_line_y = view.line_y(start_line_index) * cell_size.y;

        // Word wrapping
        state
            .view_mut(session_id)
            .set_wrap_column_index(Some((viewport_size.x / cell_size.x) as usize));
        // After word wrapping, the position of the first line will have shifted. Adjust the origin
        // of the viewport so that its in the same position relative to that line.
        let old_start_line_y = start_line_y;
        let start_line_y = state.view(session_id).line_y(start_line_index) * cell_size.y;
        viewport_origin.y += start_line_y - old_start_line_y;

        let view = state.view(session_id);
        let mut max_width = 0.0;
        let mut height = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    max_width = max_width.max(line.width()) * cell_size.x;
                    height += line.height() * cell_size.y;
                }
            }
        }

        // Do the actual drawing
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let mut cx = CxDraw {
            cx,
            viewport_origin,
            viewport_size,
            fold: Fold::default(),
            position_y: start_line_y,
            column_index: 0,
            cell_size,
            is_inlay: false,
        };
        for block in state
            .view(session_id)
            .blocks(start_line_index, end_line_index)
        {
            self.draw_block(&mut cx, block);
        }
        cx.cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx.cx);

        // Update fold animations
        if state.view_mut(session_id).update_folds() {
            // After updating the fold animations, the position of the first line will have
            // shifted. Adjust the origin of the viewport so that its in the same position
            // relative to that line.
            let old_start_line_y = start_line_y;
            let start_line_y = state.view(session_id).line_y(start_line_index) * cell_size.y;
            viewport_origin.y += start_line_y - old_start_line_y;
            cx.cx.redraw_all();
        }

        // Save the position of the scroll bar after we adjusted it.
        self.scroll_bars.set_scroll_y(cx.cx, viewport_origin.y);
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

    fn draw_block(&mut self, cx: &mut CxDraw<'_, '_>, block: Block<'_>) {
        match block {
            Block::Line { is_inlay, line } => {
                cx.is_inlay = is_inlay;
                self.draw_line(cx, line);
                cx.is_inlay = false;
            }
        }
    }

    fn draw_line(&mut self, cx: &mut CxDraw<'_, '_>, line: Line<'_>) {
        cx.fold = line.fold();
        if cx.fold == Fold::Folded {
            return;
        }
        for inline in line.inlines() {
            self.draw_inline(cx, inline);
        }
        cx.column_index = 0;
        cx.position_y += cx.fold.scale() * cx.cell_size.y;
        cx.fold = Fold::default();
    }

    fn draw_inline(&mut self, cx: &mut CxDraw<'_, '_>, inline: Inline) {
        match inline {
            Inline::Token { is_inlay, token } => {
                let old_is_inlay = cx.is_inlay;
                cx.is_inlay |= is_inlay;
                self.draw_token(cx, token);
                cx.is_inlay = old_is_inlay;
            }
            Inline::Break => {
                cx.column_index = 0;
                cx.position_y += cx.fold.scale() * cx.cell_size.y;
            }
        }
    }

    fn draw_token(&mut self, cx: &mut CxDraw<'_, '_>, token: Token<'_>) {
        use crate::{state::TokenKind, StrExt};

        self.draw_text.font_scale = cx.fold.scale();
        self.draw_text.color = if cx.is_inlay {
            self.inlay_color
        } else {
            self.token_color
        };
        if token.kind != TokenKind::Whitespace {
            self.draw_text.draw_abs(cx.cx, cx.position(), token.text);
        }
        cx.column_index += token.text.column_count();
    }
}

struct CxDraw<'a, 'b> {
    cx: &'a mut Cx2d<'b>,
    viewport_origin: DVec2,
    viewport_size: DVec2,
    fold: Fold,
    column_index: usize,
    position_y: f64,
    cell_size: DVec2,
    is_inlay: bool,
}

impl<'a, 'b> CxDraw<'a, 'b> {
    fn position(&self) -> DVec2 {
        DVec2 {
            x: self.fold.width(self.column_index) * self.cell_size.x,
            y: self.position_y,
        } - self.viewport_origin
    }
}
