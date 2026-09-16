use super::EditorBuffer;
use crate::config::TAB_WIDTH;
use ropey::RopeSlice;

pub fn char_visual_width(ch: char, current_vcol: usize) -> usize {
    if ch == '\t' {
        TAB_WIDTH - (current_vcol % TAB_WIDTH)
    } else {
        1
    }
}

pub fn line_visual_len(slice: &RopeSlice) -> usize {
    let mut vcol = 0;
    for ch in slice.chars().take_while(|&c| c != '\n' && c != '\r') {
        vcol += char_visual_width(ch, vcol);
    }
    vcol
}

pub fn char_to_visual_col(slice: &RopeSlice, char_offset: usize) -> usize {
    let mut vcol = 0;
    for (i, ch) in slice
        .chars()
        .take_while(|&c| c != '\n' && c != '\r')
        .enumerate()
    {
        if i >= char_offset {
            break;
        }
        vcol += char_visual_width(ch, vcol);
    }
    vcol
}

pub fn visual_col_to_char(slice: &RopeSlice, target_vcol: usize) -> usize {
    let mut vcol = 0;
    let mut char_idx = 0;
    for ch in slice.chars().take_while(|&c| c != '\n' && c != '\r') {
        let w = char_visual_width(ch, vcol);
        if target_vcol < vcol + w {
            if target_vcol >= vcol + (w / 2) && w > 1 {
                return char_idx + 1;
            }
            return char_idx;
        }
        vcol += w;
        char_idx += 1;
    }
    char_idx
}

impl EditorBuffer {
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
        if !sel {
            if let Some((start, _)) = self.selection_range() {
                self.cursor_char = start;
                self.selection_anchor = None;
                return;
            }
        }
        self.prepare_move(sel);
        self.cursor_char = self.cursor_char.saturating_sub(1);
    }

    pub fn move_right(&mut self, sel: bool) {
        if !sel {
            if let Some((_, end)) = self.selection_range() {
                self.cursor_char = end;
                self.selection_anchor = None;
                return;
            }
        }
        self.prepare_move(sel);
        if self.cursor_char < self.text.len_chars() {
            self.cursor_char += 1;
        }
    }

    pub fn move_up(&mut self, sel: bool) {
        if !sel {
            if let Some((start, _)) = self.selection_range() {
                self.cursor_char = start;
                self.selection_anchor = None;
            }
        }
        self.prepare_move(sel);
        let (line, _) = self.cursor_pos();
        let cur_vcol = self.visual_cursor_col();
        if line > 0 {
            let target = line - 1;
            let line_slice = self.text.line(target);
            let char_offset = visual_col_to_char(&line_slice, cur_vcol);
            self.cursor_char = self.text.line_to_char(target) + char_offset;
        } else if sel {
            self.cursor_char = 0;
        }
    }

    pub fn move_down(&mut self, sel: bool) {
        if !sel {
            if let Some((_, end)) = self.selection_range() {
                self.cursor_char = end;
                self.selection_anchor = None;
            }
        }
        self.prepare_move(sel);
        let (line, _) = self.cursor_pos();
        let cur_vcol = self.visual_cursor_col();
        if line + 1 < self.text.len_lines() {
            let target = line + 1;
            let line_slice = self.text.line(target);
            let char_offset = visual_col_to_char(&line_slice, cur_vcol);
            self.cursor_char = self.text.line_to_char(target) + char_offset;
        } else if sel {
            self.cursor_char = self.text.len_chars();
        }
    }

    pub fn move_home(&mut self, sel: bool) {
        self.prepare_move(sel);
        let (line, _) = self.cursor_pos();
        self.cursor_char = self.text.line_to_char(line);
    }

    pub fn move_end(&mut self, sel: bool) {
        self.prepare_move(sel);
        let (line, _) = self.cursor_pos();
        let line_len = self.line_len(line);
        self.cursor_char = self.text.line_to_char(line) + line_len;
    }

    pub fn set_cursor_at_visual(&mut self, target_line: usize, target_vcol: usize) {
        let total_lines = self.text.len_lines();
        if total_lines == 0 {
            self.cursor_char = 0;
            return;
        }
        let line = target_line.min(total_lines - 1);
        let line_slice = self.text.line(line);
        let char_offset = visual_col_to_char(&line_slice, target_vcol);
        self.cursor_char = self.text.line_to_char(line) + char_offset;
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        if self.text.len_chars() == 0 {
            return (0, 0);
        }
        let line = self.text.char_to_line(self.cursor_char);
        let col = self.cursor_char - self.text.line_to_char(line);
        (line, col)
    }

    pub fn visual_cursor_col(&self) -> usize {
        if self.text.len_chars() == 0 {
            return 0;
        }
        let (line, col) = self.cursor_pos();
        let line_slice = self.text.line(line);
        char_to_visual_col(&line_slice, col)
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

    pub fn fit_view(&mut self, vis_lines: usize, vis_cols: usize) {
        let (line, _) = self.cursor_pos();
        let vcol = self.visual_cursor_col();
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
            if vcol < self.scroll_col {
                self.scroll_col = vcol;
            } else if vcol >= self.scroll_col + vis_cols {
                self.scroll_col = vcol - vis_cols + 1;
            }
            let max_scroll_col = self.max_line_len.saturating_sub(vis_cols);
            if self.scroll_col > max_scroll_col {
                self.scroll_col = max_scroll_col;
            }
        }
    }
}
