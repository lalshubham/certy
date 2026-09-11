use std::collections::VecDeque;

pub const ANSI_COLORS: [u32; 16] = [
    0xFF6E7681, 0xFFFF4D4D, 0xFF2EE59D, 0xFFFFDD00, 0xFF4C9EFF, 0xFFFF55D4, 0xFF00E5FF, 0xFFE6EDF3,
    0xFF8B949E, 0xFFFF7B72, 0xFF56F39A, 0xFFFFF066, 0xFF79C0FF, 0xFFFFA8EC, 0xFF56FFFF, 0xFFFFFFFF,
];

pub const COLOR_TERMINAL_FG: u32 = ANSI_COLORS[15];

pub fn ansi_256_to_u32(idx: u8) -> u32 {
    if (idx as usize) < 16 {
        return ANSI_COLORS[idx as usize];
    }
    if idx >= 232 {
        let v = 8 + (idx - 232) * 10;
        return 0xFF000000 | ((v as u32) << 16) | ((v as u32) << 8) | (v as u32);
    }
    let idx = idx - 16;
    let b = (idx % 6) * 51;
    let g = ((idx / 6) % 6) * 51;
    let r = (idx / 36) * 51;
    0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[derive(Clone, Copy)]
pub struct TerminalCell {
    pub ch: char,
    pub fg: u32,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: COLOR_TERMINAL_FG,
        }
    }
}

#[derive(Clone)]
pub struct TerminalRow {
    pub cells: Vec<TerminalCell>,
}

