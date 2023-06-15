pub trait StrExt {
    fn column_count(&self) -> usize;
}

impl StrExt for str {
    fn column_count(&self) -> usize {
        use crate::CharExt;

        self.chars().map(|char| char.column_count()).sum()
    }
}
