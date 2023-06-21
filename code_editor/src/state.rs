pub use crate::{
    blocks::{Block, BlockInlay},
    inlines::{Inline, InlineInlay},
    lines::{FoldState, FoldingState, Line},
    tokens::{Token, TokenInfo, TokenKind},
    Blocks, Inlines, Lines, Tokens,
};

use {
    crate::{arena::Id, Arena},
    std::{
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
        let session_id = SessionId(self.sessions.insert(Session {
            wrap_column_index: None,
            height: (0..document.text.len())
            .map(|_| 0.0).collect(),
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
            new_folding: HashMap::new(),
            unfolding: HashMap::new(),
            new_unfolding: HashMap::new(),
            block_inlays: vec![
                (10, BlockInlay::new("XXX YYY ZZZ")),
                (20, BlockInlay::new("XXX YYY ZZZ")),
                (30, BlockInlay::new("XXX YYY ZZZ")),
                (40, BlockInlay::new("XXX YYY ZZZ")),
            ],
        }));
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
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
            height: &session.height,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            breaks: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            block_inlays: &session.block_inlays,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            wrap_column_index: &mut session.wrap_column_index,
            height: &mut session.height,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
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
    height: &'a [f64],
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    block_inlays: &'a Vec<(usize, BlockInlay)>,
}

impl<'a> View<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line {
            height: self.height[line_index],
            text: &self.text[line_index],
            token_infos: &self.token_infos[line_index],
            inlays: &self.inline_inlays[line_index],
            breaks: &self.breaks[line_index],
            fold_state: FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
        }
    }

    pub fn lines(&self) -> Lines<'a> {
        use crate::lines;

        lines::lines(
            self.height.iter(),
            self.text.iter(),
            self.token_infos.iter(),
            self.inline_inlays.iter(),
            self.breaks.iter(),
            &self.folded,
            &self.folding,
            &self.unfolding,
        )
    }

    pub fn blocks(&self) -> Blocks<'a> {
        use crate::blocks;

        blocks::blocks(self.lines(), self.block_inlays.iter())
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    height: &'a mut [f64],
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            wrap_column_index: *self.wrap_column_index,
            height: &self.height,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            breaks: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            block_inlays: &self.block_inlays,
        }
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_view().line(line_index)
    }

    pub fn lines(&self) -> Lines<'_> {
        self.as_view().lines()
    }

    pub fn blocks(&self) -> Blocks<'_> {
        self.as_view().blocks()
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap(line_index);
            }
            for (_, block_inlay) in self.block_inlays.iter_mut() {
                block_inlay.wrap(wrap_column_index);
            }
        }
    }

    pub fn fold(&mut self, line_index: usize, column_index: usize) {
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

    pub fn unfold(&mut self, line_index: usize, column_index: usize) {
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

    pub fn update_fold_state(&mut self) -> bool {
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

    fn wrap(&mut self, line_index: usize) {
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
        let line = self.line(line_index);
        self.height[line_index] = line.fold_state.scale() * line.row_count() as f64;
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    height: Vec<f64>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
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
