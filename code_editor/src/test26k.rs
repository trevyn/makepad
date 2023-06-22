use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
use {
    makepad_code_editor::{code_editor, state::SessionId, CodeEditor},
    makepad_widgets::*,
};

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::hook_widget::HookWidget;

    App = {{App}} {
        ui: <DesktopWindow> {
            code_editor = <HookWidget> {}
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    code_editor: CodeEditor,
    #[rust]
    state: State,
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut cx = Cx2d::new(cx, event);
            while let Some(next) = self.ui.draw_widget(&mut cx).hook_widget() {
                if next == self.ui.get_widget(id!(code_editor)) {
                    self.code_editor.draw(
                        &mut cx,
                        &mut self.state.code_editor,
                        self.state.session_id,
                    );
                }
            }
            return;
        }
        self.code_editor.handle_event(
            cx,
            &mut self.state.code_editor,
            self.state.session_id,
            event,
        );
    }
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        code_editor::live_design(cx);
    }
}

struct State {
    code_editor: makepad_code_editor::State,
    session_id: SessionId,
}

impl Default for State {
    fn default() -> Self {
        use std::env;

        let mut code_editor = makepad_code_editor::State::new();
        let session_id = code_editor
            .open_session(Some(
                env::current_dir().unwrap().join("code_editor/src/state.rs"),
            ))
            .unwrap();
        Self {
            code_editor,
            session_id,
        }
    }
}

