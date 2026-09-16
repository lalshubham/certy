use super::EditorBuffer;
use crate::editor::history::EditAction;

impl EditorBuffer {
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

        if matches!(ch, ')' | ']' | '}' | '"' | '\'')
            && self.cursor_char < self.text.len_chars()
            && self.selection_anchor.is_none()
            && self.text.char(self.cursor_char) == ch
        {
            self.cursor_char += 1;
            return;
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

        let mut insertion = String::with_capacity(indent.len() * 2 + 16);
        if extra_indent && next_is_closing_brace {
            insertion.push('\n');
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
            insertion.push('\n');
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
}
