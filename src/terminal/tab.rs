use super::process::TermMsg;
use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TerminalLine {
    pub prompt: Option<String>,
    pub content: String,
}

impl TerminalLine {
    pub fn full_text(&self) -> String {
        if let Some(ref p) = self.prompt {
            format!("{p}{}", self.content)
        } else {
            self.content.clone()
        }
    }
}

pub fn get_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

pub fn get_hostname() -> String {
    if let Ok(h) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string())
}

pub struct TerminalTab {
    pub name: String,
    pub lines: Vec<TerminalLine>,
    pub partial_line: String,
    pub current_input: String,
    pub cursor_col: usize,
    pub cwd: PathBuf,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub scroll_line: usize,
    pub scroll_col: usize,
    pub max_line_len: usize,
    pub is_running: bool,
    pub command_finished: bool,
    pub selection_anchor: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub(crate) output_rx: Receiver<TermMsg>,
    pub(crate) output_tx: Sender<TermMsg>,
    pub(crate) running_child: Arc<Mutex<Option<Child>>>,
}

impl TerminalTab {
    pub fn new(name: String, cwd: PathBuf) -> Self {
        let (output_tx, output_rx) = channel();
        Self {
            name,
            lines: Vec::new(),
            partial_line: String::new(),
            current_input: String::new(),
            cursor_col: 0,
            cwd,
            history: Vec::new(),
            history_idx: None,
            scroll_line: 0,
            scroll_col: 0,
            max_line_len: 0,
            is_running: false,
            command_finished: false,
            selection_anchor: None,
            selection_end: None,
            output_rx,
            output_tx,
            running_child: Arc::new(Mutex::new(None)),
        }
    }

    #[inline]
    pub fn width(&self, char_w: usize) -> usize {
        self.name.chars().count() * char_w + 34
    }

    pub fn prompt(&self) -> String {
        let user = get_username();
        let host = get_hostname();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path_str = if let Some(ref h) = home {
            if let Ok(rel) = self.cwd.strip_prefix(h) {
                if rel.as_os_str().is_empty() {
                    "~".to_string()
                } else {
                    format!("~/{}", rel.display())
                }
            } else {
                self.cwd.display().to_string()
            }
        } else {
            self.cwd.display().to_string()
        };
        format!("{user}@{host}:{path_str}$ ")
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len() + 1
    }

    pub fn max_content_cols(&self) -> usize {
        let mut max_c = self.max_line_len;
        if self.is_running {
            let running_len = self.partial_line.chars().count();
            if running_len > max_c {
                max_c = running_len;
            }
        } else {
            let active_len = self.prompt().chars().count() + self.current_input.chars().count();
            if active_len > max_c {
                max_c = active_len;
            }
        }
        max_c
    }

    pub fn ensure_cursor_visible(&mut self, vis_cols: usize) {
        if vis_cols == 0 {
            return;
        }
        let prompt_len = self.prompt().chars().count();
        let active_col = prompt_len + self.cursor_col;
        let active_line_len = prompt_len + self.current_input.chars().count();

        if active_col < vis_cols {
            self.scroll_col = 0;
        } else {
            if active_col >= self.scroll_col + vis_cols {
                self.scroll_col = active_col.saturating_sub(vis_cols) + 1;
            } else if active_col < self.scroll_col {
                self.scroll_col = active_col;
            }

            if self.cursor_col == self.current_input.chars().count() {
                let ideal_scroll = active_line_len.saturating_sub(vis_cols) + 1;
                if self.scroll_col > ideal_scroll {
                    self.scroll_col = ideal_scroll;
                }
            }
        }

        let max_scroll = self.max_content_cols().saturating_sub(vis_cols);
        if self.scroll_col > max_scroll {
            self.scroll_col = max_scroll;
        }
    }

    pub fn auto_scroll_to_bottom(&mut self, vis_rows: usize) {
        let total = self.total_lines();
        if total > vis_rows {
            self.scroll_line = total - vis_rows;
        } else {
            self.scroll_line = 0;
        }
    }

    pub fn get_line_text(&self, idx: usize) -> Option<String> {
        if idx < self.lines.len() {
            Some(self.lines[idx].full_text())
        } else if idx == self.lines.len() {
            if self.is_running {
                Some(self.partial_line.clone())
            } else {
                Some(format!("{}{}", self.prompt(), self.current_input))
            }
        } else {
            None
        }
    }

    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        match (self.selection_anchor, self.selection_end) {
            (Some(a), Some(e)) if a != e => {
                if a <= e {
                    Some((a, e))
                } else {
                    Some((e, a))
                }
            }
            _ => None,
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let ((start_line, start_col), (end_line, end_col)) = self.selection_range()?;
        let mut result = Vec::new();
        for line_idx in start_line..=end_line {
            if let Some(text) = self.get_line_text(line_idx) {
                let chars: Vec<char> = text.chars().collect();
                let s_col = if line_idx == start_line {
                    start_col.min(chars.len())
                } else {
                    0
                };
                let e_col = if line_idx == end_line {
                    end_col.min(chars.len())
                } else {
                    chars.len()
                };
                if s_col <= e_col {
                    let slice: String = chars[s_col..e_col].iter().collect();
                    result.push(slice);
                } else {
                    result.push(String::new());
                }
            }
        }
        Some(result.join("\n"))
    }

    pub fn history_up(&mut self) {
        if !self.history.is_empty() {
            let next = match self.history_idx {
                Some(i) if i > 0 => i - 1,
                Some(_) => 0,
                None => self.history.len().saturating_sub(1),
            };
            self.history_idx = Some(next);
            self.current_input = self.history[next].clone();
            self.cursor_col = self.current_input.chars().count();
        }
    }

    pub fn history_down(&mut self) {
        if let Some(i) = self.history_idx {
            if i + 1 < self.history.len() {
                self.history_idx = Some(i + 1);
                self.current_input = self.history[i + 1].clone();
            } else {
                self.history_idx = None;
                self.current_input.clear();
            }
            self.cursor_col = self.current_input.chars().count();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let mut chars: Vec<char> = self.current_input.chars().collect();
        chars.insert(self.cursor_col, ch);
        self.current_input = chars.into_iter().collect();
        self.cursor_col += 1;
    }

    pub fn delete_backwards(&mut self) {
        if self.cursor_col > 0 {
            let mut chars: Vec<char> = self.current_input.chars().collect();
            chars.remove(self.cursor_col - 1);
            self.current_input = chars.into_iter().collect();
            self.cursor_col -= 1;
        }
    }

    pub fn delete_forward(&mut self) {
        let mut chars: Vec<char> = self.current_input.chars().collect();
        if self.cursor_col < chars.len() {
            chars.remove(self.cursor_col);
            self.current_input = chars.into_iter().collect();
        }
    }

    pub fn move_left(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        let len = self.current_input.chars().count();
        if self.cursor_col < len {
            self.cursor_col += 1;
        }
    }
}
