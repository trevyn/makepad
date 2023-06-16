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
        self.sessions.insert(
            session_id,
            Session {
                document_id,
                inline_inlays: (0..self.documents[&document_id].text.len())
                    .map(|_| {
                        [
                            (20, "X".to_string()),
                            (40, "X".to_string()),
                            (60, "X".to_string()),
                            (80, "X".to_string()),
                        ]
                        .into()
                    })
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                new_folding: HashMap::new(),
                unfolding: HashMap::new(),
                new_unfolding: HashMap::new(),
                block_inlays: [
                    (10, "XXX".to_string()),
                    (20, "XXX".to_string()),
                    (30, "XXX".to_string()),
                    (40, "XXX".to_string()),
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
            line_index,
            text: &document.text[line_index],
            token_infos: &document.token_infos[line_index],
            inline_inlays: &session.inline_inlays[line_index],
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
        }
    }

    pub fn lines(&self, session_id: SessionId) -> Lines<'_> {
        let session = &self.sessions[&session_id];
        let document = &self.documents[&session.document_id];
        Lines {
            line_index: 0,
            text: document.text.iter(),
            token_infos: document.token_infos.iter(),
            inline_inlays: session.inline_inlays.iter(),
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
        }
    }

    pub fn blocks(&self, session_id: SessionId) -> Blocks<'_> {
        Blocks {
            lines: self.lines(session_id),
            line_index: 0,
            block_inlays: self.sessions[&session_id].block_inlays.iter(),
        }
    }

    pub fn fold_line(&mut self, session_id: SessionId, line_index: usize, column_index: usize) {
        let session = self.sessions.get_mut(&session_id).unwrap();
        let scale = if let Some(unfolding) = session.unfolding.remove(&line_index) {
            unfolding.scale
        } else if !session.folded.contains(&line_index)
            && !session.folding.contains_key(&line_index)
        {
            1.0
        } else {
            return;
        };
        session.folding.insert(
            line_index,
            Fold {
                column_index,
                scale,
            },
        );
    }

    pub fn unfold_line(&mut self, session_id: SessionId, line_index: usize, column_index: usize) {
        let session = self.sessions.get_mut(&session_id).unwrap();
        let scale = if let Some(folding) = session.folding.remove(&line_index) {
            folding.scale
        } else if session.folded.remove(&line_index) {
            0.0
        } else {
            return;
        };
        session.unfolding.insert(
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
        if session.folding.is_empty() && session.unfolding.is_empty() {
            return false;
        }
        for (line_index, fold) in &session.folding {
            let mut fold = *fold;
            fold.scale *= 0.9;
            if fold.scale < 0.001 {
                session.folded.insert(*line_index);
            } else {
                session.new_folding.insert(*line_index, fold);
            }
        }
        mem::swap(&mut session.folding, &mut session.new_folding);
        session.new_folding.clear();
        for (line_index, fold) in &session.unfolding {
            let mut fold = *fold;
            fold.scale = 1.0 - 0.9 * (1.0 - fold.scale);
            if 1.0 - fold.scale > 0.001 {
                session.new_unfolding.insert(*line_index, fold);
            }
        }
        mem::swap(&mut session.unfolding, &mut session.new_unfolding);
        session.new_unfolding.clear();
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
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(usize);

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

#[derive(Clone, Copy, Debug)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    line_index: usize,
    text: Iter<'a, String>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    inline_inlays: Iter<'a, Vec<(usize, String)>>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Fold>,
    unfolding: &'a HashMap<usize, Fold>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Line {
            line_index: {
                let line_index = self.line_index;
                self.line_index += 1;
                line_index
            },
            text: self.text.next()?,
            token_infos: self.token_infos.next()?,
            inline_inlays: self.inline_inlays.next()?,
            folded: self.folded,
            folding: self.folding,
            unfolding: self.unfolding,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    line_index: usize,
    text: &'a str,
    token_infos: &'a [TokenInfo],
    inline_inlays: &'a Vec<(usize, String)>,
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Fold>,
    unfolding: &'a HashMap<usize, Fold>,
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
            token,
            tokens,
            inline_inlays: self.inline_inlays.iter(),
        }
    }

    pub fn fold_state(&self) -> FoldState {
        if self.folded.contains(&self.line_index) {
            return FoldState::Folded;
        }
        if let Some(folding) = self.folding.get(&self.line_index) {
            return FoldState::Folding(*folding);
        }
        if let Some(unfolding) = self.unfolding.get(&self.line_index) {
            return FoldState::Unfolding(*unfolding);
        }
        FoldState::Unfolded
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoldState {
    Folded,
    Folding(Fold),
    Unfolding(Fold),
    Unfolded,
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
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inline_inlays: Iter<'a, (usize, String)>,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((byte_index, _)) = self.inline_inlays.as_slice().first() {
            if *byte_index == self.byte_index {
                let (_, inlay) = self.inline_inlays.next().unwrap();
                return Some(Inline::Inlay(inlay));
            }
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_index, _)) = self.inline_inlays.as_slice().first() {
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
        self.byte_index += byte_count;
        Some(Inline::Token(token))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token(Token<'a>),
    Inlay(&'a str),
}

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    lines: Lines<'a>,
    line_index: usize,
    block_inlays: Iter<'a, (usize, String)>,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((line_index, _)) = self.block_inlays.as_slice().first() {
            if *line_index == self.line_index {
                let (_, inlay) = self.block_inlays.next().unwrap();
                return Some(Block::Inlay(inlay));
            }
        }
        let line = self.lines.next()?;
        self.line_index += 1;
        Some(Block::Line(line))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Line(Line<'a>),
    Inlay(&'a str),
}

#[derive(Debug)]
struct Session {
    document_id: DocumentId,
    inline_inlays: Vec<Vec<(usize, String)>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, Fold>,
    new_folding: HashMap<usize, Fold>,
    unfolding: HashMap<usize, Fold>,
    new_unfolding: HashMap<usize, Fold>,
    block_inlays: Vec<(usize, String)>,
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

    text.split_whitespace_boundaries().map(|text| {
        TokenInfo {
            byte_count: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            }
        }
    }).collect()
}
