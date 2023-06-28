use {
    crate::tokenize::{TokenInfo, TokenKind},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Tokens<'a> {
    text: &'a str,
    token_infos: Iter<'a, TokenInfo>,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let info = self.token_infos.next()?;
        let (text, remaining_text) = self.text.split_at(info.byte_len);
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

pub fn tokens<'a>(text: &'a str, token_infos: &'a [TokenInfo]) -> Tokens<'a> {
    Tokens {
        text,
        token_infos: token_infos.iter(),
    }
}
