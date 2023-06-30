use {
    crate::{
        layout,
        position::{Affinity, PositionWithAffinity},
        selection,
        selection::Region,
        state::SessionId,
        Fold, State, Position,
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
        use crate::{blocks::Block, str::StrExt};

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
        let start_y = if start_line_index == 0 {
            0.0
        } else {
            view.line_summed_height(start_line_index - 1)
        };

        let mut y = start_y;
        let mut column_index = 0;
        for event in view.layout(start_line_index..end_line_index) {
            match event {
                layout::Event::LineStart { line, .. } => {
                    self.draw_text.font_scale = line.fold().scale();
                }
                layout::Event::LineEnd { line, .. } | layout::Event::Wrap { line, .. } => {
                    column_index = 0;
                    y += line.fold().scale();
                }
                layout::Event::TokenStart {
                    is_inlay_line,
                    line,
                    is_inlay_token,
                    token,
                    ..
                } => {
                    self.draw_text.color = if is_inlay_line || is_inlay_token {
                        self.inlay_color
                    } else {
                        self.token_color
                    };
                    self.draw_text.draw_abs(
                        cx,
                        DVec2 {
                            x: line.fold().x(column_index),
                            y,
                        } * cell_size
                            - viewport_position,
                        token.text,
                    );
                    column_index += token.text.column_count();
                }
                _ => {}
            }
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
        DrawSelectionContext {
            draw_selection: &mut self.draw_selection,
            draw_cursor: &mut self.draw_cursor,
            viewport_position,
            cell_size,
            active_region,
            regions,
            logical_position: PositionWithAffinity {
                position: Position {
                    line_index: start_line_index,
                    byte_index: 0,
                },
                affinity: Affinity::Before,
            },
            y: 0.0,
            column_index: 0,
            fold: Fold::default(),
        }.draw_selection(cx, view.layout(start_line_index..end_line_index));

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

struct DrawSelectionContext<'a> {
    draw_selection: &'a mut DrawSelection,
    draw_cursor: &'a mut DrawColor,
    viewport_position: DVec2,
    cell_size: DVec2,
    active_region: Option<ActiveRegion>,
    regions: Peekable<selection::Iter<'a>>,
    logical_position: PositionWithAffinity,
    y: f64,
    column_index: usize,
    fold: Fold,
}

impl<'a> DrawSelectionContext<'a> {
    fn draw_selection(&mut self, cx: &mut Cx2d<'_>, layout: layout::Layout<'a>) {
        use crate::str::StrExt;

        for event in layout {
            match event {
                layout::Event::LineStart {
                    is_inlay_line,
                    line,
                    ..
                } => {
                    self.fold = line.fold();
                    if !is_inlay_line {
                        self.handle_event(cx);
                        self.logical_position.affinity = Affinity::After;
                    }
                }
                layout::Event::LineEnd { is_inlay_line, .. } => {
                    if !is_inlay_line {
                        self.handle_event(cx);
                        if self.active_region.is_some() {
                            self.draw_selection_rect(cx);
                        }
                        self.logical_position.position.line_index += 1;
                        self.logical_position.position.byte_index = 0;
                        self.logical_position.affinity = Affinity::Before;
                    }
                    self.y += self.fold.scale();
                    self.column_index = 0;
                    self.fold = Fold::default();
                }
                layout::Event::Wrap {
                    is_inlay_line,
                    line,
                    ..
                } => {
                    if !is_inlay_line && self.active_region.is_some() {
                        self.draw_selection_rect(cx);
                    }
                    self.y += line.fold().scale();
                    self.column_index = 0;
                }
                layout::Event::Grapheme {
                    is_inlay_line,
                    is_inlay_token,
                    grapheme,
                    ..
                } => {
                    if !is_inlay_line && !is_inlay_token {
                        self.handle_event(cx);
                        self.logical_position.position.byte_index += grapheme.len();
                        self.logical_position.affinity = Affinity::Before;
                    }
                    self.column_index += grapheme.column_count();
                    if !is_inlay_line && !is_inlay_token {
                        self.handle_event(cx);
                        self.logical_position.affinity = Affinity::After;
                    }
                }
                _ => {}
            }
        }
        if self.active_region.is_some() {
            self.draw_selection.end(cx);
        }
    }

    fn physical_position(&self) -> DVec2 {
        DVec2 {
            x: self.fold.x(self.column_index),
            y: self.y,
        } * self.cell_size
            - self.viewport_position
    }

    fn height(&self) -> f64 {
        self.fold.scale() * self.cell_size.y
    }

    fn handle_event(&mut self, cx: &mut Cx2d<'_>) {
        if self
            .regions
            .peek()
            .map_or(false, |region| region.start() == self.logical_position)
        {
            let region = self.regions.next().unwrap();
            if region.cursor.position == self.logical_position {
                self.draw_cursor(cx);
            }
            self.active_region = Some(ActiveRegion {
                region,
                start_x: self.physical_position().x,
            });
            self.draw_selection.begin(cx);
        }
        if self
            .active_region
            .as_ref()
            .map_or(false, |region| region.region.end() == self.logical_position)
        {
            self.draw_selection_rect(cx);
            self.draw_selection.end(cx);
            let region = self.active_region.take().unwrap().region;
            if region.anchor.position != region.cursor.position.position
                && region.cursor.position == self.logical_position
            {
                self.draw_cursor(cx);
            }
        }
    }

    fn draw_selection_rect(&mut self, cx: &mut Cx2d<'_>) {
        let physical_position = self.physical_position();
        let height = self.height();
        let region = self.active_region.as_mut().unwrap();
        self.draw_selection.draw_rect(
            cx,
            Rect {
                pos: DVec2 {
                    x: region.start_x,
                    y: physical_position.y,
                },
                size: DVec2 {
                    x: physical_position.x - region.start_x,
                    y: height,
                },
            },
        );
        region.start_x = 0.0;
    }

    fn draw_cursor(&mut self, cx: &mut Cx2d<'_>) {
        let physical_position = self.physical_position();
        let height = self.height();
        self.draw_cursor.draw_abs(
            cx,
            Rect {
                pos: physical_position,
                size: DVec2 { x: 2.0, y: height },
            },
        )
    }
}

struct ActiveRegion {
    region: Region,
    start_x: f64,
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
