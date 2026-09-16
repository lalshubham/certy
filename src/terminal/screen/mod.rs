pub mod cell;
pub mod color;
mod parser;

pub use cell::TerminalRow;
use color::COLOR_TERMINAL_FG;

use std::collections::VecDeque;

pub struct TerminalScreen {
    pub rows: usize,
    pub cols: usize,
    pub grid: Vec<TerminalRow>,
    pub alt_grid: Vec<TerminalRow>,
    pub scrollback: VecDeque<TerminalRow>,
    pub is_alt: bool,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_visible: bool,
    pub current_fg: u32,
    pub wrap_next: bool,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    pub(crate) saved_cursor: (usize, usize),
    pub(crate) alt_saved_cursor: (usize, usize),
}

impl TerminalScreen {
    pub fn new(rows: usize, cols: usize) -> Self {
        let r = rows.max(1);
        let c = cols.max(1);
        Self {
            rows: r,
            cols: c,
            grid: vec![TerminalRow::new(c); r],
            alt_grid: vec![TerminalRow::new(c); r],
            scrollback: VecDeque::with_capacity(512),
            is_alt: false,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            current_fg: COLOR_TERMINAL_FG,
            wrap_next: false,
            scroll_top: 0,
            scroll_bottom: r.saturating_sub(1),
            saved_cursor: (0, 0),
            alt_saved_cursor: (0, 0),
        }
    }

    pub fn set_size(&mut self, new_rows: usize, new_cols: usize) {
        let r = new_rows.max(1);
        let c = new_cols.max(1);
        self.rows = r;
        self.cols = c;
        self.grid.resize(r, TerminalRow::new(c));
        for row in &mut self.grid {
            row.resize(c);
        }
        self.alt_grid.resize(r, TerminalRow::new(c));
        for row in &mut self.alt_grid {
            row.resize(c);
        }
        self.cursor_row = self.cursor_row.min(r.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(c.saturating_sub(1));
        self.scroll_top = 0;
        self.scroll_bottom = r.saturating_sub(1);
        self.wrap_next = false;
    }

    pub(crate) fn current_rows_mut(&mut self) -> &mut Vec<TerminalRow> {
        if self.is_alt {
            &mut self.alt_grid
        } else {
            &mut self.grid
        }
    }

    pub fn total_lines(&self) -> usize {
        if self.is_alt {
            self.rows
        } else {
            self.scrollback.len() + self.rows
        }
    }

    pub fn get_display_row(&self, line_idx: usize) -> Option<&TerminalRow> {
        if self.is_alt {
            self.alt_grid.get(line_idx)
        } else if line_idx < self.scrollback.len() {
            self.scrollback.get(line_idx)
        } else {
            let rel = line_idx - self.scrollback.len();
            self.grid.get(rel)
        }
    }

    pub fn scroll_up_region(&mut self, n: usize) {
        let top = self.scroll_top;
        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
        if top >= bottom {
            return;
        }
        let c = self.cols;
        let is_full_screen = top == 0 && bottom == self.rows.saturating_sub(1);
        for _ in 0..n {
            if self.is_alt {
                if top < self.alt_grid.len() && bottom < self.alt_grid.len() {
                    self.alt_grid.remove(top);
                    self.alt_grid.insert(bottom, TerminalRow::new(c));
                }
            } else if top < self.grid.len() && bottom < self.grid.len() {
                let removed = self.grid.remove(top);
                if is_full_screen {
                    if self.scrollback.len() >= 5000 {
                        self.scrollback.pop_front();
                    }
                    self.scrollback.push_back(removed);
                }
                self.grid.insert(bottom, TerminalRow::new(c));
            }
        }
    }

    pub fn scroll_down_region(&mut self, n: usize) {
        let top = self.scroll_top;
        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
        if top >= bottom {
            return;
        }
        let c = self.cols;
        let rows = self.current_rows_mut();
        for _ in 0..n {
            if top < rows.len() && bottom < rows.len() {
                rows.remove(bottom);
                rows.insert(top, TerminalRow::new(c));
            }
        }
    }
}
