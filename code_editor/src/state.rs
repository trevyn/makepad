use {
    crate::{
        arena::Id,
        blocks::Block,
        fold::Folding,
        inlays::{BlockInlay, InlineInlay},
        selection::Cursor,
        tokenize::TokenInfo,
        Arena, Blocks, Fold, Layout, Line, Lines, Selection,
    },
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        io,
        ops::Range,
        path::{Path, PathBuf},
    },
};

#[derive(Debug, Default)]
pub struct State {
    documents: Arena<Document>,
    sessions: Arena<Session>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<SessionId> {
        let document_id = self.open_document(path)?;
        let document = &self.documents[document_id];
        let session_id = SessionId(
            self.sessions.insert(Session {
                max_col_count: None,
                document_id,
                inline_inlays: (0..document.text.len())
                    .map(|_| {
                        vec![
                            (20, InlineInlay::new("X Y Z")),
                            (40, InlineInlay::new("X Y Z")),
                            (60, InlineInlay::new("X Y Z")),
                            (80, InlineInlay::new("X Y Z")),
                        ]
                    })
                    .collect(),
                breaks: document
                    .text
                    .iter()
                    .enumerate()
                    .map(|_| Vec::new())
                    .collect(),
                folded: HashSet::new(),
                folding: HashMap::new(),
                unfolding: HashMap::new(),
                heights: (0..document.text.len()).map(|_| 0.0).collect(),
                summed_heights: RefCell::new(Vec::new()),
                block_inlays: Vec::new(),
                selection: Selection::new(),
                tmp_folding: HashMap::new(),
            }),
        );
        self.documents[document_id].session_ids.insert(session_id.0);
        let mut view = self.view_mut(session_id);
        for idx in 0..19 {
            view.insert_block_inlay(idx * 10, BlockInlay::new("XXX YYY ZZZ"));
        }
        for line_idx in 0..view.line_count() {
            view.update_line_height(line_idx);
        }
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let document_id = self.sessions[session_id.0].document_id;
        let document = &mut self.documents[document_id];
        document.session_ids.remove(&session_id.0);
        if document.session_ids.is_empty() {
            self.close_document(document_id);
        }
        self.sessions.remove(session_id.0);
    }

