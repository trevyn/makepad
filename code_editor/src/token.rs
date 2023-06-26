use std::slice::Iter;

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub text: &'a str,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenInfo {
    pub len: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Whitespace,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    token_infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.token_infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.len);
        self.text = remaining_text;
        Some(Token {
            text,
            kind: info.kind,
        })
    }
}

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::StrExt;

    text.split_whitespace_boundaries()
        .map(|text| TokenInfo {
            len: text.len(),
            kind: if text.chars().next().unwrap().is_whitespace() {
                TokenKind::Whitespace
            } else {
                TokenKind::Unknown
            },
        })
        .collect()
}

pub fn tokens<'a>(text: &'a str, token_infos: &'a [TokenInfo]) -> Tokens<'a> {
    Tokens {
        text,
        token_infos: token_infos.iter(),
    }
}
