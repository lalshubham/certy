use crate::buffer::EditorBuffer;
use std::path::PathBuf;

pub struct Tab {
    pub buffer: EditorBuffer,
    pub title: String,
}

pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active_idx: Option<usize>,
    pub hovered_tab: Option<usize>,
    pub hovered_close: Option<usize>,
    pub pending_close: Option<usize>,
    pub hovered_modal_btn: Option<usize>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_idx: None,
            hovered_tab: None,
            hovered_close: None,
            pending_close: None,
            hovered_modal_btn: None,
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_idx.and_then(|i| self.tabs.get(i))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.active_idx.and_then(|i| self.tabs.get_mut(i))
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

    pub fn new_tab(&mut self) {
        self.tabs.push(Tab {
            buffer: EditorBuffer::new(),
            title: "Untitled".to_string(),
        });
        self.active_idx = Some(self.tabs.len() - 1);
    }

    pub fn request_close(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        if self.tabs[idx].buffer.is_modified {
            self.pending_close = Some(idx);
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
        } else if let Some(cur) = self.active_idx {
            if cur >= self.tabs.len() || cur == idx {
                self.active_idx = Some(self.tabs.len().saturating_sub(1));
            }
        }
        self.pending_close = None;
    }
}
