use super::EditorBuffer;

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
}
