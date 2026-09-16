mod search;

use crate::editor::buffer::EditorBuffer;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum FindField {
    #[default]
    Find,
    Replace,
}

#[derive(Default)]
pub struct FindState {
    pub is_open: bool,
    pub focused: bool,
    pub is_replace: bool,
    pub query: String,
    pub replace_text: String,
    pub query_cursor: usize,
    pub replace_cursor: usize,
    pub query_scroll: usize,
    pub replace_scroll: usize,
    pub query_selection_anchor: Option<usize>,
    pub replace_selection_anchor: Option<usize>,
    pub scroll_x: usize,
    pub active_field: FindField,
    pub matches: Vec<(usize, usize)>,
    pub active_match_idx: Option<usize>,
    pub hovered_btn: Option<usize>,
    pub match_case: bool,
    pub whole_word: bool,
    pub use_regex: bool,
}

impl FindState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn counter_text(&self) -> String {
        if self.query.is_empty() {
            String::new()
        } else if self.matches.is_empty() {
            "No results".to_string()
        } else {
            format!(
                "{} of {}",
                self.active_match_idx.map(|i| i + 1).unwrap_or(0),
                self.matches.len()
            )
        }
    }

    pub fn open(&mut self, is_replace: bool, initial_query: Option<String>, buffer: &EditorBuffer) {
        self.is_open = true;
        self.focused = true;
        self.is_replace = is_replace;
        self.active_field = FindField::Find;
        if let Some(q) = initial_query {
            if !q.is_empty() && !q.contains('\n') {
                self.query = q;
            }
        }
        self.query_cursor = self.query.chars().count();
        self.replace_cursor = self.replace_text.chars().count();
        self.query_scroll = 0;
        self.replace_scroll = 0;
        self.query_selection_anchor = None;
        self.replace_selection_anchor = None;
        self.hovered_btn = None;
        self.scroll_x = 0;
        self.update_matches(buffer);
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.focused = false;
        self.matches.clear();
        self.active_match_idx = None;
        self.hovered_btn = None;
    }

    pub fn total_content_width(&self, char_w: usize) -> usize {
        let cw = char_w.max(1);
        let find_input_w: usize = 240;
        let mc_w = "Match Case".len() * cw + 16;
        let ww_w = "Whole Word".len() * cw + 16;
        let re_w = "Regex".len() * cw + 16;
        let prev_w = "Previous".len() * cw + 16;
        let next_w = "Next".len() * cw + 16;
        let counter_w = self.counter_text().len() * cw + 16;
        let row1 =
            find_input_w + 6 + mc_w + 6 + ww_w + 6 + re_w + 6 + prev_w + 6 + next_w + 6 + counter_w;
        let row2 = if self.is_replace {
            let rep_input_w: usize = 240;
            let rep_w = "Replace".len() * cw + 16;
            let all_w = "Replace All".len() * cw + 16;
            rep_input_w + 6 + rep_w + 6 + all_w
        } else {
            0
        };
        row1.max(row2) + 20
    }

    pub fn clamp_scroll(&mut self, char_w: usize, available_w: usize) {
        let total_w = self.total_content_width(char_w);
        let max_scroll = total_w.saturating_sub(available_w);
        if self.scroll_x > max_scroll {
            self.scroll_x = max_scroll;
        }
    }

    pub fn ensure_query_visible(&mut self, max_vis: usize) {
        if self.query_cursor < self.query_scroll {
            self.query_scroll = self.query_cursor;
        } else if self.query_cursor > self.query_scroll + max_vis {
            self.query_scroll = self.query_cursor.saturating_sub(max_vis);
        }
        let max_scroll = self.query.chars().count().saturating_sub(max_vis);
        self.query_scroll = self.query_scroll.min(max_scroll);
    }

    pub fn ensure_replace_visible(&mut self, max_vis: usize) {
        if self.replace_cursor < self.replace_scroll {
            self.replace_scroll = self.replace_cursor;
        } else if self.replace_cursor > self.replace_scroll + max_vis {
            self.replace_scroll = self.replace_cursor.saturating_sub(max_vis);
        }
        let max_scroll = self.replace_text.chars().count().saturating_sub(max_vis);
        self.replace_scroll = self.replace_scroll.min(max_scroll);
    }

    pub fn delete_selection(&mut self) -> bool {
        let (anchor, cursor, text) = match self.active_field {
            FindField::Find => (
                &mut self.query_selection_anchor,
                &mut self.query_cursor,
                &mut self.query,
            ),
            FindField::Replace => (
                &mut self.replace_selection_anchor,
                &mut self.replace_cursor,
                &mut self.replace_text,
            ),
        };
        if let Some(a) = *anchor {
            let start = a.min(*cursor);
            let end = a.max(*cursor);
            if start != end {
                let mut chars: Vec<char> = text.chars().collect();
                chars.drain(start..end);
                *text = chars.into_iter().collect();
                *cursor = start;
                *anchor = None;
                return true;
            }
            *anchor = None;
        }
        false
    }

    pub fn select_all(&mut self) {
        match self.active_field {
            FindField::Find => {
                self.query_selection_anchor = Some(0);
                self.query_cursor = self.query.chars().count();
            }
            FindField::Replace => {
                self.replace_selection_anchor = Some(0);
                self.replace_cursor = self.replace_text.chars().count();
            }
        }
    }

    pub fn select_word(&mut self) {
        let (text, cursor, anchor) = match self.active_field {
            FindField::Find => (
                &self.query,
                &mut self.query_cursor,
                &mut self.query_selection_anchor,
            ),
            FindField::Replace => (
                &self.replace_text,
                &mut self.replace_cursor,
                &mut self.replace_selection_anchor,
            ),
        };
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            *anchor = None;
            return;
        }
        let cur = (*cursor).min(chars.len().saturating_sub(1));
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let target_is_word = is_word(chars[cur]);
        let mut start = cur;
        while start > 0 && is_word(chars[start - 1]) == target_is_word {
            start -= 1;
        }
        let mut end = cur + 1;
        while end < chars.len() && is_word(chars[end]) == target_is_word {
            end += 1;
        }
        *anchor = Some(start);
        *cursor = end;
    }

    pub fn selected_text(&self) -> Option<String> {
        let (anchor, cursor, text) = match self.active_field {
            FindField::Find => (self.query_selection_anchor, self.query_cursor, &self.query),
            FindField::Replace => (
                self.replace_selection_anchor,
                self.replace_cursor,
                &self.replace_text,
            ),
        };
        anchor.map(|a| {
            let start = a.min(cursor);
            let end = a.max(cursor);
            let chars: Vec<char> = text.chars().collect();
            chars[start..end].iter().collect()
        })
    }

    pub fn insert_char_at_cursor(&mut self, ch: char, buffer: &EditorBuffer) {
        self.delete_selection();
        let (cursor, text, is_find) = match self.active_field {
            FindField::Find => (&mut self.query_cursor, &mut self.query, true),
            FindField::Replace => (&mut self.replace_cursor, &mut self.replace_text, false),
        };
        let mut chars: Vec<char> = text.chars().collect();
        let pos = (*cursor).min(chars.len());
        chars.insert(pos, ch);
        *text = chars.into_iter().collect();
        *cursor = pos + 1;
        if is_find {
            self.update_matches(buffer);
        }
    }

    pub fn insert_str_at_cursor(&mut self, text_to_ins: &str, buffer: &EditorBuffer) {
        let clean: String = text_to_ins
            .chars()
            .filter(|&c| c != '\n' && c != '\r')
            .collect();
        if clean.is_empty() {
            return;
        }
        self.delete_selection();
        let count = clean.chars().count();
        let (cursor, text, is_find) = match self.active_field {
            FindField::Find => (&mut self.query_cursor, &mut self.query, true),
            FindField::Replace => (&mut self.replace_cursor, &mut self.replace_text, false),
        };
        let mut chars: Vec<char> = text.chars().collect();
        let pos = (*cursor).min(chars.len());
        for (i, ch) in clean.chars().enumerate() {
            chars.insert(pos + i, ch);
        }
        *text = chars.into_iter().collect();
        *cursor = pos + count;
        if is_find {
            self.update_matches(buffer);
        }
    }

    pub fn delete_backwards(&mut self, buffer: &EditorBuffer) {
        if self.delete_selection() {
            if self.active_field == FindField::Find {
                self.update_matches(buffer);
            }
            return;
        }
        let (cursor, text, is_find) = match self.active_field {
            FindField::Find => (&mut self.query_cursor, &mut self.query, true),
            FindField::Replace => (&mut self.replace_cursor, &mut self.replace_text, false),
        };
        if *cursor > 0 {
            let mut chars: Vec<char> = text.chars().collect();
            if *cursor <= chars.len() {
                chars.remove(*cursor - 1);
                *cursor -= 1;
                *text = chars.into_iter().collect();
                if is_find {
                    self.update_matches(buffer);
                }
            }
        }
    }

    pub fn delete_forward(&mut self, buffer: &EditorBuffer) {
        if self.delete_selection() {
            if self.active_field == FindField::Find {
                self.update_matches(buffer);
            }
            return;
        }
        let (cursor, text, is_find) = match self.active_field {
            FindField::Find => (&mut self.query_cursor, &mut self.query, true),
            FindField::Replace => (&mut self.replace_cursor, &mut self.replace_text, false),
        };
        let mut chars: Vec<char> = text.chars().collect();
        if *cursor < chars.len() {
            chars.remove(*cursor);
            *text = chars.into_iter().collect();
            if is_find {
                self.update_matches(buffer);
            }
        }
    }

    #[inline(always)]
    fn prepare_move(&mut self, sel: bool) {
        let (anchor, cursor) = match self.active_field {
            FindField::Find => (&mut self.query_selection_anchor, self.query_cursor),
            FindField::Replace => (&mut self.replace_selection_anchor, self.replace_cursor),
        };
        if sel {
            if anchor.is_none() {
                *anchor = Some(cursor);
            }
        } else {
            *anchor = None;
        }
    }

    pub fn move_cursor_left(&mut self, sel: bool) {
        let (anchor, cursor) = match self.active_field {
            FindField::Find => (&mut self.query_selection_anchor, &mut self.query_cursor),
            FindField::Replace => (&mut self.replace_selection_anchor, &mut self.replace_cursor),
        };
        if !sel {
            if let Some(a) = *anchor {
                *cursor = a.min(*cursor);
                *anchor = None;
                return;
            }
        }
        self.prepare_move(sel);
        let cursor_ref = match self.active_field {
            FindField::Find => &mut self.query_cursor,
            FindField::Replace => &mut self.replace_cursor,
        };
        *cursor_ref = cursor_ref.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self, sel: bool) {
        let (anchor, cursor) = match self.active_field {
            FindField::Find => (&mut self.query_selection_anchor, &mut self.query_cursor),
            FindField::Replace => (&mut self.replace_selection_anchor, &mut self.replace_cursor),
        };
        if !sel {
            if let Some(a) = *anchor {
                *cursor = a.max(*cursor);
                *anchor = None;
                return;
            }
        }
        self.prepare_move(sel);
        let (cursor_ref, text_ref) = match self.active_field {
            FindField::Find => (&mut self.query_cursor, &self.query),
            FindField::Replace => (&mut self.replace_cursor, &self.replace_text),
        };
        if *cursor_ref < text_ref.chars().count() {
            *cursor_ref += 1;
        }
    }

    pub fn move_cursor_home(&mut self, sel: bool) {
        self.prepare_move(sel);
        match self.active_field {
            FindField::Find => self.query_cursor = 0,
            FindField::Replace => self.replace_cursor = 0,
        }
    }

    pub fn move_cursor_end(&mut self, sel: bool) {
        self.prepare_move(sel);
        match self.active_field {
            FindField::Find => self.query_cursor = self.query.chars().count(),
            FindField::Replace => self.replace_cursor = self.replace_text.chars().count(),
        }
    }

    pub fn move_cursor_up(&mut self, sel: bool) {
        if sel {
            self.move_cursor_home(true);
        } else if self.is_replace && self.active_field == FindField::Replace {
            self.active_field = FindField::Find;
        }
    }

    pub fn move_cursor_down(&mut self, sel: bool) {
        if sel {
            self.move_cursor_end(true);
        } else if self.is_replace && self.active_field == FindField::Find {
            self.active_field = FindField::Replace;
        }
    }
}
