use crate::lines::Line;

pub fn wrap(line: Line<'_>, wrap_column_index: usize) -> Vec<usize> {
    use crate::{inlines::Inline, StrExt};

    let mut breaks = Vec::new();
    let mut inlay_byte_offset = 0;
    let mut column_index = 0;
    for inline in line.inlines() {
        match inline {
            Inline::Token { token, .. } => {
                let column_count: usize = token.text.column_count();
                if column_index + column_count > wrap_column_index {
                    breaks.push(inlay_byte_offset);
                    column_index = 0;
                }
                inlay_byte_offset += token.text.len();
                column_index += column_count;
            }
            _ => panic!(),
        }
    }
    breaks
}
