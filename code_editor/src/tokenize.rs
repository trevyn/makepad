use crate::tokens::TokenInfo;

pub fn tokenize(text: &str) -> Vec<TokenInfo> {
    use crate::{tokens::TokenKind, StrExt};

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
