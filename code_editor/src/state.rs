use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    slice::Iter,
};

#[derive(Debug, Default)]
pub struct State {
    document_id: usize,
    documents: HashMap<DocumentId, Document>,
    document_ids: HashMap<PathBuf, DocumentId>,
    session_id: usize,
    sessions: HashMap<SessionId, Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = path
            .as_ref()
            .and_then(|path| self.document_ids.get(path.as_ref()).copied())
            .map_or_else(|| self.open_document(path), |document| Ok(document))?;
        let session_id = {
            let session_id = SessionId(self.session_id);
            self.session_id += 1;
            session_id
        };
        let document = &self.documents[&document_id];
        let inline_inlays: Vec<_> = (0..document.text.len())
            .map(|_| {
                vec![
                    (20, Inlay::new("X Y Z")),
                    (40, Inlay::new("X Y Z")),
                    (60, Inlay::new("X Y Z")),
                    (80, Inlay::new("X Y Z")),
                ]
            })
            .collect();
        let break_byte_indices = document
            .text
            .iter()
            .enumerate()
            .map(|_| Vec::new())
            .collect();
        self.sessions.insert(
            session_id,
            Session {
                max_column_index: None,
                document_id,
                inline_inlays,
                break_byte_indices,
                folded_lines: HashSet::new(),
                folding_lines: HashMap::new(),
                new_folding_lines: HashMap::new(),
                unfolding_lines: HashMap::new(),
                new_unfolding_lines: HashMap::new(),
                block_inlays: [
                    (10, Inlay::new("X Y Z")),
                    (20, Inlay::new("X Y Z")),
                    (30, Inlay::new("X Y Z")),
                    (40, Inlay::new("X Y Z")),
                ]
                .into(),
            },
        );
        self.documents
            .get_mut(&document_id)
            .unwrap()
            .session_ids
            .insert(session_id);
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[&session_id].document_id;
        let document = self.documents.get_mut(&document_id).unwrap();
        document.session_ids.remove(&session_id);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(&session_id);
    }

    pub fn line_count(&self, session_id: SessionId) -> usize {
        self.documents[&self.sessions[&session_id].document_id]
            .text
            .len()
    }

    pub fn line(&self, session_id: SessionId, line_index: usize) -> Line<'_> {
        let session = &self.sessions[&session_id];
        let document = &self.documents[&session.document_id];
        Line {
            text: &document.text[line_index],
            token_infos: &document.token_infos[line_index],
            inlays: &session.inline_inlays[line_index],
            break_byte_indices: &session.break_byte_indices[line_index],
            fold_state: FoldState::new(
                line_index,
                &session.folded_lines,
                &session.folding_lines,
                &session.unfolding_lines,
            ),
        }
    }

    pub fn lines(&self, session_id: SessionId) -> Lines<'_> {
        let session = &self.sessions[&session_id];
        let document = &self.documents[&session.document_id];
        Lines {
            line_index: 0,
            text: document.text.iter(),
            token_infos: document.token_infos.iter(),
            inlays: session.inline_inlays.iter(),
            break_byte_indices: session.break_byte_indices.iter(),
            folded_lines: &session.folded_lines,
            folding_lines: &session.folding_lines,
            unfolding_lines: &session.unfolding_lines,
        }
    }

    pub fn blocks(&self, session_id: SessionId) -> Blocks<'_> {
        Blocks {
            lines: self.lines(session_id),
            line_index: 0,
            block_inlays: self.sessions[&session_id].block_inlays.iter(),
        }
    }

    pub fn set_max_column_index(&mut self, session_id: SessionId, max_column_index: Option<usize>) {
        let session = self.sessions.get_mut(&session_id).unwrap();
        if session.max_column_index != max_column_index {
            session.max_column_index = max_column_index;
            for line_index in 0..self.line_count(session_id) {
                self.wrap_line(session_id, line_index);
            }
        }
    }

    pub fn fold_line(&mut self, session_id: SessionId, line_index: usize, column_index: usize) {
        let session = self.sessions.get_mut(&session_id).unwrap();
        let scale = if let Some(unfolding_lines) = session.unfolding_lines.remove(&line_index) {
            unfolding_lines.scale
        } else if !session.folded_lines.contains(&line_index)
            && !session.folding_lines.contains_key(&line_index)
        {
            1.0
        } else {
            return;
        };
        session.folding_lines.insert(
            line_index,
            Fold {
                column_index,
                scale,
            },
        );
    }

    pub fn unfold_line(&mut self, session_id: SessionId, line_index: usize, column_index: usize) {
        let session = self.sessions.get_mut(&session_id).unwrap();
        let scale = if let Some(folding_lines) = session.folding_lines.remove(&line_index) {
            folding_lines.scale
        } else if session.folded_lines.remove(&line_index) {
            0.0
        } else {
            return;
        };
        session.unfolding_lines.insert(
            line_index,
            Fold {
                column_index,
                scale,
            },
        );
    }

    pub fn update_fold_state(&mut self, session_id: SessionId) -> bool {
        use std::mem;

        let session = self.sessions.get_mut(&session_id).unwrap();
        if session.folding_lines.is_empty() && session.unfolding_lines.is_empty() {
            return false;
        }
        for (line_index, fold) in &session.folding_lines {
            let mut fold = *fold;
            fold.scale *= 0.9;
            if fold.scale < 0.001 {
                session.folded_lines.insert(*line_index);
            } else {
                session.new_folding_lines.insert(*line_index, fold);
            }
        }
        mem::swap(&mut session.folding_lines, &mut session.new_folding_lines);
        session.new_folding_lines.clear();
        for (line_index, fold) in &session.unfolding_lines {
            let mut fold = *fold;
            fold.scale = 1.0 - 0.9 * (1.0 - fold.scale);
            if 1.0 - fold.scale > 0.001 {
                session.new_unfolding_lines.insert(*line_index, fold);
            }
        }
        mem::swap(
            &mut session.unfolding_lines,
            &mut session.new_unfolding_lines,
        );
        session.new_unfolding_lines.clear();
        true
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<DocumentId> {
        use std::fs;

        let document_id = {
            let document_id = DocumentId(self.document_id);
            self.document_id += 1;
            document_id
        };
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
        self.documents.insert(
            document_id,
            Document {
                session_ids: HashSet::new(),
                path: path.map(|path| path.into()),
                text,
                token_infos,
            },
        );
        if let Some(path) = &self.documents[&document_id].path {
            self.document_ids.insert(path.clone(), document_id);
        }
        Ok(document_id)
    }

    fn close_document(&mut self, document_id: DocumentId) {
        if let Some(path) = &self.documents[&document_id].path {
            self.document_ids.remove(path);
        }
        self.documents.remove(&document_id);
    }

    fn wrap_line(&mut self, session_id: SessionId, line_index: usize) {
        let break_byte_indices =
            if let Some(max_column_index) = self.sessions[&session_id].max_column_index {
                wrap(self.line(session_id, line_index), max_column_index)
            } else {
                Vec::new()
            };
        let session = self.sessions.get_mut(&session_id).unwrap();
        session.break_byte_indices[line_index] = break_byte_indices;
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(usize);

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
        Tokens {
            text: &self.text,
            token_infos: self.token_infos.iter(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fold {
    pub column_index: usize,
    pub scale: f64,
}

impl Default for Fold {
    fn default() -> Self {
        Self {
            column_index: 0,
            scale: 1.0,
        }
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

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inlays: Iter<'a, Vec<(usize, Inlay)>>,
    break_byte_indices: Iter<'a, Vec<usize>>,
    folded_lines: &'a HashSet<usize>,
    folding_lines: &'a HashMap<usize, Fold>,
    unfolding_lines: &'a HashMap<usize, Fold>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line_index = {
            let line_index = self.line_index;
            self.line_index += 1;
            line_index
        };
        Some(Line {
            text: self.text.next()?,
            token_infos: self.token_infos.next()?,
            inlays: self.inlays.next()?,
            break_byte_indices: self.break_byte_indices.next()?,
            fold_state: FoldState::new(
                line_index,
                &self.folded_lines,
                &self.folding_lines,
                &self.unfolding_lines,
            ),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inlays: &'a [(usize, Inlay)],
    break_byte_indices: &'a [usize],
    fold_state: FoldState,
}

impl<'a> Line<'a> {
    pub fn text(&self) -> &str {
        self.text
    }

    pub fn tokens(&self) -> Tokens<'a> {
        Tokens {
            text: self.text,
            token_infos: self.token_infos.iter(),
        }
    }

    pub fn inlines(&self) -> Inlines<'a> {
        let mut tokens = self.tokens();
        let token = tokens.next();
        Inlines {
            byte_index: 0,
            inlay_byte_index: 0,
            token,
            tokens,
            inlay_tokens: None,
            inlays: self.inlays.iter(),
            break_byte_indices: self.break_byte_indices.iter(),
        }
    }

    pub fn fold_state(&self) -> FoldState {
        self.fold_state
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(Fold),
    Unfolding(Fold),
    Unfolded,
}

impl FoldState {
    fn new(
        line_index: usize,
        folded_lines: &HashSet<usize>,
        folding_lines: &HashMap<usize, Fold>,
        unfolding_lines: &HashMap<usize, Fold>,
    ) -> Self {
        if folded_lines.contains(&line_index) {
            Self::Folded
        } else if let Some(folding_lines) = folding_lines.get(&line_index) {
            Self::Folding(*folding_lines)
        } else if let Some(unfolding_lines) = unfolding_lines.get(&line_index) {
            Self::Unfolding(*unfolding_lines)
        } else {
            Self::Unfolded
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    token_infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let token_info = self.token_infos.next()?;
        let (text, remaining_text) = self.text.split_at(token_info.byte_count);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: token_info.kind,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_index: usize,
    inlay_byte_index: usize,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlay_tokens: Option<Tokens<'a>>,
    inlays: Iter<'a, (usize, Inlay)>,
    break_byte_indices: Iter<'a, usize>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(byte_index) = self.break_byte_indices.as_slice().first() {
            if *byte_index == self.inlay_byte_index {
                self.break_byte_indices.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_index, _)) = self.inlays.as_slice().first() {
            if *byte_index == self.byte_index {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_index += token.text.len();
                return Some(Inline::Token { inlay: true, token });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_index, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(*byte_index - self.byte_index);
        }
        let token = if byte_count < token.text.len() {
            self.token = Some(Token {
                text: &token.text[byte_count..],
                kind: token.kind,
            });
            Token {
                text: &token.text[..byte_count],
                kind: token.kind,
            }
        } else {
            self.token = self.tokens.next();
            token
        };
        self.byte_index += token.text.len();
        self.inlay_byte_index += token.text.len();
        Some(Inline::Token {
            inlay: false,
            token,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token { inlay: bool, token: Token<'a> },
    Break,
}

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    lines: Lines<'a>,
    line_index: usize,
    block_inlays: Iter<'a, (usize, Inlay)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((line_index, _)) = self.block_inlays.as_slice().first() {
            if *line_index == self.line_index {
                let (_, inlay) = self.block_inlays.next().unwrap();
                return Some(Block::Line {
                    inlay: true,
                    line: Line {
                        text: &inlay.text,
                        token_infos: &inlay.token_infos,
                        inlays: &[],
                        break_byte_indices: &[],
                        fold_state: FoldState::Unfolded,
                    },
                });
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line { inlay: false, line })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line { inlay: bool, line: Line<'a> },
}

#[derive(Debug)]
struct Session {
    max_column_index: Option<usize>,
    document_id: DocumentId,
    inline_inlays: Vec<Vec<(usize, Inlay)>>,
    break_byte_indices: Vec<Vec<usize>>,
    folded_lines: HashSet<usize>,
    folding_lines: HashMap<usize, Fold>,
    new_folding_lines: HashMap<usize, Fold>,
    unfolding_lines: HashMap<usize, Fold>,
    new_unfolding_lines: HashMap<usize, Fold>,
    block_inlays: Vec<(usize, Inlay)>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct DocumentId(usize);

#[derive(Debug)]
struct Document {
    session_ids: HashSet<SessionId>,
    path: Option<PathBuf>,
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

fn wrap(line: Line<'_>, max_column_index: usize) -> Vec<usize> {
    use crate::CharExt;

    let mut break_byte_indices = Vec::new();
    let mut inlay_byte_index = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.chars().map(|char| char.column_count()).sum();
                if column_index + column_count > max_column_index {
                    break_byte_indices.push(inlay_byte_index);
                    column_index = 0;
                }
                inlay_byte_index += token.text.len();
                column_index += column_count;
            }
            _ => {}
        }
    }
    break_byte_indices
}
