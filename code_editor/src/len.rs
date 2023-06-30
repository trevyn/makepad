use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Len {
    pub line_count: usize,
    pub byte_count: usize,
}

impl Add for Len {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        if other.line_count == 0 {
            Self {
                line_count: self.line_count,
                byte_count: self.byte_count + other.byte_count,
            }
        } else {
            Self {
                line_count: self.line_count + other.line_count,
                byte_count: other.byte_count,
            }
        }
    }
}

impl AddAssign for Len {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for Len {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        if other.line_count == 0 {
            Self {
                line_count: self.line_count,
                byte_count: self.byte_count - other.byte_count,
            }
        } else {
            Self {
                line_count: self.line_count - other.line_count,
                byte_count: self.byte_count,
            }
        }
    }
}

impl SubAssign for Len {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}