app_main!(App);
use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_index: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_index: Option<usize> },
}
use {
    crate::{inlay::BlockInlay, Line, Lines},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    line_index: usize,
    lines: Lines<'a>,
    inlays: Iter<'a, (usize, BlockInlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, _)) = self.inlays.as_slice().first() {
            if *index == self.line_index {
                let (_, inlay) = self.inlays.next().unwrap();
                return Some(Block::Line {
                    is_inlay: true,
                    line: inlay.as_line(),
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
}

pub fn blocks<'a>(lines: Lines<'a>, inlays: Iter<'a, (usize, BlockInlay)>) -> Blocks<'a> {
    Blocks {
        line_index: 0,
        lines,
        inlays,
    }
}
pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
use {
    crate::{
        fold::FoldingState,
        inlines::Inline,
        state::{Block, SessionId},
        tokens::Token,
        Line, State,
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

        state.view_mut(session_id).set_wrap_column_index(Some(
            (cx.turtle().rect().size.x / column_width as f64) as usize,
        ));
        
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        let scroll_position = self.scroll_bars.get_scroll_pos();

        let view = state.view(session_id);
        let start_line_index = view.find_first_line_ending_after_y(scroll_position.y / row_height);
        let end_line_index = view.find_last_line_starting_before_y((scroll_position.y + cx.turtle().rect().size.y) / row_height);
        let mut context = DrawContext {
            draw_text: &mut self.draw_text,
            row_height,
            column_width,
            inlay_color: self.inlay_color,
            token_color: self.token_color,
            scroll_position,
            row_y: view.line_y(start_line_index) * row_height,
            column_index: 0,
            inlay: false,
            fold_state: FoldingState::default(),
        };
        for block in view.blocks(start_line_index, end_line_index) {
            context.draw_block(cx, block);
        }

        let mut height = 0.0;
        let mut max_width = 0.0;
        for block in view.blocks(0, view.line_count()) {
            match block {
                Block::Line { line, .. } => {
                    height += line.height() * row_height;
                    max_width = max_width.max(line.width()) * column_width;
                }
            }
        }

        cx.turtle_mut().set_used(max_width, height);
        self.scroll_bars.end(cx);

        if state.view_mut(session_id).update_fold_states() {
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
        use crate::fold::FoldState;

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
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(FoldingState),
    Unfolding(FoldingState),
    Unfolded,
}

impl FoldState {
    pub fn new(
        index: usize,
        folded: &HashSet<usize>,
        folding_lines: &HashMap<usize, FoldingState>,
        unfolding_lines: &HashMap<usize, FoldingState>,
    ) -> Self {
        if folded.contains(&index) {
            Self::Folded
        } else if let Some(folding) = folding_lines.get(&index) {
            Self::Folding(*folding)
        } else if let Some(unfolding) = unfolding_lines.get(&index) {
            Self::Unfolding(*unfolding)
        } else {
            Self::Unfolded
        }
    }

    pub fn scale(self) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.scale,
            Self::Unfolded => 1.0,
        }
    }

    pub fn column_x(self, column_index: usize) -> f64 {
        match self {
            Self::Folded => 0.0,
            Self::Folding(state) | Self::Unfolding(state) => state.column_x(column_index),
            Self::Unfolded => column_index as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldingState {
    pub column_index: usize,
    pub scale: f64,
}

impl FoldingState {
    pub fn column_x(self, column_index: usize) -> f64 {
        let column_count_before = column_index.min(self.column_index);
        let column_count_after = column_index - column_count_before;
        column_count_before as f64 + self.scale * column_count_after as f64
    }
}

impl Default for FoldingState {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
    }
}
use crate::{fold::FoldState, tokenize::TokenInfo, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            FoldState::Unfolded,
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::tokens;

        tokens::tokens(&self.text, self.token_infos.iter())
    }
}
use {
    crate::{inlay::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token {
                    is_inlay: true,
                    token,
                });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
            self.token = Some(Token {
                text: text_1,
                kind: token.kind,
            });
            Token {
                text: text_0,
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
        Some(Inline::Token {
            is_inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { is_inlay: bool, token: Token<'a> },
    Break,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    breaks: Iter<'a, usize>,
) -> Inlines<'a> {
    Inlines {
        byte_offset: 0,
        inlay_byte_offset: 0,
        inlay_tokens: None,
        token: tokens.next(),
        tokens,
        inlays,
        breaks,
    }
}
pub mod arena;
pub mod blocks;
pub mod char_ext;
pub mod code_editor;
pub mod fold;
pub mod inlay;
pub mod inlines;
pub mod line;
pub mod lines;
pub mod state;
pub mod str_ext;
pub mod tokenize;
pub mod tokens;
pub mod wrap;

pub use self::{
    arena::Arena,
    blocks::{blocks, Blocks},
    char_ext::CharExt,
    code_editor::CodeEditor,
    inlines::{inlines, Inlines},
    line::Line,
    lines::{lines, Lines},
    state::State,
    str_ext::StrExt,
    tokens::{tokens, Tokens},
};
use crate::{fold::FoldState, inlay::InlineInlay, tokenize::TokenInfo, Inlines, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, InlineInlay)],
    breaks: &'a [usize],
    fold_state: FoldState,
    height: f64,
}

impl<'a> Line<'a> {
    pub fn new(
        text: &'a str,
        token_infos: &'a [TokenInfo],
        inlays: &'a [(usize, InlineInlay)],
        breaks: &'a [usize],
        fold_state: FoldState,
        height: f64,
    ) -> Self {
        Self {
            text,
            token_infos,
            inlays,
            breaks,
            fold_state,
            height,
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }

    pub fn row_count(&self) -> usize {
        self.breaks.len() + 1
    }

    pub fn column_count(&self) -> usize {
        use {crate::inlines::Inline, crate::StrExt};

        let mut column_count = 0;
        let mut max_column_count = 0;
        for inline in self.inlines() {
            match inline {
                Inline::Token { token, .. } => {
                    column_count += token.text.column_count();
                    max_column_count = max_column_count.max(column_count);
                }
                Inline::Break => column_count = 0,
            }
        }
        max_column_count
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.fold_state.column_x(self.column_count())
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        crate::tokens(self.text, self.token_infos.iter())
    }

    pub fn inlines(&self) -> Inlines<'a> {
        crate::inlines(self.tokens(), self.inlays.iter(), self.breaks.iter())
    }
}
use {
    crate::{
        fold::{FoldState, FoldingState},
        inlay::InlineInlay,
        tokenize::TokenInfo,
        Line,
    },
    std::{
        collections::{HashMap, HashSet},
        slice::Iter,
    },
};

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: Iter<'a, f64>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = Line::new(
            self.text.next()?,
            self.token_infos.next()?,
            self.inlays.next()?,
            self.breaks.next()?,
            FoldState::new(
                self.line_index,
                &self.folded,
                &self.folding,
                &self.unfolding,
            ),
            *self.heights.next()?,
        );
        self.line_index += 1;
        Some(line)
    }
}

pub fn lines<'a>(
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    breaks: Iter<'a, Vec<usize>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    height: Iter<'a, f64>,
) -> Lines<'a> {
    Lines {
        line_index: 0,
        text,
        token_infos,
        inlays,
        breaks,
        folded,
        folding,
        unfolding,
        heights: height,
    }
}
pub mod app;

