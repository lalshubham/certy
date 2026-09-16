use crate::editor::buffer::EditorBuffer;

pub fn find_matches(
    text_chars: &[char],
    query: &str,
    match_case: bool,
    whole_word: bool,
) -> Vec<(usize, usize)> {
    if query.is_empty() || text_chars.is_empty() {
        return Vec::new();
    }
    let query_chars: Vec<char> = if match_case {
        query.chars().collect()
    } else {
        query.chars().map(|c| c.to_ascii_lowercase()).collect()
    };
    let q_len = query_chars.len();
    if q_len == 0 || q_len > text_chars.len() {
        return Vec::new();
    }

    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    let mut matches = Vec::new();
    let limit = text_chars.len() - q_len;

    for i in 0..=limit {
        let is_match = if match_case {
            text_chars[i..i + q_len] == query_chars[..]
        } else {
            text_chars[i..i + q_len]
                .iter()
                .zip(&query_chars)
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
        };

        if is_match {
            if whole_word {
                let prev_ok = i == 0 || !is_word_char(text_chars[i - 1]);
                let next_ok =
                    (i + q_len == text_chars.len()) || !is_word_char(text_chars[i + q_len]);
                if !prev_ok || !next_ok {
                    continue;
                }
            }
            matches.push((i, i + q_len));
        }
    }
    matches
}

impl super::FindState {
    pub fn update_matches(&mut self, buffer: &EditorBuffer) {
        if self.query.is_empty() {
            self.matches.clear();
            self.active_match_idx = None;
            return;
        }
        let text_chars: Vec<char> = buffer.text().chars().collect();
        self.matches = find_matches(&text_chars, &self.query, self.match_case, self.whole_word);
        if self.matches.is_empty() {
            self.active_match_idx = None;
        } else {
            let cur = buffer.cursor_char;
            let closest = self
                .matches
                .iter()
                .position(|&(start, _)| start >= cur)
                .unwrap_or(0);
            self.active_match_idx = Some(closest);
        }
    }

    pub fn sync_view(&self, buffer: &mut EditorBuffer, vis_lines: usize, vis_cols: usize) {
        if let Some(idx) = self.active_match_idx {
            if let Some(&(start, end)) = self.matches.get(idx) {
                buffer.selection_anchor = Some(start);
                buffer.cursor_char = end;
                buffer.fit_view(vis_lines, vis_cols);
            }
        }
    }

    pub fn next_match(&mut self, buffer: &mut EditorBuffer, vis_lines: usize, vis_cols: usize) {
        if self.matches.is_empty() {
            return;
        }
        let next_idx = match self.active_match_idx {
            Some(idx) => (idx + 1) % self.matches.len(),
            None => 0,
        };
        self.active_match_idx = Some(next_idx);
        self.sync_view(buffer, vis_lines, vis_cols);
    }

    pub fn prev_match(&mut self, buffer: &mut EditorBuffer, vis_lines: usize, vis_cols: usize) {
        if self.matches.is_empty() {
            return;
        }
        let prev_idx = match self.active_match_idx {
            Some(idx) if idx > 0 => idx - 1,
            _ => self.matches.len() - 1,
        };
        self.active_match_idx = Some(prev_idx);
        self.sync_view(buffer, vis_lines, vis_cols);
    }

    pub fn replace_current(
        &mut self,
        buffer: &mut EditorBuffer,
        vis_lines: usize,
        vis_cols: usize,
    ) {
        if self.matches.is_empty() {
            return;
        }
        let idx = self.active_match_idx.unwrap_or(0);
        if idx < self.matches.len() {
            let (start, end) = self.matches[idx];
            buffer.replace_range(start, end, &self.replace_text);
            self.update_matches(buffer);
            if !self.matches.is_empty() {
                let next_idx = idx.min(self.matches.len() - 1);
                self.active_match_idx = Some(next_idx);
                self.sync_view(buffer, vis_lines, vis_cols);
            }
        }
    }

    pub fn replace_all(&mut self, buffer: &mut EditorBuffer, vis_lines: usize, vis_cols: usize) {
        if self.matches.is_empty() {
            return;
        }
        for &(start, end) in self.matches.iter().rev() {
            buffer.replace_range(start, end, &self.replace_text);
        }
        self.update_matches(buffer);
        if !self.matches.is_empty() {
            self.active_match_idx = Some(0);
            self.sync_view(buffer, vis_lines, vis_cols);
        }
    }
}
