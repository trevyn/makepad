use {
    super::{tokens::Token, Inlay, Tokens},
    std::slice::Iter,
};

#[derive(Clone, Debug)]
pub struct Inlines<'a> {
    byte_offset: usize,
    inlay_byte_offset: usize,
    inlay_tokens: Option<Tokens<'a>>,
    token: Option<Token<'a>>,
    tokens: Tokens<'a>,
    inlays: Iter<'a, (usize, Inlay)>,
    breaks: Iter<'a, usize>,
}

impl<'a> Inlines<'a> {
    pub(super) fn new(
        mut tokens: Tokens<'a>,
        inlays: Iter<'a, (usize, Inlay)>,
        breaks: Iter<'a, usize>,
    ) -> Self {
        Self {
            byte_offset: 0,
            inlay_byte_offset: 0,
            inlay_tokens: None,
            token: tokens.next(),
            tokens,
            inlays,
            breaks,
        }
    }
}

impl<'a> Iterator for Inlines<'a> {
    type Item = Inline<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inlay_byte_offset) = self.breaks.as_slice().first() {
            if *inlay_byte_offset == self.inlay_byte_offset {
                self.breaks.next().unwrap();
                return Some(Inline::Break);
            }
        }
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            if *byte_offset == self.byte_offset {
                let (_, inlay) = self.inlays.next().unwrap();
                self.inlay_tokens = Some(inlay.tokens());
            }
        }
        if let Some(tokens) = &mut self.inlay_tokens {
            if let Some(token) = tokens.next() {
                self.inlay_byte_offset += token.text.len();
                return Some(Inline::Token { inlay: true, token });
            }
            self.inlay_tokens = None;
        }
        let token = self.token?;
        let mut byte_count = token.text.len();
        if let Some((byte_offset, _)) = self.inlays.as_slice().first() {
            byte_count = byte_count.min(byte_offset - self.byte_offset);
        }
        let token = if byte_count < token.text.len() {
            let (text_0, text_1) = token.text.split_at(byte_count);
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
        self.byte_offset += token.text.len();
        self.inlay_byte_offset += token.text.len();
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