fn main() {
    crate::app::app_main();
}
pub use {
    crate::{
        arena::Id,
        Arena,
        blocks::Block,
        fold::{FoldState, FoldingState},
        inlay::{BlockInlay, InlineInlay},
        inlines::Inline,
        tokenize::{TokenInfo, TokenKind},
        tokens::Token,
        Blocks, Inlines, Line, Lines, Tokens,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                wrap_column_index: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                new_folding: HashMap::new(),
                new_unfolding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for index in 0..5 {
            view.insert_block_inlay(index * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_index in 0..view.line_count() {
            view.update_height(line_index);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            wrap_column_index: session.wrap_column_index,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            new_folding: &mut session.new_folding,
            new_unfolding: &mut session.new_unfolding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self.summed_heights.borrow().binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap()) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line::new(
            &self.text[line_index],
            &self.token_infos[line_index],
            &self.inline_inlays[line_index],
            &self.breaks[line_index],
            FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
            self.heights[line_index],
        )
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.update_summed_heights();
        if line_index == 0 {
            0.0
        } else {
            self.summed_heights.borrow()[line_index - 1]
        }
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'a> {
        crate::lines(
            self.text[start_line_index..end_line_index].iter(),
            self.token_infos[start_line_index..end_line_index].iter(),
            self.inline_inlays[start_line_index..end_line_index].iter(),
            self.breaks[start_line_index..end_line_index].iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights[start_line_index..end_line_index].iter(),
        )
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'a> {
        crate::blocks(
            self.lines(start_line_index, end_line_index),
            self.block_inlays[self
                .block_inlays
                .iter()
                .position(|(line_index, _)| *line_index >= start_line_index)
                .unwrap_or(self.block_inlays.len())..]
                .iter(),
        )
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_index = summed_heights.len();
        let mut summed_height = if start_line_index == 0 {
            0.0
        } else {
            summed_heights[start_line_index - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_index, self.line_count()) {
            match block {
                Block::Line { is_inlay, line } => {
                    summed_height += line.height();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_last_line_starting_before_y(&self, y: f64) -> usize {
        self.as_view().find_last_line_starting_before_y(y)
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn line_y(&self, line_index: usize) -> f64 {
        self.as_view().line_y(line_index)
    }

    pub fn lines(&self, start_line_index: usize, end_line_index: usize) -> Lines<'_> {
        self.as_view().lines(start_line_index, end_line_index)
    }

    pub fn blocks(&self, start_line_index: usize, end_line_index: usize) -> Blocks<'_> {
        self.as_view().blocks(start_line_index, end_line_index)
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                let old_height = block_inlay.as_line().height();
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_index) {
            state.scale
        } else if !self.folded.contains(&line_index) && !self.folding.contains_key(&line_index) {
            1.0
        } else {
            return;
        };
        self.folding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn unfold_line(&mut self, line_index: usize, column_index: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_index) {
            state.scale
        } else if self.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(
            line_index,
            FoldingState {
                column_index,
                scale,
            },
        );
        self.update_height(line_index);
    }

    pub fn update_fold_states(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_index, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_index);
            } else {
                self.new_folding.insert(*line_index, state);
            }
        }
        mem::swap(self.folding, self.new_folding);
        self.new_folding.clear();
        for (line_index, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.new_unfolding.insert(*line_index, state);
            }
        }
        mem::swap(self.unfolding, self.new_unfolding);
        self.new_unfolding.clear();
        for line_index in 0..self.line_count() {
            self.update_height(line_index);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_index: usize, inlay: BlockInlay) {
        let index = match self
            .block_inlays
            .binary_search_by_key(&line_index, |&(line_index, _)| line_index)
        {
            Ok(index) => index,
            Err(index) => index,
        };
        self.block_inlays.insert(index, (line_index, inlay));
        self.summed_heights.borrow_mut().truncate(line_index);
    }

    fn wrap_line(&mut self, line_index: usize) {
        use crate::wrap;

        self.breaks[line_index] = Vec::new();
        self.breaks[line_index] = if let Some(wrap_column_index) = *self.wrap_column_index {
            wrap::wrap(self.line(line_index), wrap_column_index)
        } else {
            Vec::new()
        };
        self.update_height(line_index);
    }

    fn update_height(&mut self, line_index: usize) {
        let old_height = self.heights[line_index];
        let line = self.line(line_index);
        let new_height = line.fold_state().scale() * line.row_count() as f64;
        self.heights[line_index] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_index + 1);
        }
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
pub trait StrExt {
    fn column_count(&self) -> usize;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut index = 1;
        while !self.string.is_char_boundary(index) {
            index += 1;
        }
        let (grapheme, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let index = self
            .string
            .grapheme_indices()
            .find_map(|(index, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(index);
        self.string = remaining_string;
        Some(string)
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}
use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

pub fn tokens<'a>(text: &'a str, infos: Iter<'a, TokenInfo>) -> Tokens<'a> {
    Tokens { text, infos }
}
use crate::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
