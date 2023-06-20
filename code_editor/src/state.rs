mod blocks;
mod inlines;
mod lines;
mod tokens;

pub use self::{
    blocks::{Block, Blocks},
    inlines::{Inline, Inlines},
    lines::{FoldState, FoldingState, Line, Lines},
    tokens::{Token, TokenInfo, TokenKind, Tokens},
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
        let session_id = self.sessions.insert(Session {
            wrap_column_index: None,
            document_id,
            inline_inlays: (0..document.text.len())
                .map(|_| {
                    vec![
                        (20, Inlay::new("X Y Z")),
                        (40, Inlay::new("X Y Z")),
                        (60, Inlay::new("X Y Z")),
                        (80, Inlay::new("X Y Z")),
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
                (10, Inlay::new("X Y Z")),
                (20, Inlay::new("X Y Z")),
                (30, Inlay::new("X Y Z")),
                (40, Inlay::new("X Y Z")),
            ],
        });
        self.documents[document_id].session_ids.insert(session_id);
        Ok(SessionId(session_id))
    }

    pub fn close_session(&mut self, SessionId(session_id): SessionId) {
        let document_id = self.sessions[session_id].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id);
    }

    pub fn focus(&self, session_id: SessionId) -> Focus<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        Focus {
            wrap_column_index: session.wrap_column_index,
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


    pub fn focus_mut(&mut self, session_id: SessionId) -> FocusMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        FocusMut {
            wrap_column_index: &mut session.wrap_column_index,
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
        use std::fs;

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
        let token_infos = text.iter().map(|text| tokenize(text)).collect();
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

pub struct Focus<'a> {
    wrap_column_index: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, Inlay)>],
    breaks: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, FoldingState>,
    unfolding: &'a HashMap<usize, FoldingState>,
    block_inlays: &'a Vec<(usize, Inlay)>,
}

impl<'a> Focus<'a> {
    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn line(&self, line_index: usize) -> Line<'a> {
        Line {
            text: &self.text[line_index],
            token_infos: &self.token_infos[line_index],
            inlays: &self.inline_inlays[line_index],
            breaks: &self.breaks[line_index],
            fold_state: FoldState::new(line_index, &self.folded, &self.folding, &self.unfolding),
        }
    }

    pub fn lines(&self) -> Lines<'a> {
        Lines::new(
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
        Blocks::new(self.lines(), self.block_inlays.iter())
    }
}

pub struct FocusMut<'a> {
    wrap_column_index: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, Inlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, FoldingState>,
    unfolding: &'a mut HashMap<usize, FoldingState>,
    block_inlays: &'a mut Vec<(usize, Inlay)>,
    new_folding: &'a mut HashMap<usize, FoldingState>,
    new_unfolding: &'a mut HashMap<usize, FoldingState>,
}

impl<'a> FocusMut<'a> {
    pub fn as_focus(&self) -> Focus<'_> {
        Focus {
            wrap_column_index: *self.wrap_column_index,
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
        self.as_focus().line_count()
    }

    pub fn line(&self, line_index: usize) -> Line<'_> {
        self.as_focus().line(line_index)
    }

    pub fn lines(&self) -> Lines<'_> {
        self.as_focus().lines()
    }

    pub fn blocks(&self) -> Blocks<'_> {
        self.as_focus().blocks()
    }

    pub fn set_wrap_column_index(&mut self, wrap_column_index: Option<usize>) {
        if *self.wrap_column_index != wrap_column_index {
            *self.wrap_column_index = wrap_column_index;
            for line_index in 0..self.line_count() {
                self.wrap_line(line_index);
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
        true
    }

    fn wrap_line(&mut self, line_index: usize) {
        self.breaks[line_index] =
            if let Some(wrap_column_index) = *self.wrap_column_index {
                wrap(self.line(line_index), wrap_column_index)
            } else {
                Vec::new()
            };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Inlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl Inlay {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let token_infos = tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        Tokens::new(&self.text, self.token_infos.iter())
    }
}

#[derive(Debug)]
struct Session {
    wrap_column_index: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, Inlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, FoldingState>,
    unfolding: HashMap<usize, FoldingState>,
    block_inlays: Vec<(usize, Inlay)>,
    new_folding: HashMap<usize, FoldingState>,
    new_unfolding: HashMap<usize, FoldingState>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}

fn tokenize(text: &str) -> Vec<TokenInfo> {
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

fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::CharExt;

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.chars().map(|char| char.width()).sum();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => {}
        }
    }
    breaks
}
