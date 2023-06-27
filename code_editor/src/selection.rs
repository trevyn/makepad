use {
    crate::{
        position::{Affinity, PositionWithAffinity},
        Position, Range,
    },
    std::slice,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Selection {
    regions: Vec<Region>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.regions.iter(),
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            regions: vec![Region {
                anchor: PositionWithAffinity {
                    position: Position {
                        line_index: 6,
                        byte_index: 10,
                    },
                    affinity: Affinity::Before,
                },
                cursor: PositionWithAffinity {
                    position: Position {
                        line_index: 12,
                        byte_index: 20,
                    },
                    affinity: Affinity::After,
                },
            }],
        }
    }
}

impl<'a> IntoIterator for &'a Selection {
    type Item = Region;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    iter: slice::Iter<'a, Region>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Region;

    fn next(&mut self) -> Option<Self::Item> {
        Some(*self.iter.next()?)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Region {
    pub anchor: PositionWithAffinity,
    pub cursor: PositionWithAffinity,
}

impl Region {
    pub fn is_empty(self) -> bool {
        self.anchor == self.cursor
    }

    pub fn start(self) -> PositionWithAffinity {
        self.anchor.min(self.cursor)
    }

    pub fn end(self) -> PositionWithAffinity {
        self.anchor.max(self.cursor)
    }

    pub fn range(self) -> Range<PositionWithAffinity> {
        Range {
            start: self.start(),
            end: self.end(),
        }
    }
}
