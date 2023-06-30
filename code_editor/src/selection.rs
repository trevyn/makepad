use {
    crate::{
        pos::{Affinity, PosWithAffinity},
        Pos,
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
        let mut idx = match self
            .earlier_regions
            .binary_search_by_key(&self.latest_region.start(), |region| region.start())
        {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        while idx > 0 {
            let prev_idx = idx - 1;
            if let Some(merged_region) = self
                .latest_region
                .try_merge_with(self.earlier_regions[prev_idx])
            {
                self.latest_region = merged_region;
                self.earlier_regions.remove(prev_idx);
                idx = prev_idx;
            } else {
                break;
            }
        }
        while idx < self.earlier_regions.len() {
            if let Some(merged_region) =
                self.latest_region.try_merge_with(self.earlier_regions[idx])
            {
                self.latest_region = merged_region;
                self.earlier_regions.remove(idx);
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
        let mut idx = 0;
        while idx + 1 < self.earlier_regions.len() {
            if let Some(merged_region) =
                self.earlier_regions[idx].try_merge_with(self.earlier_regions[idx + 1])
            {
                self.earlier_regions[idx] = merged_region;
                self.earlier_regions.remove(idx + 1);
            } else {
                idx += 1;
            }
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            latest_region: Region {
                anchor: PosWithAffinity {
                    pos: Pos {
                        line_idx: 6,
                        byte_idx: 20,
                    },
                    affinity: Affinity::Before,
                },
                cursor: Cursor {
                    pos: PosWithAffinity {
                        pos: Pos {
                            line_idx: 11,
                            byte_idx: 20,
                        },
                        affinity: Affinity::After,
                    },
                    col_idx: None,
                },
            },
            earlier_regions: vec![Region {
                anchor: PosWithAffinity {
                    pos: Pos {
                        line_idx: 11,
                        byte_idx: 40,
                    },
                    affinity: Affinity::Before,
                },
                cursor: Cursor {
                    pos: PosWithAffinity {
                        pos: Pos {
                            line_idx: 17,
                            byte_idx: 10,
                        },
                        affinity: Affinity::After,
                    },
                    col_idx: None,
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
    pub anchor: PosWithAffinity,
    pub cursor: Cursor,
}

impl Region {
    pub fn is_empty(self) -> bool {
        self.anchor.pos == self.cursor.pos.pos
    }

    pub fn start(self) -> PosWithAffinity {
        self.anchor.min(self.cursor.pos)
    }

    pub fn end(self) -> PosWithAffinity {
        self.anchor.max(self.cursor.pos)
    }

    pub fn try_merge_with(self, other: Self) -> Option<Self> {
        use std::{cmp, cmp::Ordering, mem};

        let mut first = self;
        let mut second = other;
        if first.start() > second.start() {
            mem::swap(&mut first, &mut second);
        }
        match (
            first.anchor.pos == first.cursor.pos.pos,
            second.anchor.pos == second.cursor.pos.pos,
        ) {
            (true, true) if first.cursor.pos.pos == second.cursor.pos.pos => Some(self),
            (false, true) if first.end().pos >= second.cursor.pos.pos => Some(first),
            (true, false) if first.cursor.pos.pos == second.start().pos => Some(second),
            (false, false) if first.end().pos > second.start().pos => {
                Some(match self.anchor.pos.cmp(&self.cursor.pos.pos) {
                    Ordering::Less => Self {
                        anchor: self.anchor.min(other.anchor),
                        cursor: cmp::max_by_key(self.cursor, other.cursor, |cursor| cursor.pos),
                    },
                    Ordering::Greater => Self {
                        anchor: self.anchor.max(other.anchor),
                        cursor: cmp::min_by_key(self.cursor, other.cursor, |cursor| cursor.pos),
                    },
                    Ordering::Equal => unreachable!(),
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Cursor {
    pub pos: PosWithAffinity,
    pub col_idx: Option<usize>,
}
