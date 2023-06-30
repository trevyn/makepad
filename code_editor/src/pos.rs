use {
    crate::Len,
    std::ops::{Add, AddAssign, Sub},
};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Pos {
    pub line_idx: usize,
    pub byte_idx: usize,
}

impl Add<Len> for Pos {
    type Output = Self;

    fn add(self, len: Len) -> Self::Output {
        if len.line_count == 0 {
            Self {
                line_idx: self.line_idx,
                byte_idx: self.byte_idx + len.byte_count,
            }
        } else {
            Self {
                line_idx: self.line_idx + len.line_count,
                byte_idx: len.byte_count,
            }
        }
    }
}

impl AddAssign<Len> for Pos {
    fn add_assign(&mut self, len: Len) {
        *self = *self + len;
    }
}

impl Sub for Pos {
    type Output = Len;

    fn sub(self, other: Self) -> Self::Output {
        if self.line_idx == other.line_idx {
            Len {
                line_count: 0,
                byte_count: self.byte_idx - other.byte_idx,
            }
        } else {
            Len {
                line_count: self.line_idx - other.line_idx,
                byte_count: self.byte_idx,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PosWithAffinity {
    pub pos: Pos,
    pub affinity: Affinity,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Affinity {
    Before,
    After,
}

impl Default for Affinity {
    fn default() -> Self {
        Self::Before
    }
}
