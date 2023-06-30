pub trait StrExt {
    fn col_count(&self) -> usize;
    fn is_grapheme_boundary(&self, idx: usize) -> bool;
    fn next_grapheme_boundary(&self, idx: usize) -> Option<usize>;
    fn prev_grapheme_boundary(&self, idx: usize) -> Option<usize>;
    fn graphemes(&self) -> Graphemes<'_>;
    fn grapheme_indices(&self) -> GraphemeIndices<'_>;
    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_>;
}

impl StrExt for str {
    fn col_count(&self) -> usize {
        use crate::char::CharExt;

        self.chars().map(|char| char.col_count()).sum()
    }

    fn is_grapheme_boundary(&self, idx: usize) -> bool {
        self.is_char_boundary(idx)
    }

    fn next_grapheme_boundary(&self, idx: usize) -> Option<usize> {
        if idx == self.len() {
            return None;
        }
        let mut idx = idx + 1;
        while !self.is_grapheme_boundary(idx) {
            idx += 1;
        }
        Some(idx)
    }

    fn prev_grapheme_boundary(&self, idx: usize) -> Option<usize> {
        if idx == 0 {
            return None;
        }
        let mut idx = idx - 1;
        while !self.is_grapheme_boundary(idx) {
            idx -= 1;
        }
        Some(idx)
    }

    fn graphemes(&self) -> Graphemes<'_> {
        Graphemes { string: self }
    }

    fn grapheme_indices(&self) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            graphemes: self.graphemes(),
        }
    }

    fn split_whitespace_boundaries(&self) -> SplitWhitespaceBoundaries<'_> {
        SplitWhitespaceBoundaries { string: self }
    }
}

#[derive(Clone, Debug)]
pub struct Graphemes<'a> {
    string: &'a str,
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (grapheme, remaining_string) =
            self.string.split_at(self.string.next_grapheme_boundary(0)?);
        self.string = remaining_string;
        Some(grapheme)
    }
}

#[derive(Clone, Debug)]
pub struct GraphemeIndices<'a> {
    start: usize,
    graphemes: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = self.graphemes.next()?;
        Some((grapheme.as_ptr() as usize - self.start, grapheme))
    }
}

#[derive(Clone, Debug)]
pub struct SplitWhitespaceBoundaries<'a> {
    string: &'a str,
}

impl<'a> Iterator for SplitWhitespaceBoundaries<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.string.is_empty() {
            return None;
        }
        let mut prev_grapheme_is_whitespace = None;
        let idx = self
            .string
            .grapheme_indices()
            .find_map(|(idx, next_grapheme)| {
                let next_grapheme_is_whitespace =
                    next_grapheme.chars().all(|char| char.is_whitespace());
                let is_whitespace_boundary =
                    prev_grapheme_is_whitespace.map_or(false, |prev_grapheme_is_whitespace| {
                        prev_grapheme_is_whitespace != next_grapheme_is_whitespace
                    });
                prev_grapheme_is_whitespace = Some(next_grapheme_is_whitespace);
                if is_whitespace_boundary {
                    Some(idx)
                } else {
                    None
                }
            })
            .unwrap_or(self.string.len());
        let (string, remaining_string) = self.string.split_at(idx);
        self.string = remaining_string;
        Some(string)
    }
}
