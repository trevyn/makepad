use {crate::Length, std::ops::{Add, AddAssign, Sub}};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    pub line_index: usize,
    pub byte_index: usize,
}

impl Add<Length> for Point {
    type Output = Self;

    fn add(self, length: Length) -> Self::Output {
        if length.line_count == 0 {
            Self {
                line_index: self.line_index,
                byte_index: self.byte_index + length.byte_count,
            }
        } else {
            Self {
                line_index: self.line_index + length.line_count,
                byte_index: length.byte_count,
            }
        }
    }
}

impl AddAssign<Length> for Point {
    fn add_assign(&mut self, length: Length) {
        *self = *self + length;
    }
}

impl Sub for Point {
    type Output = Length;

    fn sub(self, other: Self) -> Self::Output {
        if self.line_index == other.line_index {
            Length {
                line_count: 0,
                byte_count: self.byte_index - other.byte_index,
            }
        } else {
            Length {
                line_count: self.line_index - other.line_index,
                byte_count: self.byte_index
            }
        }
    }
}