use super::color::COLOR_TERMINAL_FG;

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
