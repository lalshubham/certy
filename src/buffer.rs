use crate::history::{EditAction, History};
use ropey::Rope;

pub struct EditorBuffer {
    text: Rope,
    pub cursor_char: usize,
    pub selection_anchor: Option<usize>,
    pub scroll_line: usize,
    pub scroll_col: usize,
    pub max_line_len: usize,
    history: History,
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            cursor_char: 0,
            selection_anchor: None,
            scroll_line: 0,
            scroll_col: 0,
            max_line_len: 0,
            history: History::new(),
        }
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        match self.selection_anchor {
            Some(anchor) if anchor != self.cursor_char => {
                Some((anchor.min(self.cursor_char), anchor.max(self.cursor_char)))
            }
            _ => None,
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection_range()
            .map(|(start, end)| self.text.slice(start..end).to_string())
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            let removed = self.text.slice(start..end).to_string();
            self.text.remove(start..end);
            self.cursor_char = start;
            self.selection_anchor = None;
            self.history.record(EditAction::Delete {
                char_idx: start,
                text: removed,
            });
            self.recompute_max_line_len();
            true
        } else {
            false
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.text.insert_char(self.cursor_char, ch);
        self.history.record(EditAction::Insert {
            char_idx: self.cursor_char,
            text: ch.to_string(),
        });
        self.cursor_char += 1;
        self.recompute_max_line_len();
    }

    pub fn insert_str(&mut self, text: &str) {
        self.delete_selection();
        self.text.insert(self.cursor_char, text);
        let char_count = text.chars().count();
        self.history.record(EditAction::Insert {
            char_idx: self.cursor_char,
            text: text.to_string(),
        });
        self.cursor_char += char_count;
        self.recompute_max_line_len();
    }

    pub fn delete_backwards(&mut self) {
        if !self.delete_selection() && self.cursor_char > 0 {
            self.cursor_char -= 1;
            let removed = self.text.char(self.cursor_char).to_string();
            self.text.remove(self.cursor_char..self.cursor_char + 1);
            self.history.record(EditAction::Delete {
                char_idx: self.cursor_char,
                text: removed,
            });
            self.recompute_max_line_len();
        }
    }

    pub fn delete_forward(&mut self) {
        if !self.delete_selection() && self.cursor_char < self.text.len_chars() {
            let removed = self.text.char(self.cursor_char).to_string();
            self.text.remove(self.cursor_char..self.cursor_char + 1);
            self.history.record(EditAction::Delete {
                char_idx: self.cursor_char,
                text: removed,
            });
            self.recompute_max_line_len();
        }
    }

    pub fn select_all(&mut self) {
        if self.text.len_chars() > 0 {
            self.selection_anchor = Some(0);
            self.cursor_char = self.text.len_chars();
        }
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.history.pop_undo() {
            match action {
                EditAction::Insert { char_idx, text } => {
                    let count = text.chars().count();
                    self.text.remove(char_idx..char_idx + count);
                    self.cursor_char = char_idx;
                    self.history
                        .push_redo(EditAction::Insert { char_idx, text });
                }
                EditAction::Delete { char_idx, text } => {
                    self.text.insert(char_idx, &text);
                    self.cursor_char = char_idx + text.chars().count();
                    self.history
                        .push_redo(EditAction::Delete { char_idx, text });
                }
            }
            self.selection_anchor = None;
            self.recompute_max_line_len();
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.history.pop_redo() {
            match action {
                EditAction::Insert { char_idx, text } => {
                    self.text.insert(char_idx, &text);
                    self.cursor_char = char_idx + text.chars().count();
                    self.history
                        .push_undo(EditAction::Insert { char_idx, text });
                }
                EditAction::Delete { char_idx, text } => {
                    let count = text.chars().count();
                    self.text.remove(char_idx..char_idx + count);
                    self.cursor_char = char_idx;
                    self.history
                        .push_undo(EditAction::Delete { char_idx, text });
                }
            }
            self.selection_anchor = None;
            self.recompute_max_line_len();
        }
    }

    pub fn move_left(&mut self) {
        self.selection_anchor = None;
        self.cursor_char = self.cursor_char.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.selection_anchor = None;
        if self.cursor_char < self.text.len_chars() {
            self.cursor_char += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selection_anchor = None;
        let (line, col) = self.cursor_pos();
        if line > 0 {
            let target = line - 1;
            let len = self.line_len(target);
            self.cursor_char = self.text.line_to_char(target) + col.min(len);
        }
    }

    pub fn move_down(&mut self) {
        self.selection_anchor = None;
        let (line, col) = self.cursor_pos();
        if line + 1 < self.text.len_lines() {
            let target = line + 1;
            let len = self.line_len(target);
            self.cursor_char = self.text.line_to_char(target) + col.min(len);
        }
    }

    pub fn set_cursor_at(&mut self, target_line: usize, target_col: usize) {
        let total_lines = self.text.len_lines();
        if total_lines == 0 {
            self.cursor_char = 0;
            return;
        }
        let line = target_line.min(total_lines - 1);
        let col = target_col.min(self.line_len(line));
        self.cursor_char = self.text.line_to_char(line) + col;
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        if self.text.len_chars() == 0 {
            return (0, 0);
        }
        let line = self.text.char_to_line(self.cursor_char);
        let col = self.cursor_char - self.text.line_to_char(line);
        (line, col)
    }

    pub fn line_len(&self, line_idx: usize) -> usize {
        if line_idx >= self.text.len_lines() {
            return 0;
        }
        let slice = self.text.line(line_idx);
        let mut len = slice.len_chars();
        while len > 0 && matches!(slice.char(len - 1), '\n' | '\r') {
            len -= 1;
        }
        len
    }

    pub fn text(&self) -> &Rope {
        &self.text
    }

    pub fn fit_view(&mut self, vis_lines: usize, vis_cols: usize) {
        let (line, col) = self.cursor_pos();
        if vis_lines > 0 {
            if line < self.scroll_line {
                self.scroll_line = line;
            } else if line >= self.scroll_line + vis_lines {
                self.scroll_line = line - vis_lines + 1;
            }
            let max_scroll_line = self.text.len_lines().saturating_sub(vis_lines);
            if self.scroll_line > max_scroll_line {
                self.scroll_line = max_scroll_line;
            }
        }
        if vis_cols > 0 {
            if col < self.scroll_col {
                self.scroll_col = col;
            } else if col >= self.scroll_col + vis_cols {
                self.scroll_col = col - vis_cols + 1;
            }
            let max_scroll_col = self.max_line_len.saturating_sub(vis_cols);
            if self.scroll_col > max_scroll_col {
                self.scroll_col = max_scroll_col;
            }
        }
    }

    fn recompute_max_line_len(&mut self) {
        let mut max_len = 0;
        for i in 0..self.text.len_lines() {
            let len = self.line_len(i);
            if len > max_len {
                max_len = len;
            }
        }
        self.max_line_len = max_len;
    }
}