impl TerminalRow {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![TerminalCell::default(); cols],
        }
    }

    pub fn resize(&mut self, new_cols: usize) {
        self.cells.resize(new_cols, TerminalCell::default());
    }
}

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
    saved_cursor: (usize, usize),
    alt_saved_cursor: (usize, usize),
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

    fn current_rows_mut(&mut self) -> &mut Vec<TerminalRow> {
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

    fn put_char(&mut self, ch: char) {
        if self.wrap_next {
            self.wrap_next = false;
            self.cursor_col = 0;
            let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
            if self.cursor_row == bottom {
                self.scroll_up_region(1);
            } else if self.cursor_row < self.rows.saturating_sub(1) {
                self.cursor_row += 1;
            }
        }
        let fg = self.current_fg;
        let r = self.cursor_row;
        let c = self.cursor_col;
        let rows = self.current_rows_mut();
        if r < rows.len() && c < rows[r].cells.len() {
            rows[r].cells[c] = TerminalCell { ch, fg };
        }
        if self.cursor_col + 1 >= self.cols {
            self.wrap_next = true;
        } else {
            self.cursor_col += 1;
        }
    }

    pub fn process_bytes(&mut self, bytes: &[u8]) {
        let mut i = 0;
        let len = bytes.len();
        while i < len {
            let b = bytes[i];
            if b == b'\x1b' {
                self.wrap_next = false;
                i += 1;
                if i >= len {
                    break;
                }
                match bytes[i] {
                    b'[' => {
                        i += 1;
                        let mut param_str = String::new();
                        while i < len && (0x20..=0x3F).contains(&bytes[i]) {
                            param_str.push(bytes[i] as char);
                            i += 1;
                        }
                        if i < len {
                            let cmd = bytes[i];
                            i += 1;
                            self.handle_csi(&param_str, cmd);
                        }
                    }
                    b']' => {
                        i += 1;
                        while i < len && bytes[i] != 0x07 {
                            if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'\\' {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                        if i < len && bytes[i] == 0x07 {
                            i += 1;
                        }
                    }
                    b'(' | b')' | b'*' | b'+' | b'-' | b'.' | b'/' => {
                        i += 1;
                        if i < len {
                            i += 1;
                        }
                    }
                    b'=' | b'>' => {
                        i += 1;
                    }
                    b'D' => {
                        i += 1;
                        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
                        if self.cursor_row == bottom {
                            self.scroll_up_region(1);
                        } else if self.cursor_row < self.rows.saturating_sub(1) {
                            self.cursor_row += 1;
                        }
                    }
                    b'E' => {
                        i += 1;
                        self.cursor_col = 0;
                        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
                        if self.cursor_row == bottom {
                            self.scroll_up_region(1);
                        } else if self.cursor_row < self.rows.saturating_sub(1) {
                            self.cursor_row += 1;
                        }
                    }
                    b'M' => {
                        i += 1;
                        if self.cursor_row == self.scroll_top {
                            self.scroll_down_region(1);
                        } else if self.cursor_row > 0 {
                            self.cursor_row -= 1;
                        }
                    }
                    b'7' => {
                        i += 1;
                        self.saved_cursor = (self.cursor_row, self.cursor_col);
                    }
                    b'8' => {
                        i += 1;
                        self.cursor_row = self.saved_cursor.0.min(self.rows.saturating_sub(1));
                        self.cursor_col = self.saved_cursor.1.min(self.cols.saturating_sub(1));
                    }
                    b'c' => {
                        i += 1;
                        self.cursor_row = 0;
                        self.cursor_col = 0;
                        self.scroll_top = 0;
                        self.scroll_bottom = self.rows.saturating_sub(1);
                        self.current_fg = COLOR_TERMINAL_FG;
                        let c = self.cols;
                        for r in self.current_rows_mut() {
                            *r = TerminalRow::new(c);
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
                continue;
            }
            match b {
                b'\r' => {
                    self.wrap_next = false;
                    self.cursor_col = 0;
                    i += 1;
                }
                b'\n' => {
                    self.wrap_next = false;
                    let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
                    if self.cursor_row == bottom {
                        self.scroll_up_region(1);
                    } else if self.cursor_row < self.rows.saturating_sub(1) {
                        self.cursor_row += 1;
                    }
                    i += 1;
                }
                0x08 => {
                    self.wrap_next = false;
                    self.cursor_col = self.cursor_col.saturating_sub(1);
                    i += 1;
                }
                b'\t' => {
                    self.wrap_next = false;
                    let next_tab = (self.cursor_col + 8) & !7;
                    self.cursor_col = next_tab.min(self.cols.saturating_sub(1));
                    i += 1;
                }
                0x07 => {
                    i += 1;
                }
                _ => {
                    if let Ok(s) = std::str::from_utf8(&bytes[i..len]) {
                        if let Some(ch) = s.chars().next() {
                            i += ch.len_utf8();
                            if !ch.is_control() {
                                self.put_char(ch);
                            }
                            continue;
                        }
                    }
                    if !b.is_ascii_control() {
                        self.put_char(b as char);
                    }
                    i += 1;
                }
            }
        }
    }

    fn handle_csi(&mut self, params: &str, cmd: u8) {
        self.wrap_next = false;
        if params.starts_with('?') {
            let mode = &params[1..];
            match (mode, cmd) {
                ("1049", b'h') => {
                    self.alt_saved_cursor = (self.cursor_row, self.cursor_col);
                    self.is_alt = true;
                    let c = self.cols;
                    for r in &mut self.alt_grid {
                        *r = TerminalRow::new(c);
                    }
                    self.cursor_row = 0;
                    self.cursor_col = 0;
                    self.scroll_top = 0;
                    self.scroll_bottom = self.rows.saturating_sub(1);
                }
                ("1049", b'l') => {
                    self.is_alt = false;
                    self.cursor_row = self.alt_saved_cursor.0.min(self.rows.saturating_sub(1));
                    self.cursor_col = self.alt_saved_cursor.1.min(self.cols.saturating_sub(1));
                    self.scroll_top = 0;
                    self.scroll_bottom = self.rows.saturating_sub(1);
                }
                ("47", b'h') | ("1047", b'h') => {
                    self.is_alt = true;
                    let c = self.cols;
                    for r in &mut self.alt_grid {
                        *r = TerminalRow::new(c);
                    }
                    self.scroll_top = 0;
                    self.scroll_bottom = self.rows.saturating_sub(1);
                }
                ("47", b'l') | ("1047", b'l') => {
                    self.is_alt = false;
                    self.scroll_top = 0;
                    self.scroll_bottom = self.rows.saturating_sub(1);
                }
                ("1048", b'h') => {
                    self.saved_cursor = (self.cursor_row, self.cursor_col);
                }
                ("1048", b'l') => {
                    self.cursor_row = self.saved_cursor.0.min(self.rows.saturating_sub(1));
                    self.cursor_col = self.saved_cursor.1.min(self.cols.saturating_sub(1));
                }
                ("25", b'h') => self.cursor_visible = true,
                ("25", b'l') => self.cursor_visible = false,
                _ => {}
            }
            return;
        }
        let parts: Vec<usize> = params
            .split(';')
            .filter_map(|s| s.parse::<usize>().ok())
            .collect();
        match cmd {
            b'm' => self.handle_sgr(&parts),
            b's' => {
                self.saved_cursor = (self.cursor_row, self.cursor_col);
            }
            b'u' => {
                self.cursor_row = self.saved_cursor.0.min(self.rows.saturating_sub(1));
                self.cursor_col = self.saved_cursor.1.min(self.cols.saturating_sub(1));
            }
            b'H' | b'f' => {
                let r = parts.first().copied().unwrap_or(1).max(1);
                let c = parts.get(1).copied().unwrap_or(1).max(1);
                self.cursor_row = (r - 1).min(self.rows.saturating_sub(1));
                self.cursor_col = (c - 1).min(self.cols.saturating_sub(1));
            }
            b'A' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            b'B' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
            }
            b'C' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                self.cursor_col = (self.cursor_col + n).min(self.cols.saturating_sub(1));
            }
            b'D' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            b'G' => {
                let c = parts.first().copied().unwrap_or(1).max(1);
                self.cursor_col = (c - 1).min(self.cols.saturating_sub(1));
            }
            b'd' => {
                let r = parts.first().copied().unwrap_or(1).max(1);
                self.cursor_row = (r - 1).min(self.rows.saturating_sub(1));
            }
            b'L' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                let r = self.cursor_row;
                let top = self.scroll_top;
                let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
                if r >= top && r <= bottom {
                    let c = self.cols;
                    let rows = self.current_rows_mut();
                    for _ in 0..n {
                        if bottom < rows.len() {
                            rows.remove(bottom);
                            rows.insert(r, TerminalRow::new(c));
                        }
                    }
                }
            }
            b'M' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                let r = self.cursor_row;
                let top = self.scroll_top;
                let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
                if r >= top && r <= bottom {
                    let c = self.cols;
                    let rows = self.current_rows_mut();
                    for _ in 0..n {
                        if r < rows.len() && bottom < rows.len() {
                            rows.remove(r);
                            rows.insert(bottom, TerminalRow::new(c));
                        }
                    }
                }
            }
            b'P' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                let r = self.cursor_row;
                let c = self.cursor_col;
                let rows = self.current_rows_mut();
                if r < rows.len() && c < rows[r].cells.len() {
                    for _ in 0..n {
                        if c < rows[r].cells.len() {
                            rows[r].cells.remove(c);
                            rows[r].cells.push(TerminalCell::default());
                        }
                    }
                }
            }
            b'@' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                let r = self.cursor_row;
                let c = self.cursor_col;
                let rows = self.current_rows_mut();
                if r < rows.len() && c < rows[r].cells.len() {
                    for _ in 0..n {
                        rows[r].cells.insert(c, TerminalCell::default());
                        rows[r].cells.pop();
                    }
                }
            }
            b'X' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                let r = self.cursor_row;
                let c = self.cursor_col;
                let rows = self.current_rows_mut();
                if r < rows.len() {
                    for col in c..(c + n).min(rows[r].cells.len()) {
                        rows[r].cells[col] = TerminalCell::default();
                    }
                }
            }
            b'S' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                self.scroll_up_region(n);
            }
            b'T' => {
                let n = parts.first().copied().unwrap_or(1).max(1);
                self.scroll_down_region(n);
            }
            b'r' => {
                let top = parts.first().copied().unwrap_or(1).max(1).saturating_sub(1);
                let bottom = parts
                    .get(1)
                    .copied()
                    .unwrap_or(self.rows)
                    .min(self.rows)
                    .max(1)
                    .saturating_sub(1);
                if top < bottom && bottom < self.rows {
                    self.scroll_top = top;
                    self.scroll_bottom = bottom;
                } else {
                    self.scroll_top = 0;
                    self.scroll_bottom = self.rows.saturating_sub(1);
                }
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            b'J' => {
                let mode = parts.first().copied().unwrap_or(0);
                let r = self.cursor_row;
                let c = self.cursor_col;
                let rows = self.current_rows_mut();
                match mode {
                    0 => {
                        if r < rows.len() {
                            for col in c..rows[r].cells.len() {
                                rows[r].cells[col] = TerminalCell::default();
                            }
                            for row in (r + 1)..rows.len() {
                                for cell in &mut rows[row].cells {
                                    *cell = TerminalCell::default();
                                }
                            }
                        }
                    }
                    1 => {
                        for row in 0..r.min(rows.len()) {
                            for cell in &mut rows[row].cells {
                                *cell = TerminalCell::default();
                            }
                        }
                        if r < rows.len() {
                            for col in 0..=c.min(rows[r].cells.len().saturating_sub(1)) {
                                rows[r].cells[col] = TerminalCell::default();
                            }
                        }
                    }
                    2 | 3 => {
                        for row in rows {
                            for cell in &mut row.cells {
                                *cell = TerminalCell::default();
                            }
                        }
                    }
                    _ => {}
                }
            }
            b'K' => {
                let mode = parts.first().copied().unwrap_or(0);
                let r = self.cursor_row;
                let c = self.cursor_col;
                let rows = self.current_rows_mut();
                if r < rows.len() {
                    match mode {
                        0 => {
                            for col in c..rows[r].cells.len() {
                                rows[r].cells[col] = TerminalCell::default();
                            }
                        }
                        1 => {
                            for col in 0..=c.min(rows[r].cells.len().saturating_sub(1)) {
                                rows[r].cells[col] = TerminalCell::default();
                            }
                        }
                        2 => {
                            for cell in &mut rows[r].cells {
                                *cell = TerminalCell::default();
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_sgr(&mut self, parts: &[usize]) {
        if parts.is_empty() {
            self.current_fg = COLOR_TERMINAL_FG;
            return;
        }
        let is_bold = parts.contains(&1);
        let mut idx = 0;
        while idx < parts.len() {
            match parts[idx] {
                0 => self.current_fg = COLOR_TERMINAL_FG,
                30..=37 => {
                    let mut c = parts[idx] - 30;
                    if is_bold {
                        c += 8;
                    }
                    self.current_fg = ANSI_COLORS[c];
                }
                39 => self.current_fg = COLOR_TERMINAL_FG,
                90..=97 => {
                    let c = parts[idx] - 90 + 8;
                    self.current_fg = ANSI_COLORS[c];
                }
                38 => {
                    if idx + 2 < parts.len() && parts[idx + 1] == 5 {
                        let color_idx = (parts[idx + 2] as u8).min(255);
                        self.current_fg = ansi_256_to_u32(color_idx);
                        idx += 2;
                    } else if idx + 4 < parts.len() && parts[idx + 1] == 2 {
                        let r = (parts[idx + 2] as u32).min(255);
                        let g = (parts[idx + 3] as u32).min(255);
                        let b = (parts[idx + 4] as u32).min(255);
                        self.current_fg = 0xFF000000 | (r << 16) | (g << 8) | b;
                        idx += 4;
                    }
                }
                _ => {}
            }
            idx += 1;
        }
    }
}
