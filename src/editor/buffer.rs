use super::history::{EditAction, History};
use crate::syntax::Language;
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

    pub fn replace_range(&mut self, start: usize, end: usize, new_text: &str) {
        if start >= end || end > self.text.len_chars() {
            return;
        }
        let removed = self.text.slice(start..end).to_string();
        self.text.remove(start..end);
        self.history.record(EditAction::Delete {
            char_idx: start,
            text: removed,
        });
        self.text.insert(start, new_text);
        self.history.record(EditAction::Insert {
            char_idx: start,
            text: new_text.to_string(),
        });
        self.cursor_char = start + new_text.chars().count();
        self.selection_anchor = None;
        self.is_modified = true;
        self.recompute_max_line_len();
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

    pub fn indent_selection(&mut self) {
        let (start_line, end_line) = match self.selection_range() {
            Some((start, end)) => {
                let s_line = self.text.char_to_line(start);
                let mut e_line = self.text.char_to_line(end);
                if end > start && end == self.text.line_to_char(e_line) {
                    e_line = e_line.saturating_sub(1);
                }
                (s_line, e_line)
            }
            None => {
                let (line, _) = self.cursor_pos();
                (line, line)
            }
        };

        let is_multi_line = end_line > start_line;

        for line in (start_line..=end_line).rev() {
            if is_multi_line && self.line_len(line) == 0 {
                continue;
            }
            let line_start = self.text.line_to_char(line);
            self.text.insert(line_start, "    ");
            self.history.record(EditAction::Insert {
                char_idx: line_start,
                text: "    ".to_string(),
            });

            if self.cursor_char > line_start {
                self.cursor_char += 4;
            } else if self.cursor_char == line_start && self.selection_anchor.is_none() {
                self.cursor_char += 4;
            }

            if let Some(ref mut anchor) = self.selection_anchor {
                if *anchor > line_start {
                    *anchor += 4;
                }
            }
        }

        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn unindent_selection(&mut self) {
        let (start_line, end_line) = match self.selection_range() {
            Some((start, end)) => {
                let s_line = self.text.char_to_line(start);
                let mut e_line = self.text.char_to_line(end);
                if end > start && end == self.text.line_to_char(e_line) {
                    e_line = e_line.saturating_sub(1);
                }
                (s_line, e_line)
            }
            None => {
                let (line, _) = self.cursor_pos();
                (line, line)
            }
        };

        for line in (start_line..=end_line).rev() {
            let line_start = self.text.line_to_char(line);
            let line_slice = self.text.line(line);
            let mut remove_count = 0;
            for ch in line_slice.chars() {
                if ch == ' ' {
                    remove_count += 1;
                    if remove_count == 4 {
                        break;
                    }
                } else if ch == '\t' {
                    if remove_count == 0 {
                        remove_count = 1;
                    }
                    break;
                } else {
                    break;
                }
            }

            if remove_count == 0 {
                continue;
            }

            let remove_range = line_start..line_start + remove_count;
            let removed_text = self.text.slice(remove_range.clone()).to_string();
            self.text.remove(remove_range);
            self.history.record(EditAction::Delete {
                char_idx: line_start,
                text: removed_text,
            });

            if self.cursor_char >= line_start + remove_count {
                self.cursor_char -= remove_count;
            } else if self.cursor_char > line_start {
                self.cursor_char = line_start;
            }

            if let Some(ref mut anchor) = self.selection_anchor {
                if *anchor >= line_start + remove_count {
                    *anchor -= remove_count;
                } else if *anchor > line_start {
                    *anchor = line_start;
                }
            }
        }

        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn duplicate_line(&mut self) {
        let (start_line, end_line) = match self.selection_range() {
            Some((start, end)) => {
                let s_line = self.text.char_to_line(start);
                let mut e_line = self.text.char_to_line(end);
                if end > start && end == self.text.line_to_char(e_line) {
                    e_line = e_line.saturating_sub(1);
                }
                (s_line, e_line)
            }
            None => {
                let (line, _) = self.cursor_pos();
                (line, line)
            }
        };

        let line_start = self.text.line_to_char(start_line);
        let has_next = end_line + 1 < self.text.len_lines();
        let line_end = if has_next {
            self.text.line_to_char(end_line + 1)
        } else {
            self.text.len_chars()
        };

        let mut text = self.text.slice(line_start..line_end).to_string();
        let insert_pos = line_end;
        if !has_next && !text.ends_with('\n') {
            text.insert(0, '\n');
        }

        let len = text.chars().count();
        self.text.insert(insert_pos, &text);
        self.history.record(EditAction::Insert {
            char_idx: insert_pos,
            text,
        });

        self.cursor_char += len;
        if let Some(ref mut anchor) = self.selection_anchor {
            *anchor += len;
        }

        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn move_line_up(&mut self) {
        let (start_line, end_line) = match self.selection_range() {
            Some((start, end)) => {
                let s_line = self.text.char_to_line(start);
                let mut e_line = self.text.char_to_line(end);
                if end > start && end == self.text.line_to_char(e_line) {
                    e_line = e_line.saturating_sub(1);
                }
                (s_line, e_line)
            }
            None => {
                let (line, _) = self.cursor_pos();
                (line, line)
            }
        };

        if start_line == 0 {
            return;
        }

        let prev_line = start_line - 1;
        let prev_start = self.text.line_to_char(prev_line);
        let block_start = self.text.line_to_char(start_line);
        let has_next = end_line + 1 < self.text.len_lines();
        let block_end = if has_next {
            self.text.line_to_char(end_line + 1)
        } else {
            self.text.len_chars()
        };

        let prev_text = self.text.slice(prev_start..block_start).to_string();
        let block_text = self.text.slice(block_start..block_end).to_string();

        let newline_seq = if prev_text.ends_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        };

        let (new_block, new_prev) = if !block_text.ends_with('\n') && prev_text.ends_with('\n') {
            let prev_trimmed = &prev_text[..prev_text.len() - newline_seq.len()];
            (
                format!("{block_text}{newline_seq}"),
                prev_trimmed.to_string(),
            )
        } else {
            (block_text, prev_text)
        };

        let combined = format!("{new_block}{new_prev}");
        let total_range = prev_start..block_end;
        let old_text = self.text.slice(total_range.clone()).to_string();

        self.text.remove(total_range);
        self.history.record(EditAction::Delete {
            char_idx: prev_start,
            text: old_text,
        });

        self.text.insert(prev_start, &combined);
        self.history.record(EditAction::Insert {
            char_idx: prev_start,
            text: combined,
        });

        let offset_in_block = self.cursor_char.saturating_sub(block_start);
        self.cursor_char = prev_start + offset_in_block;

        if let Some(ref mut anchor) = self.selection_anchor {
            let anchor_offset = anchor.saturating_sub(block_start);
            *anchor = prev_start + anchor_offset;
        }

        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn move_line_down(&mut self) {
        let (start_line, end_line) = match self.selection_range() {
            Some((start, end)) => {
                let s_line = self.text.char_to_line(start);
                let mut e_line = self.text.char_to_line(end);
                if end > start && end == self.text.line_to_char(e_line) {
                    e_line = e_line.saturating_sub(1);
                }
                (s_line, e_line)
            }
            None => {
                let (line, _) = self.cursor_pos();
                (line, line)
            }
        };

        let next_line = end_line + 1;
        if next_line >= self.text.len_lines() {
            return;
        }

        let block_start = self.text.line_to_char(start_line);
        let next_line_start = self.text.line_to_char(next_line);
        let has_after_next = next_line + 1 < self.text.len_lines();
        let next_line_end = if has_after_next {
            self.text.line_to_char(next_line + 1)
        } else {
            self.text.len_chars()
        };

        let block_text = self.text.slice(block_start..next_line_start).to_string();
        let next_text = self.text.slice(next_line_start..next_line_end).to_string();

        let newline_seq = if block_text.ends_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        };

        let (new_next, new_block) = if !next_text.ends_with('\n') && block_text.ends_with('\n') {
            let block_trimmed = &block_text[..block_text.len() - newline_seq.len()];
            (
                format!("{next_text}{newline_seq}"),
                block_trimmed.to_string(),
            )
        } else {
            (next_text, block_text)
        };

        let combined = format!("{new_next}{new_block}");
        let total_range = block_start..next_line_end;
        let old_text = self.text.slice(total_range.clone()).to_string();

        self.text.remove(total_range);
        self.history.record(EditAction::Delete {
            char_idx: block_start,
            text: old_text,
        });

        self.text.insert(block_start, &combined);
        self.history.record(EditAction::Insert {
            char_idx: block_start,
            text: combined,
        });

        let new_block_start = block_start + new_next.chars().count();
        let offset_in_block = self.cursor_char.saturating_sub(block_start);
        self.cursor_char = new_block_start + offset_in_block;

        if let Some(ref mut anchor) = self.selection_anchor {
            let anchor_offset = anchor.saturating_sub(block_start);
            *anchor = new_block_start + anchor_offset;
        }

        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn delete_line(&mut self) {
        let total_lines = self.text.len_lines();
        if self.text.len_chars() == 0 {
            return;
        }

        let (start_line, end_line) = match self.selection_range() {
            Some((start, end)) => {
                let s_line = self.text.char_to_line(start);
                let mut e_line = self.text.char_to_line(end);
                if end > start && end == self.text.line_to_char(e_line) {
                    e_line = e_line.saturating_sub(1);
                }
                (s_line, e_line)
            }
            None => {
                let (line, _) = self.cursor_pos();
                (line, line)
            }
        };

        let (del_start, del_end) = if end_line + 1 < total_lines {
            (
                self.text.line_to_char(start_line),
                self.text.line_to_char(end_line + 1),
            )
        } else if start_line > 0 {
            (
                self.text.line_to_char(start_line - 1) + self.line_len(start_line - 1),
                self.text.len_chars(),
            )
        } else {
            (0, self.text.len_chars())
        };

        if del_start < del_end {
            let removed = self.text.slice(del_start..del_end).to_string();
            self.text.remove(del_start..del_end);
            self.history.record(EditAction::Delete {
                char_idx: del_start,
                text: removed,
            });
        }

        self.selection_anchor = None;
        self.cursor_char = del_start.min(self.text.len_chars());
        self.is_modified = true;
        self.recompute_max_line_len();
    }

    pub fn toggle_line_comment(&mut self) {
        let lang = Language::from_path(self.file_path.as_deref());
        let prefix = match lang.line_comment_prefix() {
            Some(p) => p,
            None => return,
        };

        if self.text.len_chars() == 0 {
            let comment_str = format!("{prefix} ");
            let comment_len = comment_str.chars().count();
            self.text.insert(0, &comment_str);
            self.history.record(EditAction::Insert {
                char_idx: 0,
                text: comment_str,
            });
            self.cursor_char = comment_len;
            self.is_modified = true;
            self.recompute_max_line_len();
            return;
        }

        let (start_line, end_line) = match self.selection_range() {
            Some((start, end)) => {
                let s_line = self.text.char_to_line(start);
                let mut e_line = self.text.char_to_line(end);
                if end > start && end == self.text.line_to_char(e_line) {
                    e_line = e_line.saturating_sub(1);
                }
                (s_line, e_line)
            }
            None => {
                let (line, _) = self.cursor_pos();
                (line, line)
            }
        };

        let mut has_non_empty = false;
        let mut all_commented = true;

        for line in start_line..=end_line {
            let line_len = self.line_len(line);
            let line_start = self.text.line_to_char(line);
            let line_str = self
                .text
                .slice(line_start..line_start + line_len)
                .to_string();
            let indent_len = line_str
                .chars()
                .take_while(|&c| c == ' ' || c == '\t')
                .count();
            let trimmed = &line_str[indent_len..];

            if !trimmed.is_empty() {
                has_non_empty = true;
                if !trimmed.starts_with(prefix) {
                    all_commented = false;
                    break;
                }
            }
        }

        if !has_non_empty {
            all_commented = false;
        }

        if all_commented {
            for line in (start_line..=end_line).rev() {
                let line_len = self.line_len(line);
                if line_len == 0 {
                    continue;
                }
                let line_start = self.text.line_to_char(line);
                let line_str = self
                    .text
                    .slice(line_start..line_start + line_len)
                    .to_string();
                let indent_len = line_str
                    .chars()
                    .take_while(|&c| c == ' ' || c == '\t')
                    .count();
                let trimmed = &line_str[indent_len..];

                if trimmed.starts_with(prefix) {
                    let del_start = line_start + indent_len;
                    let del_count = if trimmed[prefix.len()..].starts_with(' ') {
                        prefix.chars().count() + 1
                    } else {
                        prefix.chars().count()
                    };

                    let del_range = del_start..del_start + del_count;
                    let removed = self.text.slice(del_range.clone()).to_string();
                    self.text.remove(del_range);
                    self.history.record(EditAction::Delete {
                        char_idx: del_start,
                        text: removed,
                    });

                    if self.cursor_char >= del_start + del_count {
                        self.cursor_char -= del_count;
                    } else if self.cursor_char > del_start {
                        self.cursor_char = del_start;
                    }

                    if let Some(ref mut anchor) = self.selection_anchor {
                        if *anchor >= del_start + del_count {
                            *anchor -= del_count;
                        } else if *anchor > del_start {
                            *anchor = del_start;
                        }
                    }
                }
            }
        } else {
            let comment_str = format!("{prefix} ");
            let comment_len = comment_str.chars().count();

            for line in (start_line..=end_line).rev() {
                let line_len = self.line_len(line);
                if start_line != end_line && line_len == 0 {
                    continue;
                }
                let line_start = self.text.line_to_char(line);
                let line_str = self
                    .text
                    .slice(line_start..line_start + line_len)
                    .to_string();
                let indent_len = line_str
                    .chars()
                    .take_while(|&c| c == ' ' || c == '\t')
                    .count();
                let ins_pos = line_start + indent_len;

                self.text.insert(ins_pos, &comment_str);
                self.history.record(EditAction::Insert {
                    char_idx: ins_pos,
                    text: comment_str.clone(),
                });

                if self.cursor_char >= ins_pos {
                    self.cursor_char += comment_len;
                }

                if let Some(ref mut anchor) = self.selection_anchor {
                    if *anchor >= ins_pos {
                        *anchor += comment_len;
                    }
                }
            }
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
