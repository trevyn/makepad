use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    slice::Iter,
};

#[derive(Debug, Default)]
pub struct State {
    session_id: usize,
    sessions: HashMap<SessionId, Session>,
    document_id: usize,
    documents: HashMap<DocumentId, Document>,
    document_ids: HashMap<PathBuf, DocumentId>,
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
                block_inlays: vec![
                    (10, "XXX".to_string()),
                    (20, "XXX".to_string()),
                    (30, "XXX".to_string()),
                    (40, "XXX".to_string()),
                ],
                inline_inlays: (0..self.documents[&document_id].text.len())
                    .map(|_| vec![
                        (20, "X".to_string()),
                        (40, "X".to_string()),
                        (60, "X".to_string()),
                        (80, "X".to_string()),
                    ])
                    .collect(),
                token_infos: self.documents[&document_id]
                    .text
                    .iter()
                    .map(|text| {
                        vec![TokenInfo {
                            byte_count: text.len(),
                            kind: TokenKind::Unknown,
                        }]
                    })
                    .collect(),
                document_id,
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

    pub fn blocks(&self, session_id: SessionId) -> Blocks<'_> {
        Blocks {
            block_inlays: self.sessions[&session_id].block_inlays.iter(),
            lines: self.lines(session_id),
            line_index: 0,
        }
    }

    pub fn lines(&self, session_id: SessionId) -> Lines<'_> {
        let session = &self.sessions[&session_id];
        Lines {
            inline_inlays: session.inline_inlays.iter(),
            token_infos: session.token_infos.iter(),
            text: self.documents[&session.document_id].text.iter(),
        }
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
        self.documents.insert(
            document_id,
            Document {
                session_ids: HashSet::new(),
                path: path.map(|path| path.into()),
                text,
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

#[derive(Clone, Copy, Debug)]
pub struct TokenInfo {
    pub byte_count: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug)]
pub enum TokenKind {
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    block_inlays: Iter<'a, (usize, String)>,
    lines: Lines<'a>,
    line_index: usize,
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
    Inlay(&'a str),
    Line(Line<'a>),
}

impl<'a> Block<'a> {
    pub fn row_count(&self) -> usize {
        match &self {
            Self::Inlay(_) => 1,
            Self::Line(line) => line.row_count(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    inline_inlays: Iter<'a, Vec<(usize, String)>>,
    token_infos: Iter<'a, Vec<TokenInfo>>,
    text: Iter<'a, String>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Line {
            inline_inlays: self.inline_inlays.next()?,
            token_infos: self.token_infos.next()?,
            text: self.text.next()?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    inline_inlays: &'a [(usize, String)],
    token_infos: &'a [TokenInfo],
    text: &'a str,
}

impl<'a> Line<'a> {
    pub fn row_count(&self) -> usize {
        1
    }

    pub fn inlines(&self) -> Inlines<'a> {
        let mut tokens = self.tokens();
        let token = tokens.next();
        Inlines {
            inline_inlays: self.inline_inlays.iter(),
            tokens,
            token,
            byte_index: 0,
        }
    }

    pub fn tokens(&self) -> Tokens<'a> {
        Tokens {
            token_infos: self.token_infos.iter(),
            text: self.text,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    inline_inlays: Iter<'a, (usize, String)>,
    tokens: Tokens<'a>,
    token: Option<Token<'a>>,
    byte_index: usize,
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
    Inlay(&'a str),
    Token(Token<'a>),
}

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    token_infos: Iter<'a, TokenInfo>,
    text: &'a str,
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

#[derive(Debug)]
struct Session {
    block_inlays: Vec<(usize, String)>,
    inline_inlays: Vec<Vec<(usize, String)>>,
    token_infos: Vec<Vec<TokenInfo>>,
    document_id: DocumentId,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct DocumentId(usize);

#[derive(Debug)]
struct Document {
    session_ids: HashSet<SessionId>,
    path: Option<PathBuf>,
    text: Vec<String>,
}
