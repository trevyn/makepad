use {
    crate::{
        layout,
        pos::{Affinity, PosWithAffinity},
        selection,
        selection::Region,
        state::{SessionId, View, ViewMut},
        Fold, Pos, State,
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
    #[rust]
    viewport_rect: Rect,
    #[rust]
    cell_size: DVec2,
    #[rust]
    start_line_idx: usize,
    #[rust]
    end_line_idx: usize,
    #[rust]
    start_y: f64,
}

impl CodeEditor {
    pub fn begin(&mut self, cx: &mut Cx2d<'_>, view: &mut ViewMut<'_>) {
        self.viewport_rect = Rect {
            pos: self.scroll_bars.get_scroll_pos(),
            size: cx.turtle().rect().size,
        };
        self.cell_size =
            self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);
        view.set_max_col_count(Some(
            (self.viewport_rect.size.x / self.cell_size.x) as usize,
        ));
        self.start_line_idx =
            view.find_first_line_ending_after_y(self.viewport_rect.pos.y / self.cell_size.y);
        self.end_line_idx = view.find_first_line_starting_after_y(
            (self.viewport_rect.pos.y + self.viewport_rect.size.y) / self.cell_size.y,
        );
        self.start_y = if self.start_line_idx == 0 {
            0.0
        } else {
            view.line_summed_height(self.start_line_idx - 1)
        };
        self.scroll_bars.begin(cx, self.walk, Layout::default());
    }

    pub fn end(&mut self, cx: &mut Cx2d<'_>, view: &mut ViewMut<'_>) {
        self.scroll_bars.end(cx);
        if view.update_folds() {
            cx.cx.redraw_all();
        }
    }

    pub fn draw_text(&mut self, cx: &mut Cx2d<'_>, view: &View<'_>) {
        use crate::{layout::Event, str::StrExt};

        let mut y = self.start_y;
        let mut col_idx = 0;
        for event in view.layout(self.start_line_idx..self.end_line_idx) {
            match event {
                Event::LineEnd { line, .. } | Event::Wrap { line, .. } => {
                    col_idx = 0;
                    y += line.fold().scale();
                }
                Event::TokenStart {
                    is_inlay_line,
                    line,
                    is_inlay_token,
                    token,
                    ..
                } => {
                    self.draw_text.font_scale = line.fold().scale();
                    self.draw_text.color = if is_inlay_line || is_inlay_token {
                        self.inlay_color
                    } else {
                        self.token_color
                    };
                    self.draw_text.draw_abs(
                        cx,
                        DVec2 {
                            x: line.fold().x(col_idx),
                            y,
                        } * self.cell_size
                            - self.viewport_rect.pos,
                        token.text,
                    );
                    col_idx += token.text.col_count();
                }
                _ => {}
            }
        }
    }

    pub fn draw_selection(&mut self, cx: &mut Cx2d<'_>, view: &View<'_>) {
        use crate::{blocks::Block};

        let mut active_region = None;
        let mut regions = view.selection().iter().peekable();
        while regions.peek().map_or(false, |region| {
            region.end().pos.line_idx < self.start_line_idx
        }) {
            regions.next();
        }
        if regions.peek().map_or(false, |region| {
            region.start().pos.line_idx < self.start_line_idx
        }) {
            active_region = Some(ActiveRegion {
                region: regions.next().unwrap(),
                start_x: 0.0,
            });
        }
        DrawSelectionContext {
            draw_selection: &mut self.draw_selection,
            draw_cursor: &mut self.draw_cursor,
            viewport_pos: self.viewport_rect.pos,
            cell_size: self.cell_size,
            active_region,
            regions,
            logical_pos: PosWithAffinity {
                pos: Pos {
                    line_idx: self.start_line_idx,
                    byte_idx: 0,
                },
                affinity: Affinity::Before,
            },
            y: self.start_y,
            col_idx: 0,
            fold: Fold::default(),
        }
        .draw_selection(cx, view.layout(self.start_line_idx..self.end_line_idx));

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
            .set_used(max_width * self.cell_size.x, height * self.cell_size.y);
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
                for line_idx in 0..view.line_count() {
                    if view
                        .line(line_idx)
                        .text()
                        .chars()
                        .take_while(|char| char.is_whitespace())
                        .count()
                        >= 8
                    {
                        view.fold_line(line_idx, 8);
                    }
                }
                cx.redraw_all();
            }
            Event::KeyUp(KeyEvent {
                key_code: KeyCode::Alt,
                ..
            }) => {
                for line_idx in 0..view.line_count() {
                    if view
                        .line(line_idx)
                        .text()
                        .chars()
                        .take_while(|char| char.is_whitespace())
                        .count()
                        >= 8
                    {
                        view.unfold_line(line_idx, 8);
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
    viewport_pos: DVec2,
    cell_size: DVec2,
    active_region: Option<ActiveRegion>,
    regions: Peekable<selection::Iter<'a>>,
    logical_pos: PosWithAffinity,
    y: f64,
    col_idx: usize,
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
                        self.logical_pos.affinity = Affinity::After;
                    }
                }
                layout::Event::LineEnd { is_inlay_line, .. } => {
                    if !is_inlay_line {
                        self.handle_event(cx);
                        if self.active_region.is_some() {
                            self.draw_selection_rect(cx);
                        }
                        self.logical_pos.pos.line_idx += 1;
                        self.logical_pos.pos.byte_idx = 0;
                        self.logical_pos.affinity = Affinity::Before;
                    }
                    self.y += self.fold.scale();
                    self.col_idx = 0;
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
                    self.col_idx = 0;
                }
                layout::Event::Grapheme {
                    is_inlay_line,
                    is_inlay_token,
                    grapheme,
                    ..
                } => {
                    if !is_inlay_line && !is_inlay_token {
                        self.handle_event(cx);
                        self.logical_pos.pos.byte_idx += grapheme.len();
                        self.logical_pos.affinity = Affinity::Before;
                    }
                    self.col_idx += grapheme.col_count();
                    if !is_inlay_line && !is_inlay_token {
                        self.handle_event(cx);
                        self.logical_pos.affinity = Affinity::After;
                    }
                }
                _ => {}
            }
        }
        if self.active_region.is_some() {
            self.draw_selection.end_region(cx);
        }
    }

    fn physical_pos(&self) -> DVec2 {
        DVec2 {
            x: self.fold.x(self.col_idx),
            y: self.y,
        } * self.cell_size
            - self.viewport_pos
    }

    fn height(&self) -> f64 {
        self.fold.scale() * self.cell_size.y
    }

    fn handle_event(&mut self, cx: &mut Cx2d<'_>) {
        if self
            .regions
            .peek()
            .map_or(false, |region| region.start() == self.logical_pos)
        {
            let region = self.regions.next().unwrap();
            if region.cursor.pos == self.logical_pos {
                self.draw_cursor(cx);
            }
            self.active_region = Some(ActiveRegion {
                region,
                start_x: self.physical_pos().x,
            });
            self.draw_selection.begin_region();
        }
        if self
            .active_region
            .as_ref()
            .map_or(false, |region| region.region.end() == self.logical_pos)
        {
            self.draw_selection_rect(cx);
            self.draw_selection.end_region(cx);
            let region = self.active_region.take().unwrap().region;
            if region.anchor.pos != region.cursor.pos.pos && region.cursor.pos == self.logical_pos {
                self.draw_cursor(cx);
            }
        }
    }

    fn draw_selection_rect(&mut self, cx: &mut Cx2d<'_>) {
        let physical_pos = self.physical_pos();
        let height = self.height();
        let region = self.active_region.as_mut().unwrap();
        self.draw_selection.draw_rect(
            cx,
            Rect {
                pos: DVec2 {
                    x: region.start_x,
                    y: physical_pos.y,
                },
                size: DVec2 {
                    x: physical_pos.x - region.start_x,
                    y: height,
                },
            },
        );
        region.start_x = 0.0;
    }

    fn draw_cursor(&mut self, cx: &mut Cx2d<'_>) {
        let physical_pos = self.physical_pos();
        let height = self.height();
        self.draw_cursor.draw_abs(
            cx,
            Rect {
                pos: physical_pos,
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
    fn begin_region(&mut self) {
        debug_assert!(self.prev_rect.is_none());
    }

    fn end_region(&mut self, cx: &mut Cx2d<'_>) {
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
