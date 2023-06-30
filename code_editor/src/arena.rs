use std::{
    fmt,
    hash::{Hash, Hasher},
    iter::Enumerate,
    marker::PhantomData,
    ops::{Index, IndexMut},
    slice,
};

#[derive(Clone, Debug)]
pub struct Arena<T> {
    len: usize,
    entries: Vec<Entry<T>>,
    generation: usize,
    first_vacant_idx: Option<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        match self.entries.get(id.idx) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.idx) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.entries.iter().enumerate(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.entries.iter_mut().enumerate(),
        }
    }

    pub fn insert(&mut self, value: T) -> Id<T> {
        let entry = Entry::Occupied {
            generation: self.generation,
            value,
        };
        let idx = if let Some(idx) = self.first_vacant_idx {
            match self.entries[idx] {
                Entry::Vacant { next_vacant_idx } => {
                    self.first_vacant_idx = next_vacant_idx;
                    self.entries[idx] = entry;
                    idx
                }
                _ => unreachable!(),
            }
        } else {
            let idx = self.entries.len();
            self.entries.push(entry);
            idx
        };
        Id::new(self.generation, idx)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.idx) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.idx],
                    Entry::Vacant {
                        next_vacant_idx: self.first_vacant_idx,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_idx = Some(id.idx);
                        Some(value)
                    }
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.generation += 1;
        self.first_vacant_idx = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_idx: None,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T> {
    iter: Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (idx, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(idx, *generation), value));
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Id<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (idx, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(idx, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    idx: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(idx: usize, generation: usize) -> Self {
        Self {
            idx,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("idx", &self.idx)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.idx.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.idx != other.idx {
            return false;
        }
        if self.generation != other.generation {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied { generation: usize, value: T },
    Vacant { next_vacant_idx: Option<usize> },
}
