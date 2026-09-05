use crate::buffer::EditorBuffer;
use std::path::{Path, PathBuf};

pub struct Tab {
    pub buffer: EditorBuffer,
    pub title: String,
}

impl Tab {
    #[inline]
    pub fn width(&self, char_w: usize) -> usize {
        let dirty_len = if self.buffer.is_modified { 2 } else { 0 };
        (self.title.len() + dirty_len) * char_w + 38
    }
}

pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active_idx: Option<usize>,
    pub hovered_tab: Option<usize>,
    pub hovered_close: Option<usize>,
    pub pending_close: Option<usize>,
    pub closing_app: bool,
    pub hovered_modal_btn: Option<usize>,
    pub scroll_x: usize,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_idx: None,
            hovered_tab: None,
            hovered_close: None,
            pending_close: None,
            closing_app: false,
            hovered_modal_btn: None,
            scroll_x: 0,
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_idx.and_then(|i| self.tabs.get(i))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.active_idx.and_then(|i| self.tabs.get_mut(i))
    }

    pub fn has_modified(&self) -> bool {
        self.tabs.iter().any(|t| t.buffer.is_modified)
    }

    pub fn total_tabs_width(&self, char_w: usize) -> usize {
        self.tabs.iter().map(|t| t.width(char_w)).sum()
    }

    pub fn clamp_scroll(&mut self, char_w: usize, available_w: usize) {
        let total_w = self.total_tabs_width(char_w);
        let max_scroll = total_w.saturating_sub(available_w);
        if self.scroll_x > max_scroll {
            self.scroll_x = max_scroll;
        }
    }

    pub fn ensure_active_tab_visible(&mut self, char_w: usize, available_w: usize) {
        let active_idx = match self.active_idx {
            Some(i) => i,
            None => return,
        };
        let mut start_x = 0;
        for i in 0..active_idx {
            if let Some(t) = self.tabs.get(i) {
                start_x += t.width(char_w);
            }
        }
        let active_w = self
            .tabs
            .get(active_idx)
            .map(|t| t.width(char_w))
            .unwrap_or(0);

        if start_x < self.scroll_x {
            self.scroll_x = start_x;
        } else if start_x + active_w > self.scroll_x + available_w {
            self.scroll_x = (start_x + active_w).saturating_sub(available_w);
        }
        self.clamp_scroll(char_w, available_w);
    }

    pub fn open_file(&mut self, path: PathBuf) {
        for (idx, tab) in self.tabs.iter().enumerate() {
            if tab.buffer.file_path.as_ref() == Some(&path) {
                self.active_idx = Some(idx);
                return;
            }
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let mut buffer = EditorBuffer::new();
        let _ = buffer.load_file(path);

        self.tabs.push(Tab {
            buffer,
            title: name,
        });
        self.active_idx = Some(self.tabs.len() - 1);
    }

    pub fn open_recovered(&mut self, path: PathBuf, recovery_path: &Path) {
        for (idx, tab) in self.tabs.iter().enumerate() {
            if tab.buffer.file_path.as_ref() == Some(&path) {
                self.active_idx = Some(idx);
                return;
            }
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let mut buffer = EditorBuffer::new();
        let _ = buffer.load_recovered(path, recovery_path);

        self.tabs.push(Tab {
            buffer,
            title: name,
        });
        self.active_idx = Some(self.tabs.len() - 1);
    }

    pub fn request_close(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        if self.tabs[idx].buffer.is_modified {
            self.pending_close = Some(idx);
            self.hovered_tab = None;
            self.hovered_close = None;
        } else {
            self.close_tab(idx);
        }
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.active_idx = None;
            self.scroll_x = 0;
        } else if let Some(cur) = self.active_idx {
            if cur >= self.tabs.len() || cur == idx {
                self.active_idx = Some(self.tabs.len().saturating_sub(1));
            }
        }
        self.pending_close = None;
        self.hovered_tab = None;
        self.hovered_close = None;
    }

    pub fn close_folder_tabs(&mut self, root: &Path) {
        self.tabs.retain(|tab| {
            if let Some(ref p) = tab.buffer.file_path {
                !p.starts_with(root)
            } else {
                true
            }
        });
        if self.tabs.is_empty() {
            self.active_idx = None;
            self.scroll_x = 0;
        } else if let Some(cur) = self.active_idx {
            if cur >= self.tabs.len() {
                self.active_idx = Some(self.tabs.len().saturating_sub(1));
            }
        }
        self.pending_close = None;
        self.hovered_tab = None;
        self.hovered_close = None;
    }
}
