pub trait CharExt {
    fn width(self) -> usize;
}

impl CharExt for char {
    fn width(self) -> usize {
        if self == '\t' {
            4
        } else {
            1
        }
    }
}
