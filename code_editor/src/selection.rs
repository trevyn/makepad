use {
    crate::{
        position::{Bias, BiasedPosition},
        Position,
    },
    std::slice,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Selection {
    latest_region: Region,
    earlier_regions: Vec<Region>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            latest_region: Some(&self.latest_region),
            earlier_regions: self.earlier_regions.iter(),
        }
    }

    pub fn modify_latest_region(&mut self, mut f: impl FnMut(Region) -> Region) {
        self.latest_region = f(self.latest_region);
        self.normalize_latest_region();
    }

    pub fn modify_all_regions(&mut self, mut f: impl FnMut(Region) -> Region) {
        for region in &mut self.earlier_regions {
            *region = f(*region);
        }
        self.normalize_earlier_regions();
        self.modify_latest_region(f);
    }

    fn normalize_latest_region(&mut self) {
        let mut index = match self
            .earlier_regions
            .binary_search_by_key(&self.latest_region.start(), |region| region.start())
        {
            Ok(index) => index,
            Err(index) => index,
        };
        while index > 0 {
            let prev_index = index - 1;
            if let Some(merged_region) = self
                .latest_region
                .try_merge_with(self.earlier_regions[prev_index])
            {
                self.latest_region = merged_region;
                self.earlier_regions.remove(prev_index);
                index = prev_index;
            } else {
                break;
            }
        }
        while index < self.earlier_regions.len() {
            if let Some(merged_region) = self
                .latest_region
                .try_merge_with(self.earlier_regions[index])
            {
                self.latest_region = merged_region;
                self.earlier_regions.remove(index);
            } else {
                break;
            }
        }
    }

    fn normalize_earlier_regions(&mut self) {
        if self.earlier_regions.is_empty() {
            return;
        }
        self.earlier_regions.sort_by_key(|region| region.start());
        let mut index = 0;
        while index + 1 < self.earlier_regions.len() {
            if let Some(merged_region) =
                self.earlier_regions[index].try_merge_with(self.earlier_regions[index + 1])
            {
                self.earlier_regions[index] = merged_region;
                self.earlier_regions.remove(index + 1);
            } else {
                index += 1;
            }
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            latest_region: Region {
                anchor: BiasedPosition {
                    position: Position {
                        line_index: 6,
                        byte_index: 20,
                    },
                    bias: Bias::Before,
                },
                cursor: Cursor {
                    position: BiasedPosition {
                        position: Position {
                            line_index: 11,
                            byte_index: 20,
                        },
                        bias: Bias::After,
                    },
                    column_index: None,
                },
            },
            earlier_regions: vec![Region {
                anchor: BiasedPosition {
                    position: Position {
                        line_index: 11,
                        byte_index: 40,
                    },
                    bias: Bias::Before,
                },
                cursor: Cursor {
                    position: BiasedPosition {
                        position: Position {
                            line_index: 17,
                            byte_index: 10,
                        },
                        bias: Bias::After,
                    },
                    column_index: None,
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
    latest_region: Option<&'a Region>,
    earlier_regions: slice::Iter<'a, Region>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Region;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.latest_region, self.earlier_regions.as_slice().first()) {
            (Some(latest_region), Some(earlier_region)) => {
                if latest_region.start() <= earlier_region.start() {
                    self.latest_region.take()
                } else {
                    self.earlier_regions.next()
                }
            }
            (Some(_), _) => self.latest_region.take(),
            (_, Some(_)) => self.earlier_regions.next(),
            _ => None,
        }
        .copied()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Region {
    pub anchor: BiasedPosition,
    pub cursor: Cursor,
}

impl Region {
    pub fn is_empty(self) -> bool {
        self.anchor == self.cursor.position
    }

    pub fn start(self) -> BiasedPosition {
        self.anchor.min(self.cursor.position)
    }

    pub fn end(self) -> BiasedPosition {
        self.anchor.max(self.cursor.position)
    }

    pub fn try_merge_with(self, other: Self) -> Option<Self> {
        use std::{cmp, cmp::Ordering, mem};

        let mut first = self;
        let mut second = other;
        if first.start() > second.start() {
            mem::swap(&mut first, &mut second);
        }
        match (
            first.anchor.position == first.cursor.position.position,
            second.anchor.position == second.cursor.position.position,
        ) {
            (true, true) if first.cursor.position.position == second.cursor.position.position => {
                Some(self)
            }
            (false, true) if first.end().position >= second.cursor.position.position => Some(first),
            (true, false) if first.cursor.position.position == second.start().position => {
                Some(second)
            }
            (false, false) if first.end().position > second.start().position => Some(
                match self.anchor.position.cmp(&self.cursor.position.position) {
                    Ordering::Less => Self {
                        anchor: self.anchor.min(other.anchor),
                        cursor: cmp::max_by_key(self.cursor, other.cursor, |cursor| {
                            cursor.position
                        }),
                    },
                    Ordering::Greater => Self {
                        anchor: self.anchor.max(other.anchor),
                        cursor: cmp::min_by_key(self.cursor, other.cursor, |cursor| {
                            cursor.position
                        }),
                    },
                    Ordering::Equal => unreachable!(),
                },
            ),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Cursor {
    pub position: BiasedPosition,
    pub column_index: Option<usize>,
}