    pub fn view(&self, session_id: SessionId) -> View<'_> {
        let session = &self.sessions[session_id.0];
        let document = &self.documents[session.document_id];
        View {
            max_col_count: session.max_col_count,
            text: &document.text,
            token_infos: &document.token_infos,
            inline_inlays: &session.inline_inlays,
            wraps: &session.breaks,
            folded: &session.folded,
            folding: &session.folding,
            unfolding: &session.unfolding,
            heights: &session.heights,
            summed_heights: &session.summed_heights,
            block_inlays: &session.block_inlays,
            selection: &session.selection,
        }
    }

    pub fn view_mut(&mut self, session_id: SessionId) -> ViewMut<'_> {
        let session = &mut self.sessions[session_id.0];
        let document = &mut self.documents[session.document_id];
        ViewMut {
            max_col_count: &mut session.max_col_count,
            text: &mut document.text,
            token_infos: &mut document.token_infos,
            inline_inlays: &mut session.inline_inlays,
            breaks: &mut session.breaks,
            folded: &mut session.folded,
            folding: &mut session.folding,
            unfolding: &mut session.unfolding,
            heights: &mut session.heights,
            summed_heights: &mut session.summed_heights,
            block_inlays: &mut session.block_inlays,
            selection: &mut session.selection,
            tmp_folding: &mut session.tmp_folding,
        }
    }

    fn open_document(
        &mut self,
        path: Option<impl AsRef<Path> + Into<PathBuf>>,
    ) -> io::Result<Id<Document>> {
        use {crate::tokenize, std::fs};

        let text = {
            let mut text: Vec<_> = String::from_utf8_lossy(
                &path
                    .as_ref()
                    .map_or_else(|| Ok(Vec::new()), |path| fs::read(path.as_ref()))?,
            )
            .lines()
            .map(|text| text.to_string())
            .collect();
            if text.is_empty() {
                text.push(String::new());
            }
            text
        };
        let token_infos = text.iter().map(|text| tokenize::tokenize(text)).collect();
        Ok(self.documents.insert(Document {
            session_ids: HashSet::new(),
            text,
            token_infos,
        }))
    }

    fn close_document(&mut self, document_id: Id<Document>) {
        self.documents.remove(document_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(Id<Session>);

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    max_col_count: Option<usize>,
    text: &'a [String],
    token_infos: &'a [Vec<TokenInfo>],
    inline_inlays: &'a [Vec<(usize, InlineInlay)>],
    wraps: &'a [Vec<usize>],
    folded: &'a HashSet<usize>,
    folding: &'a HashMap<usize, Folding>,
    unfolding: &'a HashMap<usize, Folding>,
    heights: &'a [f64],
    summed_heights: &'a RefCell<Vec<f64>>,
    block_inlays: &'a [(usize, BlockInlay)],
    selection: &'a Selection,
}

impl<'a> View<'a> {
    pub fn max_col_count(&self) -> Option<usize> {
        self.max_col_count
    }

    pub fn line_count(&self) -> usize {
        self.text.len()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self
            .summed_heights
            .borrow()
            .binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap())
        {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }

    pub fn find_first_line_starting_after_y(&self, y: f64) -> usize {
        self.update_summed_heights();
        match self
            .summed_heights
            .borrow()
            .binary_search_by(|summed_height| summed_height.partial_cmp(&y).unwrap())
        {
            Ok(idx) => idx + 1,
            Err(idx) => {
                if idx == self.line_count() {
                    idx
                } else {
                    idx + 1
                }
            }
        }
    }

    pub fn line(&self, line_idx: usize) -> Line<'a> {
        crate::line(
            &self.text[line_idx],
            &self.token_infos[line_idx],
            &self.inline_inlays[line_idx],
            &self.wraps[line_idx],
            Fold::new(&self.folded, &self.folding, &self.unfolding, line_idx),
            self.heights[line_idx],
        )
    }

    pub fn line_summed_height(&self, line_idx: usize) -> f64 {
        self.update_summed_heights();
        self.summed_heights.borrow()[line_idx]
    }

    pub fn lines(&self, line_idx_range: Range<usize>) -> Lines<'a> {
        crate::lines(
            self.text,
            self.token_infos,
            self.inline_inlays,
            self.wraps,
            &self.folded,
            &self.folding,
            &self.unfolding,
            self.heights,
            line_idx_range,
        )
    }

    pub fn blocks(&self, line_idx_range: Range<usize>) -> Blocks<'a> {
        crate::blocks(self.lines(line_idx_range), self.block_inlays)
    }

    pub fn layout(&self, line_idx_range: Range<usize>) -> Layout<'a> {
        crate::layout(self, line_idx_range)
    }

    pub fn selection(&self) -> &'a Selection {
        &self.selection
    }

    fn update_summed_heights(&self) {
        let summed_heights = self.summed_heights.borrow();
        let start_line_idx = summed_heights.len();
        let mut summed_height = if start_line_idx == 0 {
            0.0
        } else {
            summed_heights[start_line_idx - 1]
        };
        drop(summed_heights);
        for block in self.blocks(start_line_idx..self.line_count()) {
            match block {
                Block::Line(is_inlay, line) => {
                    summed_height += line.row_count() as f64 * line.fold().scale();
                    if !is_inlay {
                        self.summed_heights.borrow_mut().push(summed_height);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ViewMut<'a> {
    max_col_count: &'a mut Option<usize>,
    text: &'a mut [String],
    token_infos: &'a mut [Vec<TokenInfo>],
    inline_inlays: &'a mut [Vec<(usize, InlineInlay)>],
    breaks: &'a mut [Vec<usize>],
    folded: &'a mut HashSet<usize>,
    folding: &'a mut HashMap<usize, Folding>,
    unfolding: &'a mut HashMap<usize, Folding>,
    heights: &'a mut [f64],
    summed_heights: &'a mut RefCell<Vec<f64>>,
    block_inlays: &'a mut Vec<(usize, BlockInlay)>,
    selection: &'a mut Selection,
    tmp_folding: &'a mut HashMap<usize, Folding>,
}

impl<'a> ViewMut<'a> {
    pub fn as_view(&self) -> View<'_> {
        View {
            max_col_count: *self.max_col_count,
            text: &self.text,
            token_infos: &self.token_infos,
            inline_inlays: &self.inline_inlays,
            wraps: &self.breaks,
            folded: &self.folded,
            folding: &self.folding,
            unfolding: &self.unfolding,
            heights: &self.heights,
            summed_heights: &self.summed_heights,
            block_inlays: &self.block_inlays,
            selection: &self.selection,
        }
    }

    pub fn max_col_count(&self) -> Option<usize> {
        self.as_view().max_col_count()
    }

    pub fn line_count(&self) -> usize {
        self.as_view().line_count()
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_ending_after_y(y)
    }

    pub fn find_first_line_starting_after_y(&self, y: f64) -> usize {
        self.as_view().find_first_line_starting_after_y(y)
    }

    pub fn line(&self, line_idx: usize) -> Line<'_> {
        self.as_view().line(line_idx)
    }

    pub fn line_summed_height(&self, line_idx: usize) -> f64 {
        self.as_view().line_summed_height(line_idx)
    }

    pub fn lines(&self, line_idx_range: Range<usize>) -> Lines<'_> {
        self.as_view().lines(line_idx_range)
    }

    pub fn blocks(&self, line_idx_range: Range<usize>) -> Blocks<'_> {
        self.as_view().blocks(line_idx_range)
    }

    pub fn layout(&self, line_idx_range: Range<usize>) -> Layout<'_> {
        self.as_view().layout(line_idx_range)
    }

    pub fn selection(&self) -> &Selection {
        self.as_view().selection()
    }

    pub fn set_max_col_count(&mut self, max_col_count: Option<usize>) {
        if *self.max_col_count != max_col_count {
            *self.max_col_count = max_col_count;
            for line_idx in 0..self.line_count() {
                self.wrap_line(line_idx);
            }
            for &mut (line_idx, ref mut inlay) in self.block_inlays.iter_mut() {
                let old_height = inlay.as_line().height();
                inlay.wrap(max_col_count);
                let new_height = inlay.as_line().height();
                if old_height != new_height {
                    self.summed_heights.borrow_mut().truncate(line_idx);
                }
            }
        }
    }

    pub fn fold_line(&mut self, line_idx: usize, col_idx: usize) {
        let scale = if let Some(state) = self.unfolding.remove(&line_idx) {
            state.scale
        } else if !self.folded.contains(&line_idx) && !self.folding.contains_key(&line_idx) {
            1.0
        } else {
            return;
        };
        self.folding.insert(line_idx, Folding { col_idx, scale });
        self.update_line_height(line_idx);
    }

    pub fn unfold_line(&mut self, line_idx: usize, col_idx: usize) {
        let scale = if let Some(state) = self.folding.remove(&line_idx) {
            state.scale
        } else if self.folded.remove(&line_idx) {
            0.0
        } else {
            return;
        };
        self.unfolding.insert(line_idx, Folding { col_idx, scale });
        self.update_line_height(line_idx);
    }

    pub fn update_folds(&mut self) -> bool {
        use std::mem;

        if self.folding.is_empty() && self.unfolding.is_empty() {
            return false;
        }
        for (line_idx, state) in self.folding.iter() {
            let mut state = *state;
            state.scale *= 0.9;
            if state.scale < 0.001 {
                self.folded.insert(*line_idx);
            } else {
                self.tmp_folding.insert(*line_idx, state);
            }
        }
        mem::swap(self.folding, self.tmp_folding);
        self.tmp_folding.clear();
        for (line_idx, state) in self.unfolding.iter() {
            let mut state = *state;
            state.scale = 1.0 - 0.9 * (1.0 - state.scale);
            if 1.0 - state.scale > 0.001 {
                self.tmp_folding.insert(*line_idx, state);
            }
        }
        mem::swap(self.unfolding, self.tmp_folding);
        self.tmp_folding.clear();
        for line_idx in 0..self.line_count() {
            self.update_line_height(line_idx);
        }
        true
    }

    pub fn insert_block_inlay(&mut self, line_idx: usize, inlay: BlockInlay) {
        let idx = match self
            .block_inlays
            .binary_search_by_key(&line_idx, |&(line_idx, _)| line_idx)
        {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        self.block_inlays.insert(idx, (line_idx, inlay));
        self.summed_heights.borrow_mut().truncate(line_idx);
    }

    fn wrap_line(&mut self, line_idx: usize) {
        use crate::wrap;

        self.breaks[line_idx] = Vec::new();
        self.breaks[line_idx] = if let &mut Some(max_col_count) = self.max_col_count {
            wrap::wrap(self.line(line_idx), max_col_count)
        } else {
            Vec::new()
        };
        self.update_line_height(line_idx);
    }

    fn update_line_height(&mut self, line_idx: usize) {
        let old_height = self.heights[line_idx];
        let line = self.line(line_idx);
        let new_height = line.fold().scale() * line.row_count() as f64;
        self.heights[line_idx] = new_height;
        if old_height != new_height {
            self.summed_heights.borrow_mut().truncate(line_idx);
        }
    }

    fn _modify_selection(&mut self, select: bool, mut f: impl FnMut(Cursor) -> Cursor) {
        use {crate::selection::Region, std::mem};

        let mut selection = mem::take(self.selection);
        selection.modify_all_regions(|region| {
            let cursor = f(region.cursor);
            Region {
                anchor: if select { region.anchor } else { cursor.pos },
                cursor,
            }
        });
        *self.selection = selection;
    }
}

#[derive(Debug)]
struct Session {
    max_col_count: Option<usize>,
    document_id: Id<Document>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    breaks: Vec<Vec<usize>>,
    folded: HashSet<usize>,
    folding: HashMap<usize, Folding>,
    unfolding: HashMap<usize, Folding>,
    heights: Vec<f64>,
    summed_heights: RefCell<Vec<f64>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    selection: Selection,
    tmp_folding: HashMap<usize, Folding>,
}

#[derive(Debug)]
struct Document {
    session_ids: HashSet<Id<Session>>,
    text: Vec<String>,
    token_infos: Vec<Vec<TokenInfo>>,
}
