use crate::{blocks::Block, inlines::Inline, tokens::Token, Line};

pub trait Visitor: Sized {
    fn visit_block(&mut self, block: Block<'_>) {
        walk_block(self, block);
    }

    fn visit_line(&mut self, _is_inlay: bool, line: Line<'_>) {
        walk_line(self, line);
    }

    fn visit_inline(&mut self, inline: Inline<'_>) {
        walk_inline(self, inline);
    }

    fn visit_token(&mut self, _is_inlay: bool, token: Token<'_>) {
        walk_token(self, token);
    }

    fn visit_grapheme(&mut self, _grapheme: &str) {}

    fn visit_wrap(&mut self) {}
}

pub fn walk_block(visitor: &mut impl Visitor, block: Block<'_>) {
    match block {
        Block::Line(is_inlay, line) => visitor.visit_line(is_inlay, line),
    }
}

pub fn walk_line(visitor: &mut impl Visitor, line: Line<'_>) {
    for inline in line.inlines() {
        visitor.visit_inline(inline);
    }
}

pub fn walk_inline(visitor: &mut impl Visitor, inline: Inline<'_>) {
    match inline {
        Inline::Token(is_inlay, token) => visitor.visit_token(is_inlay, token),
        Inline::Wrap => visitor.visit_wrap(),
    }
}

pub fn walk_token(visitor: &mut impl Visitor, token: Token<'_>) {
    use crate::str::StrExt;

    for grapheme in token.text.graphemes() {
        visitor.visit_grapheme(grapheme);
    }
}
