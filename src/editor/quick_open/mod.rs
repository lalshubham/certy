mod scanner;

pub use scanner::{collect_workspace_files, fuzzy_score, QuickOpenFileItem};

use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct QuickOpenState {
    pub is_open: bool,
    pub focused: bool,
    pub query: String,
    pub cursor: usize,
    pub query_scroll: usize,
    pub selection_anchor: Option<usize>,
    pub all_files: Vec<QuickOpenFileItem>,
    pub matches: Vec<QuickOpenFileItem>,
    pub selected_match: usize,
    pub hovered_match: Option<usize>,
    pub hovered_close: bool,
}

impl QuickOpenState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, root: Option<&Path>) {
        let base_dir = root
            .map(|p| p.to_path_buf())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        self.all_files = collect_workspace_files(&base_dir, 10_000);
        self.is_open = true;
        self.focused = true;
        self.query.clear();
        self.cursor = 0;
        self.query_scroll = 0;
        self.selection_anchor = None;
        self.hovered_match = None;
        self.hovered_close = false;
        self.update_filter();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.focused = false;
        self.query.clear();
        self.cursor = 0;
        self.query_scroll = 0;
        self.selection_anchor = None;
        self.all_files.clear();
        self.matches.clear();
        self.selected_match = 0;
        self.hovered_match = None;
        self.hovered_close = false;
    }

    pub fn selected_file(&self) -> Option<PathBuf> {
        self.matches
            .get(self.selected_match)
            .map(|m| m.path.clone())
    }

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            let max_idx = self.matches.len().saturating_sub(1);
            self.selected_match = (self.selected_match + 1).min(max_idx);
        }
    }

    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.selected_match = self.selected_match.saturating_sub(1);
        }
    }

    pub fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.matches = self.all_files.iter().take(50).cloned().collect();
            self.selected_match = 0;
            return;
        }
        let q_lower: Vec<char> = self.query.chars().map(|c| c.to_ascii_lowercase()).collect();
        let mut scored: Vec<(QuickOpenFileItem, i32)> = self
            .all_files
            .iter()
            .filter_map(|item| {
                fuzzy_score(&q_lower, &item.relative_path).map(|score| (item.clone(), score))
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.relative_path.len().cmp(&b.0.relative_path.len()))
                .then_with(|| a.0.relative_path.cmp(&b.0.relative_path))
        });
        self.matches = scored.into_iter().take(50).map(|(item, _)| item).collect();
        self.selected_match = 0;
    }

    pub fn ensure_query_visible(&mut self, max_vis: usize) {
        if self.cursor < self.query_scroll {
            self.query_scroll = self.cursor;
        } else if self.cursor > self.query_scroll + max_vis {
            self.query_scroll = self.cursor.saturating_sub(max_vis);
        }
        let max_scroll = self.query.chars().count().saturating_sub(max_vis);
        self.query_scroll = self.query_scroll.min(max_scroll);
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some(anchor) = self.selection_anchor {
            let start = anchor.min(self.cursor);
            let end = anchor.max(self.cursor);
            if start != end {
                let mut chars: Vec<char> = self.query.chars().collect();
                chars.drain(start..end);
                self.query = chars.into_iter().collect();
                self.cursor = start;
                self.selection_anchor = None;
                self.update_filter();
                return true;
            }
            self.selection_anchor = None;
        }
        false
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.query.chars().count();
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor);
            let end = anchor.max(self.cursor);
            let chars: Vec<char> = self.query.chars().collect();
            chars[start..end].iter().collect()
        })
    }

    pub fn insert_char_at_cursor(&mut self, ch: char) {
        self.delete_selection();
        let mut chars: Vec<char> = self.query.chars().collect();
        let pos = self.cursor.min(chars.len());
        chars.insert(pos, ch);
        self.query = chars.into_iter().collect();
        self.cursor = pos + 1;
        self.update_filter();
    }

    pub fn insert_str_at_cursor(&mut self, text: &str) {
        let clean: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
        if clean.is_empty() {
            return;
        }
        self.delete_selection();
        let count = clean.chars().count();
        let mut chars: Vec<char> = self.query.chars().collect();
        let pos = self.cursor.min(chars.len());
        for (i, ch) in clean.chars().enumerate() {
            chars.insert(pos + i, ch);
        }
        self.query = chars.into_iter().collect();
        self.cursor = pos + count;
        self.update_filter();
    }

    pub fn delete_backwards(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            let mut chars: Vec<char> = self.query.chars().collect();
            if self.cursor <= chars.len() {
                chars.remove(self.cursor - 1);
                self.cursor -= 1;
                self.query = chars.into_iter().collect();
                self.update_filter();
            }
        }
    }

    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let mut chars: Vec<char> = self.query.chars().collect();
        if self.cursor < chars.len() {
            chars.remove(self.cursor);
            self.query = chars.into_iter().collect();
            self.update_filter();
        }
    }

    #[inline(always)]
    fn prepare_move(&mut self, selecting: bool) {
        if selecting {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
        } else {
            self.clear_selection();
        }
    }

    pub fn move_cursor_left(&mut self, sel: bool) {
        if !sel {
            if let Some(anchor) = self.selection_anchor {
                self.cursor = anchor.min(self.cursor);
                self.clear_selection();
                return;
            }
        }
        self.prepare_move(sel);
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self, sel: bool) {
        if !sel {
            if let Some(anchor) = self.selection_anchor {
                self.cursor = anchor.max(self.cursor);
                self.clear_selection();
                return;
            }
        }
        self.prepare_move(sel);
        let len = self.query.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn move_cursor_home(&mut self, sel: bool) {
        self.prepare_move(sel);
        self.cursor = 0;
    }

    pub fn move_cursor_end(&mut self, sel: bool) {
        self.prepare_move(sel);
        self.cursor = self.query.chars().count();
    }

    pub fn move_cursor_up(&mut self, sel: bool) {
        if sel {
            self.move_cursor_home(true);
        } else {
            self.prev_match();
        }
    }

    pub fn move_cursor_down(&mut self, sel: bool) {
        if sel {
            self.move_cursor_end(true);
        } else {
            self.next_match();
        }
    }
}
