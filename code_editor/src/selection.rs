use crate::text;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Region {
    pub anchor: Point,
    pub cursor: Point,
}

impl Region {
    pub fn is_empty(self) -> bool {
        self.anchor == self.cursor
    }

    pub fn start(self) -> Point {
        self.anchor.min(self.cursor)
    }

    pub fn end(self) -> Point {
        self.anchor.max(self.cursor)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Point {
    pub point: text::Point,
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
