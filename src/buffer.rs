use ropey::Rope;

pub struct EditorBuffer {
    text: Rope,
    cursor_char: usize,
    pub scroll_line: usize,
    pub scroll_col: usize,
    pub max_line_len: usize,
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            cursor_char: 0,
            scroll_line: 0,
            scroll_col: 0,
            max_line_len: 0,
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.text.insert_char(self.cursor_char, ch);
        self.cursor_char += 1;
        self.recompute_max_line_len();
    }

    pub fn delete_backwards(&mut self) {
        if self.cursor_char > 0 {
            self.cursor_char -= 1;
            self.text.remove(self.cursor_char..self.cursor_char + 1);
            self.recompute_max_line_len();
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_char < self.text.len_chars() {
            self.text.remove(self.cursor_char..self.cursor_char + 1);
            self.recompute_max_line_len();
        }
    }

    pub fn move_left(&mut self) {
        self.cursor_char = self.cursor_char.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        if self.cursor_char < self.text.len_chars() {
            self.cursor_char += 1;
        }
    }

    pub fn move_up(&mut self) {
        let (line, col) = self.cursor_pos();
        if line > 0 {
            let target_line = line - 1;
            let line_len = self.line_len(target_line);
            self.cursor_char = self.text.line_to_char(target_line) + col.min(line_len);
        }
    }

    pub fn move_down(&mut self) {
        let (line, col) = self.cursor_pos();
        if line + 1 < self.text.len_lines() {
            let target_line = line + 1;
            let line_len = self.line_len(target_line);
            self.cursor_char = self.text.line_to_char(target_line) + col.min(line_len);
        }
    }

    pub fn set_cursor_at(&mut self, target_line: usize, target_col: usize) {
        let total_lines = self.text.len_lines();
        if total_lines == 0 {
            self.cursor_char = 0;
            return;
        }
        let clamped_line = target_line.min(total_lines - 1);
        let max_line_col = self.line_len(clamped_line);
        let clamped_col = target_col.min(max_line_col);
        self.cursor_char = self.text.line_to_char(clamped_line) + clamped_col;
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
        if len > 0 && slice.char(len - 1) == '\n' {
            len -= 1;
            if len > 0 && slice.char(len - 1) == '\r' {
                len -= 1;
            }
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
