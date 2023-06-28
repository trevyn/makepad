use {
    crate::{blocks::Block, position::PositionWithAffinity, state::SessionId, Range, State},
    makepad_widgets::*,
    std::iter::Peekable,
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
        use crate::{layout::EventKind, position::Affinity, tokenize::TokenKind, Position};

        self.scroll_bars.begin(cx, self.walk, Layout::default());

        let viewport_position = self.scroll_bars.get_scroll_pos();
        let viewport_size = cx.turtle().rect().size;
        let cell_size = self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);

        state
            .view_mut(session_id)
            .set_max_column_count(Some((viewport_size.x / cell_size.x) as usize));

        let view = state.view(session_id);
        let line_start = view.find_first_line_ending_after_y(viewport_position.y / cell_size.y);
        let line_end = view.find_first_line_starting_after_y(
            (viewport_position.y + viewport_size.y) / cell_size.y,
        );
        let start_line_y = if line_start == 0 {
            0.0
        } else {
            view.line_summed_height(line_start - 1)
        };

        for event in view.layout(line_start..line_end) {
            match event.kind {
                EventKind::LineStart { scale } => {
                    self.draw_text.font_scale = scale;
                }
                EventKind::Grapheme {
                    is_inlay,
                    token_kind,
                    grapheme,
                    ..
                } => {
                    if token_kind == TokenKind::Whitespace {
                        continue;
                    }
                    self.draw_text.color = if is_inlay {
                        self.inlay_color
                    } else {
                        self.token_color
                    };
                    self.draw_text.draw_abs(
                        cx,
                        DVec2 {
                            x: event.position.x,
                            y: event.position.y,
                        } * cell_size
                            - viewport_position,
                        grapheme,
                    );
                }
                _ => {}
            }
        }

        DrawOverlayContext {
            viewport_position,
            cell_size,
            active_range: None,
            regions: view
                .selection()
                .iter()
                .map(|region| region.range())
                .peekable(),
            logical_position: PositionWithAffinity {
                position: Position {
                    line_index: line_start,
                    byte_index: 0,
                },
                affinity: Affinity::Before,
            },
            physical_position: DVec2::new(),
            scale: 0.0,
        }
        .draw_overlay(view.layout(line_start..line_end));

        let view = state.view(session_id);
        let mut max_width = 0.0;
        let mut height = 0.0;
        for block in view.blocks(0..view.line_count()) {
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

#[derive(Clone, Debug)]
struct DrawOverlayContext<I>
where
    I: Iterator<Item = Range<PositionWithAffinity>>,
{
    viewport_position: DVec2,
    cell_size: DVec2,
    active_range: Option<ActiveRange>,
    regions: Peekable<I>,
    logical_position: PositionWithAffinity,
    physical_position: DVec2,
    scale: f64,
}

impl<I> DrawOverlayContext<I>
where
    I: Iterator<Item = Range<PositionWithAffinity>>,
{
    fn draw_overlay(&mut self, layout: crate::Layout<'_>) {
        use crate::{layout::EventKind, position::Affinity};

        for event in layout {
            self.physical_position = DVec2 {
                x: event.position.x,
                y: event.position.y,
            } * self.cell_size
                - self.viewport_position;
            match event.kind {
                EventKind::LineStart { scale } => {
                    self.scale = scale;
                    self.handle_event();
                    self.logical_position.affinity = Affinity::After;
                }
                EventKind::LineEnd => {
                    self.handle_event();
                    if self.active_range.is_some() {
                        self.draw_rect();
                    }
                    self.logical_position.position.line_index += 1;
                    self.logical_position.position.byte_index = 0;
                    self.logical_position.affinity = Affinity::Before;
                }
                EventKind::Grapheme {
                    is_inlay: false,
                    width,
                    grapheme,
                    ..
                } => {
                    self.handle_event();
                    self.logical_position.position.byte_index += grapheme.len();
                    self.logical_position.affinity = Affinity::Before;
                    self.physical_position.x += width * self.cell_size.x;
                    self.handle_event();
                    self.logical_position.affinity = Affinity::After;
                }
                EventKind::Wrap => {
                    if self.active_range.is_some() {
                        self.draw_rect();
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_event(&mut self) {
        if self
            .regions
            .peek()
            .map_or(false, |region| region.start == self.logical_position)
        {
            self.begin();
            self.active_range = Some(ActiveRange {
                range: self.regions.next().unwrap(),
                start_x: self.physical_position.x,
            });
        }
        if self
            .active_range
            .as_ref()
            .map_or(false, |range| range.range.end == self.logical_position)
        {
            self.draw_rect();
            self.end();
            let active_region = self.active_range.take().unwrap();
        }
    }

    fn begin(&mut self) {
        println!("BEGIN");
        // TODO
    }

    fn end(&mut self) {
        println!("END");
        // TODO
    }

    fn draw_rect(&mut self) {
        use std::mem;

        let start_x = mem::replace(&mut self.active_range.as_mut().unwrap().start_x, 0.0);
        println!(
            "DRAW RECT {:?} {:?} {:?}",
            start_x, self.physical_position, self.scale
        );
        // TODO
    }
}

#[derive(Clone, Debug)]
struct ActiveRange {
    range: Range<PositionWithAffinity>,
    start_x: f64,
}
