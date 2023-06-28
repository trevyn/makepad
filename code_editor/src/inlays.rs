use crate::{tokenize::TokenInfo, Fold, Line, Tokens};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    text: String,
    token_infos: Vec<TokenInfo>,
    wraps: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let token_infos = tokenize::tokenize(&text);
        Self {
            text,
            token_infos,
            wraps: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        crate::line(
            &self.text,
            &self.token_infos,
            &[],
            &self.wraps,
            Fold::default(),
            (self.wraps.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.wraps = Vec::new();
        self.wraps = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub tokens: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::tokenize;

        let text = text.into();
        let tokens = tokenize::tokenize(&text);
        Self { text, tokens }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        crate::tokens(&self.text, &self.tokens)
    }
}
