use {
    crate::{
        blocks::Block,
        inlines::Inline,
        position::{Bias, BiasedPosition},
        selection::{Iter, Region},
        state::{SessionId, View, ViewMut},
        tokens::Token,
        Fold, Position, State,
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
            color: #C0C0C0,
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
    line_start: usize,
    #[rust]
    line_end: usize,
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
        view.set_max_column_count(Some(
            (self.viewport_rect.size.x / self.cell_size.x) as usize,
        ));
        self.line_start =
            view.find_first_line_ending_after_y(self.viewport_rect.pos.y / self.cell_size.y);
        self.line_end = view.find_first_line_starting_after_y(
            (self.viewport_rect.pos.y + self.viewport_rect.size.y) / self.cell_size.y,
        );
        self.start_y = if self.line_start == 0 {
            0.0
        } else {
            view.line_summed_height(self.line_start - 1)
        };
        self.scroll_bars.begin(cx, self.walk, Layout::default());
    }

    pub fn end(&mut self, cx: &mut Cx2d<'_>, view: &mut ViewMut<'_>) {
        cx.turtle_mut()
            .set_used(view.max_width() * self.cell_size.x, view.height() * self.cell_size.y);
        self.scroll_bars.end(cx);
        if view.update_folds() {
            cx.cx.redraw_all();
        }
    }

    pub fn draw_text(&mut self, cx: &mut Cx2d<'_>, view: &View<'_>) {
        let y = self.start_y;
        DrawTextCx {
            cx,
            code_editor: self,
            is_inlay_line: false,
            is_inlay_token: false,
            fold: Fold::default(),
            y,
            column_index: 0,
        }
        .visit_view(view)
    }

    pub fn draw_selection(&mut self, cx: &mut Cx2d<'_>, view: &View<'_>) {
        let mut active_region = None;
        let mut regions = view.selection().iter().peekable();
        while regions.peek().map_or(false, |region| {
            region.end().position.line_index < self.line_start
        }) {
            regions.next().unwrap();
        }
        if regions.peek().map_or(false, |region| {
            region.start().position.line_index < self.line_start
        }) {
            active_region = Some(ActiveRegion {
                region: regions.next().unwrap(),
                start_x: 0.0,
            });
        }
        let line_index = self.line_start;
        let y = self.start_y;
        DrawSelectionCx {
            cx,
            code_editor: self,
            active_region,
            regions,
            line_index,
            byte_index: 0,
            bias: Bias::Before,
            fold: Fold::default(),
            y,
            column_index: 0,
        }
        .visit_view(view)
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

struct DrawTextCx<'a, 'b> {
    cx: &'a mut Cx2d<'b>,
    code_editor: &'a mut CodeEditor,
    is_inlay_line: bool,
    is_inlay_token: bool,
    fold: Fold,
    y: f64,
    column_index: usize,
}

impl<'a, 'b> DrawTextCx<'a, 'b> {
    fn screen_position(&self) -> DVec2 {
        DVec2 {
            x: self.fold.width(self.column_index),
            y: self.y,
        } * self.code_editor.cell_size
            - self.code_editor.viewport_rect.pos
    }

    fn visit_view(&mut self, view: &View<'_>) {
        for block in view.blocks(self.code_editor.line_start..self.code_editor.line_end) {
            self.visit_block(block);
        }
    }

    fn visit_block(&mut self, block: Block<'_>) {
        match block {
            Block::Line(is_inlay_line, line) => {
                self.is_inlay_line = is_inlay_line;
                self.fold = line.fold();
                for inline in line.inlines() {
                    self.visit_inline(inline);
                }
                self.visit_new_row();
                self.fold = Fold::default();
                self.is_inlay_line = false;
            }
        }
    }

    fn visit_inline(&mut self, inline: Inline<'_>) {
        use crate::str::StrExt;

        match inline {
            Inline::Token(is_inlay_token, token) => {
                self.is_inlay_token = is_inlay_token;
                self.draw_token(token);
                self.column_index += token.text.column_count();
                self.is_inlay_token = false;
            }
            Inline::Wrap => {
                self.visit_new_row();
            }
        }
    }

    fn draw_token(&mut self, token: Token<'_>) {
        self.code_editor.draw_text.font_scale = self.fold.scale();
        self.code_editor.draw_text.color = if self.is_inlay_line || self.is_inlay_token {
            self.code_editor.inlay_color
        } else {
            self.code_editor.token_color
        };
        self.code_editor
            .draw_text
            .draw_abs(self.cx, self.screen_position(), token.text);
    }

    fn visit_new_row(&mut self) {
        self.y += self.fold.scale();
        self.column_index = 0;
    }
}

struct DrawSelectionCx<'a, 'b> {
    cx: &'a mut Cx2d<'b>,
    code_editor: &'a mut CodeEditor,
    active_region: Option<ActiveRegion>,
    regions: Peekable<Iter<'a>>,
    line_index: usize,
    byte_index: usize,
    bias: Bias,
    fold: Fold,
    y: f64,
    column_index: usize,
}

impl<'a, 'b> DrawSelectionCx<'a, 'b> {
    fn text_position(&self) -> BiasedPosition {
        BiasedPosition {
            position: Position {
                line_index: self.line_index,
                byte_index: self.byte_index,
            },
            bias: self.bias,
        }
    }

    fn screen_position(&self) -> DVec2 {
        DVec2 {
            x: self.fold.width(self.column_index),
            y: self.y,
        } * self.code_editor.cell_size
            - self.code_editor.viewport_rect.pos
    }

    fn visit_view(&mut self, view: &View<'_>) {
        for block in view.blocks(self.code_editor.line_start..self.code_editor.line_end) {
            self.visit_block(block);
        }
    }

    fn visit_block(&mut self, block: Block<'_>) {
        match block {
            Block::Line(false, line) => {
                self.fold = line.fold();
                self.visit_gap();
                self.bias = Bias::After;
                for inline in line.inlines() {
                    self.visit_inline(inline);
                }
                self.visit_gap();
                self.visit_new_row();
                self.line_index += 1;
                self.byte_index = 0;
                self.bias = Bias::Before;
                self.fold = Fold::default();
            }
            _ => {
                if self.active_region.is_some() {
                    self.code_editor.draw_selection.end(self.cx);
                }
                self.y += block.height();
                if self.active_region.is_some() {
                    self.code_editor.draw_selection.begin();
                }
            }
        }
    }

    fn visit_inline(&mut self, inline: Inline<'_>) {
        use crate::str::StrExt;

        match inline {
            Inline::Token(false, token) => {
                for grapheme in token.text.graphemes() {
                    self.visit_grapheme(grapheme);
                }
            }
            Inline::Wrap => {
                self.visit_new_row();
            }
            _ => self.column_index += inline.column_count(),
        }
    }

    fn visit_grapheme(&mut self, grapheme: &str) {
        use crate::str::StrExt;

        self.visit_gap();
        self.byte_index += grapheme.len();
        self.column_index += grapheme.column_count();
        self.bias = Bias::Before;
        self.visit_gap();
        self.bias = Bias::After;
    }

    fn visit_gap(&mut self) {
        let text_position = self.text_position();
        if self
            .regions
            .peek()
            .map_or(false, |region| region.start() == text_position)
        {
            let region = self.regions.next().unwrap();
            if region.cursor.position == text_position {
                self.draw_cursor();
            }
            self.active_region = Some(ActiveRegion {
                region,
                start_x: self.screen_position().x,
            });
            self.code_editor.draw_selection.begin();
        }
        if self
            .active_region
            .as_ref()
            .map_or(false, |region| region.region.end() == text_position)
        {
            self.draw_selection_rect();
            self.code_editor.draw_selection.end(self.cx);
            let region = self.active_region.take().unwrap().region;
            if region.anchor.position != region.cursor.position.position
                && region.cursor.position == text_position
            {
                self.draw_cursor();
            }
        }
    }

    fn visit_new_row(&mut self) {
        self.column_index += 1;
        if self.active_region.is_some() {
            self.draw_selection_rect();
        }
        self.y += self.fold.scale();
        self.column_index = 0;
    }

    fn draw_selection_rect(&mut self) {
        use std::mem;

        let screen_position = self.screen_position();
        let start_x = mem::take(&mut self.active_region.as_mut().unwrap().start_x);
        self.code_editor.draw_selection.draw_rect(
            self.cx,
            Rect {
                pos: DVec2 {
                    x: start_x,
                    y: screen_position.y,
                },
                size: DVec2 {
                    x: screen_position.x - start_x,
                    y: self.fold.scale() * self.code_editor.cell_size.y,
                },
            },
        );
    }

    fn draw_cursor(&mut self) {
        let screen_position = self.screen_position();
        self.code_editor.draw_cursor.draw_abs(
            self.cx,
            Rect {
                pos: screen_position,
                size: DVec2 {
                    x: 2.0,
                    y: self.fold.scale() * self.code_editor.cell_size.y,
                },
            },
        );
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
    fn begin(&mut self) {
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