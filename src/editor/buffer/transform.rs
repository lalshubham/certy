use super::EditorBuffer;
use crate::editor::history::EditAction;
use crate::syntax::Language;

impl EditorBuffer {
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
            if self.cursor_char >= line_start {
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
}
