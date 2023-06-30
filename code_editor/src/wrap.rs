use crate::Line;

pub fn wrap(line: Line<'_>, max_col_count: usize) -> Vec<usize> {
    use crate::{inlines::Inline, str::StrExt};

    let mut wraps = Vec::new();
    let mut inlay_byte_idx = 0;
    let mut col_idx = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token(_, token) => {
                let col_count: usize = token.text.col_count();
                if col_idx + col_count > max_col_count {
                    wraps.push(inlay_byte_idx);
                    col_idx = 0;
                }
                inlay_byte_idx += token.text.len();
                col_idx += col_count;
            }
            _ => panic!(),
        }
    }
    wraps
}
