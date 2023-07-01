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
    first_vacant_index: Option<usize>,
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
        match self.entries.get(id.index) {
            Some(Entry::Occupied { generation, value }) if *generation == id.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        match self.entries.get_mut(id.index) {
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
        let index = if let Some(index) = self.first_vacant_index {
            match self.entries[index] {
                Entry::Vacant { next_vacant_index } => {
                    self.first_vacant_index = next_vacant_index;
                    self.entries[index] = entry;
                    index
                }
                _ => unreachable!(),
            }
        } else {
            let index = self.entries.len();
            self.entries.push(entry);
            index
        };
        Id::new(self.generation, index)
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        use std::mem;

        match self.entries.get_mut(id.index) {
            Some(Entry::Occupied { generation, .. }) if *generation == id.generation => {
                match mem::replace(
                    &mut self.entries[id.index],
                    Entry::Vacant {
                        next_vacant_index: self.first_vacant_index,
                    },
                ) {
                    Entry::Occupied { generation, value } => {
                        if generation == self.generation {
                            self.generation += 1;
                        }
                        self.first_vacant_index = Some(id.index);
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
        self.first_vacant_index = None;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            len: 0,
            entries: Vec::new(),
            generation: 0,
            first_vacant_index: None,
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
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
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
            let (index, entry) = self.iter.next()?;
            if let Entry::Occupied { generation, value } = entry {
                break Some((Id::new(index, *generation), value));
            }
        }
    }
}

pub struct Id<T> {
    index: usize,
    generation: usize,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {
            index,
            generation,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
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
        self.index.hash(hasher);
        self.generation.hash(hasher);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.index != other.index {
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
    Vacant { next_vacant_index: Option<usize> },
}
