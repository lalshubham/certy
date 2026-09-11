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
        self.is_replace = false;
        self.active_field = FindField::Find;
        self.query.clear();
        self.replace_text.clear();
        self.query_cursor = 0;
        self.replace_cursor = 0;
        self.matches.clear();
        self.active_match_idx = None;
        self.hovered_btn = None;
        self.query_selection_anchor = None;
        self.replace_selection_anchor = None;
        self.scroll_x = 0;
        self.query_scroll = 0;
        self.replace_scroll = 0;
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

    pub fn clamp_scroll(&mut self, char_w: usize, available_w: usize) {
        let total_w = self.total_content_width(char_w);
        let max_scroll = total_w.saturating_sub(available_w);
        if self.scroll_x > max_scroll {
            self.scroll_x = max_scroll;
        }
    }

    pub fn total_content_width(&self, char_w: usize) -> usize {
        let cw = char_w.max(1);
        let counter_str = self.counter_text();
        let counter_w = if !counter_str.is_empty() {
            counter_str.len() * cw + 12 + 6
        } else {
            0
        };
        let mc_w = "Match Case".len() * cw + 16;
        let ww_w = "Whole Word".len() * cw + 16;
        let re_w = "Regex".len() * cw + 16;
        let prev_w = "Previous".len() * cw + 16;
        let next_w = "Next".len() * cw + 16;
        let row1_w =
            240 + 6 + mc_w + 6 + ww_w + 6 + re_w + 6 + prev_w + 6 + next_w + 6 + counter_w + 12;
        let row2_w = if self.is_replace {
            let rep_w = "Replace".len() * cw + 16;
            let all_w = "Replace All".len() * cw + 16;
            240 + 6 + rep_w + 6 + all_w + 12
        } else {
            0
        };
        row1_w.max(row2_w)
    }

    pub fn delete_selection(&mut self) -> bool {
        match self.active_field {
            FindField::Find => {
                if let Some(anchor) = self.query_selection_anchor {
                    let start = anchor.min(self.query_cursor);
                    let end = anchor.max(self.query_cursor);
                    if start != end {
                        let mut chars: Vec<char> = self.query.chars().collect();
                        chars.drain(start..end);
                        self.query = chars.into_iter().collect();
                        self.query_cursor = start;
                        self.query_selection_anchor = None;
                        return true;
                    }
                    self.query_selection_anchor = None;
                }
            }
            FindField::Replace => {
                if let Some(anchor) = self.replace_selection_anchor {
                    let start = anchor.min(self.replace_cursor);
                    let end = anchor.max(self.replace_cursor);
                    if start != end {
                        let mut chars: Vec<char> = self.replace_text.chars().collect();
                        chars.drain(start..end);
                        self.replace_text = chars.into_iter().collect();
                        self.replace_cursor = start;
                        self.replace_selection_anchor = None;
                        return true;
                    }
                    self.replace_selection_anchor = None;
                }
            }
        }
        false
    }

    pub fn clear_selection(&mut self) {
        match self.active_field {
            FindField::Find => self.query_selection_anchor = None,
            FindField::Replace => self.replace_selection_anchor = None,
        }
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
        let (text, cursor) = match self.active_field {
            FindField::Find => (&self.query, self.query_cursor),
            FindField::Replace => (&self.replace_text, self.replace_cursor),
        };
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return;
        }
        let check_idx = if cursor >= chars.len() {
            chars.len().saturating_sub(1)
        } else {
            cursor
        };
        let ch = chars[check_idx];
        let is_alnum = ch.is_alphanumeric() || ch == '_';
        let mut start = check_idx;
        while start > 0 {
            let prev = chars[start - 1];
            if (prev.is_alphanumeric() || prev == '_') != is_alnum {
                break;
            }
            start -= 1;
        }
        let mut end = check_idx;
        while end < chars.len() {
            let next = chars[end];
            if (next.is_alphanumeric() || next == '_') != is_alnum {
                break;
            }
            end += 1;
        }
        match self.active_field {
            FindField::Find => {
                self.query_selection_anchor = Some(start);
                self.query_cursor = end;
            }
            FindField::Replace => {
                self.replace_selection_anchor = Some(start);
                self.replace_cursor = end;
            }
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        match self.active_field {
            FindField::Find => self.query_selection_anchor.map(|anchor| {
                let start = anchor.min(self.query_cursor);
                let end = anchor.max(self.query_cursor);
                let chars: Vec<char> = self.query.chars().collect();
                chars[start..end].iter().collect()
            }),
            FindField::Replace => self.replace_selection_anchor.map(|anchor| {
                let start = anchor.min(self.replace_cursor);
                let end = anchor.max(self.replace_cursor);
                let chars: Vec<char> = self.replace_text.chars().collect();
                chars[start..end].iter().collect()
            }),
        }
    }

    pub fn insert_char_at_cursor(&mut self, ch: char, buffer: &EditorBuffer) {
        self.delete_selection();
        match self.active_field {
            FindField::Find => {
                let mut chars: Vec<char> = self.query.chars().collect();
                let pos = self.query_cursor.min(chars.len());
                chars.insert(pos, ch);
                self.query = chars.into_iter().collect();
                self.query_cursor = pos + 1;
                self.update_matches(buffer);
            }
            FindField::Replace => {
                let mut chars: Vec<char> = self.replace_text.chars().collect();
                let pos = self.replace_cursor.min(chars.len());
                chars.insert(pos, ch);
                self.replace_text = chars.into_iter().collect();
                self.replace_cursor = pos + 1;
            }
        }
    }

    pub fn insert_str_at_cursor(&mut self, text: &str, buffer: &EditorBuffer) {
        let clean: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
        if clean.is_empty() {
            return;
        }
        self.delete_selection();
        let count = clean.chars().count();
        match self.active_field {
            FindField::Find => {
                let mut chars: Vec<char> = self.query.chars().collect();
                let pos = self.query_cursor.min(chars.len());
                for (i, ch) in clean.chars().enumerate() {
                    chars.insert(pos + i, ch);
                }
                self.query = chars.into_iter().collect();
                self.query_cursor = pos + count;
                self.update_matches(buffer);
            }
            FindField::Replace => {
                let mut chars: Vec<char> = self.replace_text.chars().collect();
                let pos = self.replace_cursor.min(chars.len());
                for (i, ch) in clean.chars().enumerate() {
                    chars.insert(pos + i, ch);
                }
                self.replace_text = chars.into_iter().collect();
                self.replace_cursor = pos + count;
            }
        }
    }

    pub fn delete_backwards(&mut self, buffer: &EditorBuffer) {
        if self.delete_selection() {
            if self.active_field == FindField::Find {
                self.update_matches(buffer);
            }
            return;
        }
        match self.active_field {
            FindField::Find => {
                if self.query_cursor > 0 {
                    let mut chars: Vec<char> = self.query.chars().collect();
                    if self.query_cursor <= chars.len() {
                        chars.remove(self.query_cursor - 1);
                        self.query_cursor -= 1;
                        self.query = chars.into_iter().collect();
                        self.update_matches(buffer);
                    }
                }
            }
            FindField::Replace => {
                if self.replace_cursor > 0 {
                    let mut chars: Vec<char> = self.replace_text.chars().collect();
                    if self.replace_cursor <= chars.len() {
                        chars.remove(self.replace_cursor - 1);
                        self.replace_cursor -= 1;
                        self.replace_text = chars.into_iter().collect();
                    }
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
        match self.active_field {
            FindField::Find => {
                let mut chars: Vec<char> = self.query.chars().collect();
                if self.query_cursor < chars.len() {
                    chars.remove(self.query_cursor);
                    self.query = chars.into_iter().collect();
                    self.update_matches(buffer);
                }
            }
            FindField::Replace => {
                let mut chars: Vec<char> = self.replace_text.chars().collect();
                if self.replace_cursor < chars.len() {
                    chars.remove(self.replace_cursor);
                    self.replace_text = chars.into_iter().collect();
                }
            }
        }
    }

    #[inline(always)]
    fn prepare_move(&mut self, selecting: bool) {
        if selecting {
            match self.active_field {
                FindField::Find => {
                    if self.query_selection_anchor.is_none() {
                        self.query_selection_anchor = Some(self.query_cursor);
                    }
                }
                FindField::Replace => {
                    if self.replace_selection_anchor.is_none() {
                        self.replace_selection_anchor = Some(self.replace_cursor);
                    }
                }
            }
        } else {
            self.clear_selection();
        }
    }

    pub fn move_cursor_left(&mut self, sel: bool) {
        if !sel {
            let sel_start = match self.active_field {
                FindField::Find => self
                    .query_selection_anchor
                    .map(|a| a.min(self.query_cursor)),
                FindField::Replace => self
                    .replace_selection_anchor
                    .map(|a| a.min(self.replace_cursor)),
            };
            if let Some(start) = sel_start {
                match self.active_field {
                    FindField::Find => self.query_cursor = start,
                    FindField::Replace => self.replace_cursor = start,
                }
                self.clear_selection();
                return;
            }
        }
        self.prepare_move(sel);
        match self.active_field {
            FindField::Find => self.query_cursor = self.query_cursor.saturating_sub(1),
            FindField::Replace => self.replace_cursor = self.replace_cursor.saturating_sub(1),
        }
    }

    pub fn move_cursor_right(&mut self, sel: bool) {
        if !sel {
            let sel_end = match self.active_field {
                FindField::Find => self
                    .query_selection_anchor
                    .map(|a| a.max(self.query_cursor)),
                FindField::Replace => self
                    .replace_selection_anchor
                    .map(|a| a.max(self.replace_cursor)),
            };
            if let Some(end) = sel_end {
                match self.active_field {
                    FindField::Find => self.query_cursor = end,
                    FindField::Replace => self.replace_cursor = end,
                }
                self.clear_selection();
                return;
            }
        }
        self.prepare_move(sel);
        match self.active_field {
            FindField::Find => {
                let len = self.query.chars().count();
                if self.query_cursor < len {
                    self.query_cursor += 1;
                }
            }
            FindField::Replace => {
                let len = self.replace_text.chars().count();
                if self.replace_cursor < len {
                    self.replace_cursor += 1;
                }
            }
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
            self.clear_selection();
            self.active_field = FindField::Find;
            let len = self.query.chars().count();
            self.query_cursor = self.query_cursor.min(len);
        }
    }

    pub fn move_cursor_down(&mut self, sel: bool) {
        if sel {
            self.move_cursor_end(true);
        } else if self.is_replace && self.active_field == FindField::Find {
            self.clear_selection();
            self.active_field = FindField::Replace;
            let len = self.replace_text.chars().count();
            self.replace_cursor = self.replace_cursor.min(len);
        }
    }

    pub fn update_matches(&mut self, buffer: &EditorBuffer) {
        self.matches.clear();
        if self.query.is_empty() {
            self.active_match_idx = None;
            return;
        }
        let text = buffer.text();
        let compiled_re = if self.use_regex {
            compile_simple_regex(&self.query, self.match_case)
        } else {
            None
        };
        let q_chars: Vec<char> = if self.match_case {
            self.query.chars().collect()
        } else {
            self.query.to_lowercase().chars().collect()
        };
        let q_len = q_chars.len();
        let mut line_chars = Vec::with_capacity(128);

        for line_idx in 0..text.len_lines() {
            line_chars.clear();
            let line = text.line(line_idx);
            line_chars.extend(line.chars().take_while(|&c| c != '\n' && c != '\r'));
            let line_start_char = text.line_to_char(line_idx);
            let line_len = line_chars.len();

            if self.use_regex {
                if let Some(ref re) = compiled_re {
                    let mut col = 0;
                    while col <= line_len {
                        if let Some(m_len) = re.match_at(&line_chars, col) {
                            let match_len = m_len.max(1);
                            let is_valid = if self.whole_word {
                                is_word_boundary(&line_chars, col, col + match_len)
                            } else {
                                true
                            };
                            if is_valid {
                                let start = line_start_char + col;
                                let end = start + match_len;
                                self.matches.push((start, end));
                                col += match_len;
                                continue;
                            }
                        }
                        col += 1;
                    }
                }
            } else {
                if q_len == 0 || line_len < q_len {
                    continue;
                }
                let mut col = 0;
                while col + q_len <= line_len {
                    let matched = (0..q_len).all(|k| {
                        if self.match_case {
                            line_chars[col + k] == q_chars[k]
                        } else {
                            line_chars[col + k].to_ascii_lowercase() == q_chars[k]
                        }
                    });
                    if matched {
                        let is_valid = if self.whole_word {
                            is_word_boundary(&line_chars, col, col + q_len)
                        } else {
                            true
                        };
                        if is_valid {
                            let start = line_start_char + col;
                            let end = start + q_len;
                            self.matches.push((start, end));
                            col += q_len;
                            continue;
                        }
                    }
                    col += 1;
                }
            }
        }
        if self.matches.is_empty() {
            self.active_match_idx = None;
        } else {
            let cur = buffer.cursor_char;
            let closest = self
                .matches
                .iter()
                .position(|&(_, e)| cur <= e)
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
            Some(i) => (i + 1) % self.matches.len(),
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
            Some(i) => {
                if i == 0 {
                    self.matches.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.matches.len() - 1,
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
        if let Some(idx) = self.active_match_idx {
            if let Some(&(start, end)) = self.matches.get(idx) {
                buffer.replace_range(start, end, &self.replace_text);
                self.update_matches(buffer);
                if !self.matches.is_empty() {
                    self.active_match_idx = Some(idx.min(self.matches.len() - 1));
                    self.sync_view(buffer, vis_lines, vis_cols);
                }
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
        self.sync_view(buffer, vis_lines, vis_cols);
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn is_word_boundary(chars: &[char], start: usize, end: usize) -> bool {
    if start < end && !is_word_char(chars[start]) {
        return true;
    }
    let left_ok = if start > 0 {
        !is_word_char(chars[start - 1])
    } else {
        true
    };
    let right_ok = if end < chars.len() {
        !is_word_char(chars[end])
    } else {
        true
    };
    left_ok && right_ok
}

#[derive(Clone)]
enum Atom {
    Any,
    Literal(char),
    Digit,
    Word,
    Whitespace,
}

#[derive(Clone)]
enum PatternPiece {
    Single(Atom),
    ZeroOrMore(Atom),
    OneOrMore(Atom),
    ZeroOrOne(Atom),
}

struct CompiledRegex {
    pieces: Vec<PatternPiece>,
    match_case: bool,
}

impl CompiledRegex {
    fn match_at(&self, chars: &[char], start_col: usize) -> Option<usize> {
        let mut best = None;
        self.match_pieces(&self.pieces, chars, start_col, 0, &mut best);
        best
    }

    fn match_pieces(
        &self,
        pieces: &[PatternPiece],
        chars: &[char],
        char_idx: usize,
        matched_len: usize,
        best: &mut Option<usize>,
    ) {
        if pieces.is_empty() {
            if best.map_or(true, |b| matched_len > b) {
                *best = Some(matched_len);
            }
            return;
        }
        match &pieces[0] {
            PatternPiece::Single(atom) => {
                if char_idx < chars.len() && self.check_atom(atom, chars[char_idx]) {
                    self.match_pieces(&pieces[1..], chars, char_idx + 1, matched_len + 1, best);
                }
            }
            PatternPiece::ZeroOrOne(atom) => {
                if char_idx < chars.len() && self.check_atom(atom, chars[char_idx]) {
                    self.match_pieces(&pieces[1..], chars, char_idx + 1, matched_len + 1, best);
                }
                self.match_pieces(&pieces[1..], chars, char_idx, matched_len, best);
            }
            PatternPiece::ZeroOrMore(atom) => {
                let mut max_k = 0;
                while char_idx + max_k < chars.len()
                    && self.check_atom(atom, chars[char_idx + max_k])
                {
                    max_k += 1;
                }
                for k in (0..=max_k).rev() {
                    self.match_pieces(&pieces[1..], chars, char_idx + k, matched_len + k, best);
                    if best.is_some() {
                        break;
                    }
                }
            }
            PatternPiece::OneOrMore(atom) => {
                let mut max_k = 0;
                while char_idx + max_k < chars.len()
                    && self.check_atom(atom, chars[char_idx + max_k])
                {
                    max_k += 1;
                }
                for k in (1..=max_k).rev() {
                    self.match_pieces(&pieces[1..], chars, char_idx + k, matched_len + k, best);
                    if best.is_some() {
                        break;
                    }
                }
            }
        }
    }

    fn check_atom(&self, atom: &Atom, ch: char) -> bool {
        match atom {
            Atom::Any => true,
            Atom::Literal(c) => {
                if self.match_case {
                    *c == ch
                } else {
                    c.to_ascii_lowercase() == ch.to_ascii_lowercase()
                }
            }
            Atom::Digit => ch.is_ascii_digit(),
            Atom::Word => is_word_char(ch),
            Atom::Whitespace => ch.is_whitespace(),
        }
    }
}

fn compile_simple_regex(pat: &str, match_case: bool) -> Option<CompiledRegex> {
    let chars: Vec<char> = pat.chars().collect();
    let mut pieces = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let atom = match chars[i] {
            '.' => Atom::Any,
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    return None;
                }
                match chars[i] {
                    'd' => Atom::Digit,
                    'w' => Atom::Word,
                    's' => Atom::Whitespace,
                    c => Atom::Literal(c),
                }
            }
            c => Atom::Literal(c),
        };
        i += 1;
        let piece = if i < chars.len() {
            match chars[i] {
                '*' => {
                    i += 1;
                    PatternPiece::ZeroOrMore(atom)
                }
                '+' => {
                    i += 1;
                    PatternPiece::OneOrMore(atom)
                }
                '?' => {
                    i += 1;
                    PatternPiece::ZeroOrOne(atom)
                }
                _ => PatternPiece::Single(atom),
            }
        } else {
            PatternPiece::Single(atom)
        };
        pieces.push(piece);
    }
    Some(CompiledRegex { pieces, match_case })
}
