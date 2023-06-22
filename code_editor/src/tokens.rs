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
