use {
    crate::{
        position::{Affinity, Position, PositionWithAffinity},
        state::SessionId,
        tokens::Token,
        visit::Visitor,
        Fold, Line, Range, State,
    },
    makepad_widgets::*,
    std::iter::Peekable,
};

live_design! {
    import makepad_draw::shader::std::*;
    import makepad_widgets::theme::*;

    DrawSelection = {{DrawSelection}} {
        uniform gloopiness: 8.0
        uniform border_radius: 2.0

        fn vertex(self) -> vec4 {
            let clipped: vec2 = clamp(
                self.geom_pos * vec2(self.rect_size.x + 16., self.rect_size.y) + self.rect_pos - vec2(8., 0.),
                self.draw_clip.xy,
                self.draw_clip.zw
            );
            self.pos = (clipped - self.rect_pos) / self.rect_size;
            return self.camera_projection * (self.camera_view * (
                self.view_transform * vec4(clipped.x, clipped.y, self.draw_depth + self.draw_zbias, 1.)
            ));
        }

        fn pixel(self) -> vec4 {
            let sdf = Sdf2d::viewport(self.rect_pos + self.pos * self.rect_size);
            sdf.box(
                self.rect_pos.x,
                self.rect_pos.y,
                self.rect_size.x,
                self.rect_size.y,
                self.border_radius
            );
            if self.prev_w > 0.0 {
                sdf.box(
                    self.prev_x,
                    self.rect_pos.y - self.rect_size.y,
                    self.prev_w,
                    self.rect_size.y,
                    self.border_radius
                );
                sdf.gloop(self.gloopiness);
            }
            if self.next_w > 0.0 {
                sdf.box(
                    self.next_x,
                    self.rect_pos.y + self.rect_size.y,
                    self.next_w,
                    self.rect_size.y,
                    self.border_radius
                );
                sdf.gloop(self.gloopiness);
            }
            return sdf.fill(#08f8);
        }
    }

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
        draw_selection: {
            draw_depth: 1.0,
        }
        draw_cursor: {
            draw_depth: 2.0,
            color: #FFF,
        }
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
    draw_selection: DrawSelection,
    #[live]
    draw_cursor: DrawColor,
    #[live]
    inlay_color: Vec4,
    #[live]
    token_color: Vec4,
}

impl CodeEditor {
    pub fn draw(&mut self, cx: &mut Cx2d<'_>, state: &mut State, session_id: SessionId) {
        use crate::{blocks::Block, selection::Region};

        let viewport_position = self.scroll_bars.get_scroll_pos();
        let viewport_size = cx.turtle().rect().size;
        let cell_size = self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);

        state
            .view_mut(session_id)
            .set_max_column_count(Some((viewport_size.x / cell_size.x) as usize));

        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let view = state.view(session_id);
        let start_line_index =
            view.find_first_line_ending_after_y(viewport_position.y / cell_size.y);
        let end_line_index = view.find_first_line_starting_after_y(
            (viewport_position.y + viewport_size.y) / cell_size.y,
        );
        let y = if start_line_index == 0 {
            0.0
        } else {
            view.line_summed_height(start_line_index - 1)
        };

        let mut visitor = DrawTextVisitor {
            cx,
            draw_text: &mut self.draw_text,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            viewport_position,
            cell_size,
            is_inlay_line: false,
            fold: Fold::default(),
            y,
            column_index: 0,
        };
        for block in view.blocks(start_line_index..end_line_index) {
            visitor.visit_block(block);
        }

        struct ActiveRegion {
            region: Region,
            start_x: f64,
        }

        let mut active_region = None;
        let mut regions = view.selection().iter().peekable();
        while regions.peek().map_or(false, |region| {
            region.end().position.line_index < start_line_index
        }) {
            regions.next();
        }
        if regions.peek().map_or(false, |region| {
            region.start().position.line_index < start_line_index
        }) {
            active_region = Some(ActiveRegion {
                region: regions.next().unwrap(),
                start_x: 0.0,
            });
        }
        view.gaps(start_line_index..end_line_index, |gap| {
            let physical_position = DVec2 {
                x: gap.physical_position.x,
                y: gap.physical_position.y,
            } * cell_size
                - viewport_position;
            if regions
                .peek()
                .map_or(false, |region| region.start() == gap.logical_position)
            {
                let region = regions.next().unwrap();
                if region.cursor.position == gap.logical_position {
                    self.draw_cursor.draw_abs(cx, Rect {
                        pos: physical_position,
                        size: DVec2 {
                            x: 2.0,
                            y: gap.scale * cell_size.y,
                        },
                    })
                }
                active_region = Some(ActiveRegion {
                    region,
                    start_x: physical_position.x,
                });
                self.draw_selection.begin(cx);
            }
            if let Some(region) = active_region.as_mut() {
                if region.region.end() == gap.logical_position || gap.is_at_end_of_row {
                    self.draw_selection.draw_rect(cx, Rect {
                        pos: DVec2 {
                            x: region.start_x,
                            y: physical_position.y,
                        },
                        size: DVec2 {
                            x: physical_position.x - region.start_x,
                            y: gap.scale * cell_size.y,
                        }
                    });
                    region.start_x = 0.0;
                }
            }
            if active_region
                .as_ref()
                .map_or(false, |region| region.region.end() == gap.logical_position)
            {
                self.draw_selection.end(cx);
                let region = active_region.take().unwrap().region;
                if region.anchor.position != region.cursor.position.position && region.cursor.position == gap.logical_position {
                    self.draw_cursor.draw_abs(cx, Rect {
                        pos: physical_position,
                        size: DVec2 {
                            x: 2.0,
                            y: gap.scale * cell_size.y,
                        },
                    })
                }
            }
        });
        if active_region.is_some() {
            self.draw_selection.end(cx);
        }

        let mut max_width = 0.0;
        let mut height = 0.0;
        for block in view.blocks(0..view.line_count()) {
            match block {
                Block::Line(_, line) => {
                    max_width = max_width.max(line.width());
                    height += line.height();
                }
            }
        }
        cx.turtle_mut()
            .set_used(max_width * cell_size.x, height * cell_size.y);
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
        let mut view = state.view_mut(session_id);
        match *event {
            Event::KeyDown(KeyEvent {
                key_code: KeyCode::Alt,
                ..
            }) => {
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

struct DrawTextVisitor<'a, 'b> {
    cx: &'a mut Cx2d<'b>,
    draw_text: &'a mut DrawText,
    inlay_color: Vec4,
    token_color: Vec4,
    viewport_position: DVec2,
    cell_size: DVec2,
    is_inlay_line: bool,
    fold: Fold,
    y: f64,
    column_index: usize,
}

impl<'a, 'b> DrawTextVisitor<'a, 'b> {
    fn position(&self) -> DVec2 {
        DVec2 {
            x: self.fold.x(self.column_index),
            y: self.y,
        } * self.cell_size
            - self.viewport_position
    }
}

impl<'a, 'b> Visitor for DrawTextVisitor<'a, 'b> {
    fn visit_line(&mut self, is_inlay: bool, line: Line<'_>) {
        use crate::visit;

        self.is_inlay_line = is_inlay;
        self.fold = line.fold();
        visit::walk_line(self, line);
        self.column_index = 0;
        self.y += self.fold.scale();
        self.fold = Fold::default();
        self.is_inlay_line = false;
    }

    fn visit_token(&mut self, is_inlay: bool, token: Token<'_>) {
        use crate::{str::StrExt, tokenize::TokenKind};

        if token.kind != TokenKind::Whitespace {
            self.draw_text.font_scale = self.fold.scale();
            self.draw_text.color = if self.is_inlay_line || is_inlay {
                self.inlay_color
            } else {
                self.token_color
            };
            self.draw_text
                .draw_abs(self.cx, self.position(), token.text);
        }
        self.column_index += token.text.column_count();
    }

    fn visit_wrap(&mut self) {
        self.column_index = 0;
        self.y += self.fold.scale();
    }
}

#[derive(Live, LiveHook)]
#[repr(C)]
struct DrawSelection {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    prev_x: f32,
    #[live]
    prev_w: f32,
    #[live]
    next_x: f32,
    #[live]
    next_w: f32,
    #[rust]
    prev_prev_rect: Option<Rect>,
    #[rust]
    prev_rect: Option<Rect>,
}

impl DrawSelection {
    fn begin(&mut self, _cx: &mut Cx2d<'_>) {
        debug_assert!(self.prev_rect.is_none());
    }

    fn end(&mut self, cx: &mut Cx2d<'_>) {
        self.draw_rect_internal(cx, None);
        self.prev_prev_rect = None;
        self.prev_rect = None;
    }

    fn draw_rect(&mut self, cx: &mut Cx2d<'_>, rect: Rect) {
        self.draw_rect_internal(cx, Some(rect));
        self.prev_prev_rect = self.prev_rect;
        self.prev_rect = Some(rect);
    }

    fn draw_rect_internal(&mut self, cx: &mut Cx2d, rect: Option<Rect>) {
        if let Some(prev_rect) = self.prev_rect {
            if let Some(prev_prev_rect) = self.prev_prev_rect {
                self.prev_x = prev_prev_rect.pos.x as f32;
                self.prev_w = prev_prev_rect.size.x as f32;
            } else {
                self.prev_x = 0.0;
                self.prev_w = 0.0;
            }
            if let Some(rect) = rect {
                self.next_x = rect.pos.x as f32;
                self.next_w = rect.size.x as f32;
            } else {
                self.next_x = 0.0;
                self.next_w = 0.0;
            }
            self.draw_abs(cx, prev_rect);
        }
    }
}
