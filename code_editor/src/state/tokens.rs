use std::slice::Iter;

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    infos: Iter<'a, TokenInfo>,
}

impl<'a> Tokens<'a> {
    pub(super) fn new(text: &'a str, infos: Iter<'a, TokenInfo>) -> Self {
        Self { text, infos }
    }
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
