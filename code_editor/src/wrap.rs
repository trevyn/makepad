use crate::Line;

pub fn wrap(line: Line<'_>, max_column_count: usize) -> Vec<usize> {
    use crate::{inline::Inline, StrExt};

    let mut wraps = Vec::new();
    let mut inlay_byte_index = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > max_column_count {
                    wraps.push(inlay_byte_index);
                    column_index = 0;
                }
                inlay_byte_index += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    wraps
}
