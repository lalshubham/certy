mod cursor;
mod mutation;
mod selection;
mod transform;

use crate::editor::history::{EditAction, History};
use ropey::Rope;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub struct EditorBuffer {
    pub(crate) text: Rope,
    pub file_path: Option<PathBuf>,
    pub is_modified: bool,
    pub cursor_char: usize,
    pub selection_anchor: Option<usize>,
    pub scroll_line: usize,
    pub scroll_col: usize,
    pub max_line_len: usize,
    pub(crate) history: History,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self {
            text: Rope::new(),
            file_path: None,
            is_modified: false,
            cursor_char: 0,
            selection_anchor: None,
            scroll_line: 0,
            scroll_col: 0,
            max_line_len: 0,
            history: History::new(),
        }
    }

    pub fn load_file(&mut self, path: PathBuf) -> io::Result<()> {
        let file = File::open(&path)?;
        let text = Rope::from_reader(BufReader::new(file))?;
        self.text = text;
        self.file_path = Some(path);
        self.is_modified = false;
        self.cursor_char = 0;
        self.selection_anchor = None;
        self.scroll_line = 0;
        self.scroll_col = 0;
        self.history = History::new();
        self.recompute_max_line_len();
        Ok(())
    }

    pub fn load_recovered(&mut self, path: PathBuf, recovery_path: &Path) -> io::Result<()> {
        let file = File::open(recovery_path)?;
        let text = Rope::from_reader(BufReader::new(file))?;
        self.text = text;
        self.file_path = Some(path);
        self.is_modified = true;
        self.cursor_char = 0;
        self.selection_anchor = None;
        self.scroll_line = 0;
        self.scroll_col = 0;
        self.history = History::new();
        self.recompute_max_line_len();
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.file_path {
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            for chunk in self.text.chunks() {
                writer.write_all(chunk.as_bytes())?;
            }
            writer.flush()?;
            self.is_modified = false;
        }
        Ok(())
    }

    pub fn text(&self) -> &Rope {
        &self.text
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.history.pop_undo() {
            match action {
                EditAction::Insert { char_idx, text } => {
                    let count = text.chars().count();
                    self.text.remove(char_idx..char_idx + count);
                    self.cursor_char = char_idx;
                    self.history
                        .push_redo(EditAction::Insert { char_idx, text });
                }
                EditAction::Delete { char_idx, text } => {
                    self.text.insert(char_idx, &text);
                    self.cursor_char = char_idx + text.chars().count();
                    self.history
                        .push_redo(EditAction::Delete { char_idx, text });
                }
            }
            self.selection_anchor = None;
            self.is_modified = true;
            self.recompute_max_line_len();
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.history.pop_redo() {
            match action {
                EditAction::Insert { char_idx, text } => {
                    self.text.insert(char_idx, &text);
                    self.cursor_char = char_idx + text.chars().count();
                    self.history
                        .push_undo(EditAction::Insert { char_idx, text });
                }
                EditAction::Delete { char_idx, text } => {
                    let count = text.chars().count();
                    self.text.remove(char_idx..char_idx + count);
                    self.cursor_char = char_idx;
                    self.history
                        .push_undo(EditAction::Delete { char_idx, text });
                }
            }
            self.selection_anchor = None;
            self.is_modified = true;
            self.recompute_max_line_len();
        }
    }

    pub(crate) fn recompute_max_line_len(&mut self) {
        self.max_line_len = self
            .text
            .lines()
            .map(|slice| {
                let mut len = slice.len_chars();
                while len > 0 && matches!(slice.char(len - 1), '\n' | '\r') {
                    len -= 1;
                }
                len
            })
            .max()
            .unwrap_or(0);
    }
}
