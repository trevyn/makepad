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
    inlay_color: Vec4,
    #[live]
    token_color: Vec4,
}

impl CodeEditor {
    pub fn draw(&mut self, cx: &mut Cx2d<'_>, state: &mut State, session_id: SessionId) {
        use crate::blocks::Block;

        let viewport_position = self.scroll_bars.get_scroll_pos();
        let viewport_size = cx.turtle().rect().size;
        let cell_size = self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);

        state
            .view_mut(session_id)
            .set_max_column_count(Some((viewport_size.x / cell_size.x) as usize));

        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let view = state.view(session_id);
        let line_start = view.find_first_line_ending_after_y(viewport_position.y / cell_size.y);
        let line_end = view.find_first_line_starting_after_y(
            (viewport_position.y + viewport_size.y) / cell_size.y,
        );
        let y = if line_start == 0 {
            0.0
        } else {
            view.line_summed_height(line_start - 1)
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
        for block in view.blocks(line_start..line_end) {
            visitor.visit_block(block);
        }

        let mut visitor = DrawOverlayVisitor::new(
            cx,
            &mut self.draw_selection,
            viewport_position,
            cell_size,
            view.selection().iter().map(|region| region.range()),
            line_start,
            y
        );
        for block in view.blocks(line_start..line_end) {
            visitor.visit_block(block);
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

struct DrawOverlayVisitor<'a, 'b, I, D>
where
    I: Iterator<Item = Range<PositionWithAffinity>>,
    D: DrawOverlay,
{
    cx: &'a mut Cx2d<'b>,
    draw_overlay: &'a mut D,
    viewport_position: DVec2,
    cell_size: DVec2,
    active_range: Option<ActiveRange>,
    ranges: Peekable<I>,
    logical_position: PositionWithAffinity,
    fold: Fold,
    y: f64,
    column_index: usize,
}

impl<'a, 'b, I, D> DrawOverlayVisitor<'a, 'b, I, D>
where
    I: Iterator<Item = Range<PositionWithAffinity>>,
    D: DrawOverlay,
{
    fn new(
        cx: &'a mut Cx2d<'b>,
        draw_overlay: &'a mut D,
        viewport_position: DVec2,
        cell_size: DVec2,
        ranges: I,
        line_start: usize,
        y: f64,
    ) -> Self {
        let mut active_range = None;
        let mut ranges = ranges.peekable();
        while ranges
            .peek()
            .map_or(false, |range| range.end.position.line_index < line_start)
        {
            ranges.next();
        }
        if ranges
            .peek()
            .map_or(false, |range| range.start.position.line_index < line_start)
        {
            active_range = Some(ActiveRange {
                range: ranges.next().unwrap(),
                start_x: 0.0,
            });
        }
        Self {
            cx,
            draw_overlay,
            viewport_position,
            cell_size,
            active_range,
            ranges,
            logical_position: PositionWithAffinity {
                position: Position {
                    line_index: line_start,
                    byte_index: 0,
                },
                affinity: Affinity::Before,
            },
            fold: Fold::default(),
            y,
            column_index: 0,
        }
    }

    fn physical_position(&self) -> DVec2 {
        DVec2 {
            x: self.fold.x(self.column_index),
            y: self.y,
        } * self.cell_size
            - self.viewport_position
    }

    fn handle_event(&mut self) {
        if self
            .ranges
            .peek()
            .map_or(false, |range| range.start == self.logical_position)
        {
            self.draw_overlay.begin(self.cx);
            self.active_range = Some(ActiveRange {
                range: self.ranges.next().unwrap(),
                start_x: self.physical_position().x,
            });
        }
        if self
            .active_range
            .as_ref()
            .map_or(false, |range| range.range.end == self.logical_position)
        {
            self.draw_rect();
            self.draw_overlay.end(self.cx);
            self.active_range.take().unwrap();
        }
    }

    fn draw_rect(&mut self) {
        use std::mem;

        let start_x = mem::replace(&mut self.active_range.as_mut().unwrap().start_x, 0.0);
        let DVec2 { x: end_x, y } = self.physical_position();
        self.draw_overlay.draw_rect(
            self.cx,
            Rect {
                pos: DVec2 { x: start_x, y },
                size: DVec2 {
                    x: end_x - start_x,
                    y: self.fold.scale() * self.cell_size.y,
                },
            },
        );
    }
}

impl<'a, 'b, I, D> Visitor for DrawOverlayVisitor<'a, 'b, I, D>
where
    I: Iterator<Item = Range<PositionWithAffinity>>,
    D: DrawOverlay,
{
    fn visit_line(&mut self, is_inlay: bool, line: Line<'_>) {
        use crate::visit;

        if is_inlay {
            self.column_index += 1;
            if self.active_range.is_some() {
                self.draw_rect();
            }
            self.column_index = 0;
            self.y += line.fold().scale();
        } else {
            self.fold = line.fold();
            self.handle_event();
            visit::walk_line(self, line);
            self.handle_event();
            self.column_index += 1;
            if self.active_range.is_some() {
                self.draw_rect();
            }
            self.logical_position.position.line_index += 1;
            self.logical_position.position.byte_index = 0;
            self.logical_position.affinity = Affinity::Before;
            self.column_index = 0;
            self.y += self.fold.scale();
            self.fold = Fold::default();
        }
    }

    fn visit_token(&mut self, is_inlay: bool, token: Token<'_>) {
        use crate::{str::StrExt, visit};

        if is_inlay {
            self.column_index += token.text.column_count();
        } else {
            visit::walk_token(self, token);
        }
    }

    fn visit_grapheme(&mut self, grapheme: &str) {
        use crate::str::StrExt;

        self.handle_event();
        self.logical_position.position.byte_index += grapheme.len();
        self.logical_position.affinity = Affinity::Before;
        self.column_index += grapheme.column_count();
        self.handle_event();
        self.logical_position.affinity = Affinity::After;
    }

    fn visit_wrap(&mut self) {
        self.column_index += 1;
        if self.active_range.is_some() {
            self.draw_rect();
        }
        self.column_index = 0;
        self.y += self.fold.scale();
    }
}

struct ActiveRange {
    range: Range<PositionWithAffinity>,
    start_x: f64,
}

trait DrawOverlay {
    fn begin(&mut self, cx: &mut Cx2d<'_>);
    fn end(&mut self, cx: &mut Cx2d<'_>);
    fn draw_rect(&mut self, cx: &mut Cx2d<'_>, rect: Rect);
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

impl DrawOverlay for DrawSelection {
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
}
