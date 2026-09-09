use super::history::{EditAction, History};
use ropey::Rope;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub struct EditorBuffer {
    text: Rope,
    pub file_path: Option<PathBuf>,
    pub is_modified: bool,
    pub cursor_char: usize,
    pub selection_anchor: Option<usize>,
    pub scroll_line: usize,
    pub scroll_col: usize,
    pub max_line_len: usize,
    history: History,
}

#[derive(PartialEq, Eq)]
enum CharCategory {
    Word,
    Whitespace,
    Punctuation,
}

fn categorize_char(ch: char) -> CharCategory {
    if ch.is_alphanumeric() || ch == '_' {
        CharCategory::Word
    } else if ch == ' ' || ch == '\t' {
        CharCategory::Whitespace
    } else {
        CharCategory::Punctuation
    }
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            file_path: None,
            is_modified: false,
            cursor_char: 0,
            selection_anchor: None,
            scroll_line: 0,
            scroll_col: 0,
            max_line_len: 0,
            history: History::new(),
        }
    }

    pub fn load_file(&mut self, path: PathBuf) -> io::Result<()> {
        let file = File::open(&path)?;
        let text = Rope::from_reader(BufReader::new(file))?;
        self.text = text;
        self.file_path = Some(path);
        self.is_modified = false;
        self.cursor_char = 0;
        self.selection_anchor = None;
        self.scroll_line = 0;
        self.scroll_col = 0;
        self.history = History::new();
        self.recompute_max_line_len();
        Ok(())
    }

    pub fn load_recovered(&mut self, path: PathBuf, recovery_path: &Path) -> io::Result<()> {
        let file = File::open(recovery_path)?;
        let text = Rope::from_reader(BufReader::new(file))?;
        self.text = text;
        self.file_path = Some(path);
        self.is_modified = true;
        self.cursor_char = 0;
        self.selection_anchor = None;
        self.scroll_line = 0;
        self.scroll_col = 0;
        self.history = History::new();
        self.recompute_max_line_len();
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.file_path {
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            for chunk in self.text.chunks() {
                writer.write_all(chunk.as_bytes())?;
            }
            writer.flush()?;
            self.is_modified = false;
        }
        Ok(())
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
            .map(|(s, e)| self.text.slice(s..e).to_string())
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            let removed = self.text.slice(start..end).to_string();
            self.text.remove(start..end);
            self.cursor_char = start;
            self.selection_anchor = None;
            self.is_modified = true;
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

    pub fn type_char(&mut self, ch: char) {
        if let Some((start, end)) = self.selection_range() {
            let matching_close = match ch {
                '(' => Some(')'),
                '[' => Some(']'),
                '{' => Some('}'),
                '"' => Some('"'),
                '\'' => Some('\''),
                _ => None,
            };

            if let Some(close_ch) = matching_close {
                let selected = self.text.slice(start..end).to_string();
                let wrapped = format!("{ch}{selected}{close_ch}");
                self.text.remove(start..end);
                self.history.record(EditAction::Delete {
                    char_idx: start,
                    text: selected,
                });
                self.text.insert(start, &wrapped);
                self.history.record(EditAction::Insert {
                    char_idx: start,
                    text: wrapped,
                });
                self.selection_anchor = Some(start + 1);
                self.cursor_char = end + 1;
                self.is_modified = true;
                self.recompute_max_line_len();
                return;
            }
        }

        if matches!(ch, ')' | ']' | '}' | '"' | '\'') {
            if self.cursor_char < self.text.len_chars() && self.selection_anchor.is_none() {
                let next_ch = self.text.char(self.cursor_char);
                if next_ch == ch {
                    self.cursor_char += 1;
                    return;
                }
            }
        }

        let pair = match ch {
            '(' => Some("()"),
            '[' => Some("[]"),
            '{' => Some("{}"),
            '"' => Some("\"\""),
            '\'' => {
                let prev_is_alphanumeric = if self.cursor_char > 0 {
                    self.text.char(self.cursor_char - 1).is_alphanumeric()
                } else {
                    false
                };
                if prev_is_alphanumeric {
                    None
                } else {
                    Some("''")
                }
            }
            _ => None,
        };

        if let Some(p) = pair {
            self.delete_selection();
            self.text.insert(self.cursor_char, p);
            self.history.record(EditAction::Insert {
                char_idx: self.cursor_char,
                text: p.to_string(),
            });
            self.cursor_char += 1;
            self.is_modified = true;
            self.recompute_max_line_len();
        } else {
            self.insert_char(ch);
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
        self.is_modified = true;
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
        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn insert_newline(&mut self) {
        self.delete_selection();

        let (line_idx, col_idx) = self.cursor_pos();
        let line_start = self.text.line_to_char(line_idx);
        let line = self.text.line(line_idx);

        let mut indent = String::new();
        for ch in line.chars() {
            if ch == ' ' || ch == '\t' {
                indent.push(ch);
            } else {
                break;
            }
        }

        if col_idx < indent.chars().count() {
            indent.clear();
        }

        let prefix = self.text.slice(line_start..self.cursor_char).to_string();
        let extra_indent = prefix.trim_end().ends_with('{');

        let next_is_closing_brace = if self.cursor_char < self.text.len_chars() {
            self.text.char(self.cursor_char) == '}'
        } else {
            false
        };

        if extra_indent && next_is_closing_brace {
            let mut insertion = String::from("\n");
            insertion.push_str(&indent);
            insertion.push_str("    \n");
            insertion.push_str(&indent);

            let cursor_target = self.cursor_char + 1 + indent.chars().count() + 4;

            self.text.insert(self.cursor_char, &insertion);
            self.history.record(EditAction::Insert {
                char_idx: self.cursor_char,
                text: insertion,
            });
            self.cursor_char = cursor_target;
        } else {
            let mut insertion = String::from("\n");
            insertion.push_str(&indent);
            if extra_indent {
                insertion.push_str("    ");
            }

            let char_count = insertion.chars().count();
            self.text.insert(self.cursor_char, &insertion);
            self.history.record(EditAction::Insert {
                char_idx: self.cursor_char,
                text: insertion,
            });
            self.cursor_char += char_count;
        }

        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn delete_backwards(&mut self) {
        if !self.delete_selection() && self.cursor_char > 0 {
            let prev_idx = self.cursor_char - 1;
            if self.cursor_char < self.text.len_chars() {
                let prev = self.text.char(prev_idx);
                let next = self.text.char(self.cursor_char);
                if matches!(
                    (prev, next),
                    ('(', ')') | ('[', ']') | ('{', '}') | ('"', '"') | ('\'', '\'')
                ) {
                    let removed = self.text.slice(prev_idx..prev_idx + 2).to_string();
                    self.text.remove(prev_idx..prev_idx + 2);
                    self.cursor_char = prev_idx;
                    self.history.record(EditAction::Delete {
                        char_idx: prev_idx,
                        text: removed,
                    });
                    self.is_modified = true;
                    self.recompute_max_line_len();
                    return;
                }
            }

            self.cursor_char -= 1;
            let removed = self.text.char(self.cursor_char).to_string();
            self.text.remove(self.cursor_char..self.cursor_char + 1);
            self.history.record(EditAction::Delete {
                char_idx: self.cursor_char,
                text: removed,
            });
            self.is_modified = true;
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
            self.is_modified = true;
            self.recompute_max_line_len();
        }
    }

    pub fn select_all(&mut self) {
        if self.text.len_chars() > 0 {
            self.selection_anchor = Some(0);
            self.cursor_char = self.text.len_chars();
        }
    }

    pub fn select_word_at_cursor(&mut self, target_col: usize) {
        let total_chars = self.text.len_chars();
        if total_chars == 0 {
            return;
        }
        let (line_idx, _) = self.cursor_pos();
        let line_len = self.line_len(line_idx);
        if line_len == 0 || target_col > line_len {
            self.selection_anchor = None;
            return;
        }

        let line_start = self.text.line_to_char(line_idx);
        let line_end = line_start + line_len;

        let check_idx = if self.cursor_char >= line_end {
            line_end.saturating_sub(1)
        } else {
            self.cursor_char
        };

        let ch = self.text.char(check_idx);
        let target_cat = categorize_char(ch);

        let mut start = check_idx;
        while start > line_start {
            let prev_ch = self.text.char(start - 1);
            if categorize_char(prev_ch) != target_cat {
                break;
            }
            start -= 1;
        }

        let mut end = check_idx + 1;
        while end < line_end {
            let next_ch = self.text.char(end);
            if categorize_char(next_ch) != target_cat {
                break;
            }
            end += 1;
        }

        self.selection_anchor = Some(start);
        self.cursor_char = end;
    }

    pub fn select_line_at_cursor(&mut self) {
        let total_chars = self.text.len_chars();
        if total_chars == 0 {
            return;
        }
        let (line_idx, _) = self.cursor_pos();
        let line_len = self.line_len(line_idx);
        let line_start = self.text.line_to_char(line_idx);
        let line_end = line_start + line_len;

        if line_len > 0 {
            self.selection_anchor = Some(line_start);
            self.cursor_char = line_end;
        } else {
            self.selection_anchor = None;
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
            self.is_modified = true;
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
            self.is_modified = true;
            self.recompute_max_line_len();
        }
    }

    #[inline(always)]
    fn prepare_move(&mut self, selecting: bool) {
        if selecting {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_char);
            }
        } else {
            self.selection_anchor = None;
        }
    }

    pub fn move_left(&mut self, sel: bool) {
        self.prepare_move(sel);
        self.cursor_char = self.cursor_char.saturating_sub(1);
    }

    pub fn move_right(&mut self, sel: bool) {
        self.prepare_move(sel);
        if self.cursor_char < self.text.len_chars() {
            self.cursor_char += 1;
        }
    }

    pub fn move_up(&mut self, sel: bool) {
        self.prepare_move(sel);
        let (line, col) = self.cursor_pos();
        if line > 0 {
            let target = line - 1;
            let len = self.line_len(target);
            self.cursor_char = self.text.line_to_char(target) + col.min(len);
        }
    }

    pub fn move_down(&mut self, sel: bool) {
        self.prepare_move(sel);
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
            let max_scroll_line = self.text.len_lines().saturating_sub(1);
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
        self.max_line_len = self
            .text
            .lines()
            .map(|slice| {
                let mut len = slice.len_chars();
                while len > 0 && matches!(slice.char(len - 1), '\n' | '\r') {
                    len -= 1;
                }
                len
            })
            .max()
            .unwrap_or(0);
    }
}
