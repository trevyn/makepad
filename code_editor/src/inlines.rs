use {
    crate::{inlays::InlineInlay, tokens::Token, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, InlineInlay)>,
    wraps: Iter<'a, usize>,
    token: Option<Token<'a>>,
    inlay_tokens: Option<Tokens<'a>>,
    byte_idx: usize,
    inlay_byte_idx: usize,
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_offset) = self.wraps.as_slice().first() {
            if *inlay_offset == self.inlay_byte_idx {
                self.wraps.next().unwrap();
                return Some(Inline::Wrap);
            }
        }
        if let Some((offset, _)) = self.inlays.as_slice().first() {
            if *offset == self.byte_idx {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_idx += token.text.len();
                return Some(Inline::Token(true, token));
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut len = token.text.len();
        if let Some((offset, _)) = self.inlays.as_slice().first() {
            len = len.min(offset - self.byte_idx);
        }
        let token = if len < token.text.len() {
            let (text_0, text_1) = token.text.split_at(len);
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
        self.byte_idx += token.text.len();
        self.inlay_byte_idx += token.text.len();
        Some(Inline::Token(false, token))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Inline<'a> {
    Token(bool, Token<'a>),
    Wrap,
}

pub fn inlines<'a>(
    mut tokens: Tokens<'a>,
    inlays: &'a [(usize, InlineInlay)],
    wraps: &'a [usize],
) -> Inlines<'a> {
    let token = tokens.next();
    Inlines {
        tokens,
        inlays: inlays.iter(),
        wraps: wraps.iter(),
        token,
        inlay_tokens: None,
        byte_idx: 0,
        inlay_byte_idx: 0,
    }
}
