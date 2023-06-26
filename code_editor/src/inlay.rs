use crate::{
    token::{TokenInfo, Tokens},
    Fold, Line,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
    pub breaks: Vec<usize>,
}

impl BlockInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::token;

        let text = text.into();
        let token_infos = token::tokenize(&text);
        Self {
            text,
            token_infos,
            breaks: Vec::new(),
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        Line::new(
            &self.text,
            &self.token_infos,
            &[],
            &self.breaks,
            Fold::default(),
            (self.breaks.len() + 1) as f64,
        )
    }

    pub fn wrap(&mut self, wrap_column_index: Option<usize>) {
        use crate::wrap;

        self.breaks = Vec::new();
        self.breaks = if let Some(wrap_column_index) = wrap_column_index {
            wrap::wrap(self.as_line(), wrap_column_index)
        } else {
            Vec::new()
        };
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InlineInlay {
    pub text: String,
    pub token_infos: Vec<TokenInfo>,
}

impl InlineInlay {
    pub fn new(text: impl Into<String>) -> Self {
        use crate::token;

        let text = text.into();
        let token_infos = token::tokenize(&text);
        Self { text, token_infos }
    }

    pub fn tokens(&self) -> Tokens<'_> {
        use crate::token;

        token::tokens(&self.text, &self.token_infos)
    }
}
