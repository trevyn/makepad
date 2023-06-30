pub trait CharExt {
    fn col_count(self) -> usize;
}

impl CharExt for char {
    fn col_count(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
