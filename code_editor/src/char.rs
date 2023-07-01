pub trait CharExt {
    fn column_count(self) -> usize;
}

impl CharExt for char {
    fn column_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
