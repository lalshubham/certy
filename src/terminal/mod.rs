pub mod completion;
pub mod process;
pub mod tab;

use std::path::{Path, PathBuf};
use tab::TerminalTab;

pub fn detect_shell_name() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            if let Some(name) = Path::new(&shell).file_name().and_then(|s| s.to_str()) {
                let lower = name.to_lowercase();
                let stripped = lower.strip_suffix(".exe").unwrap_or(&lower);
                if !stripped.is_empty() {
                    return stripped.to_string();
                }
            }
        }
        if std::env::var_os("PSModulePath").is_some() {
            return "powershell".to_string();
        }
        "cmd".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            if let Some(name) = Path::new(&shell).file_name().and_then(|s| s.to_str()) {
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        "bash".to_string()
    }
}

pub struct Terminal {
    pub is_open: bool,
    pub height: usize,
    pub focused: bool,
    pub tabs: Vec<TerminalTab>,
    pub active_idx: usize,
    pub default_cwd: PathBuf,
    pub tab_scroll_x: usize,
    pub hovered_new: bool,
    pub hovered_tab: Option<usize>,
    pub hovered_close_tab: Option<usize>,
    pub needs_fs_refresh: bool,
}

impl Terminal {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let initial_tab = TerminalTab::new(detect_shell_name(), cwd.clone());
        Self {
            is_open: false,
            height: 220,
            focused: false,
            tabs: vec![initial_tab],
            active_idx: 0,
            default_cwd: cwd,
            tab_scroll_x: 0,
            hovered_new: false,
            hovered_tab: None,
            hovered_close_tab: None,
            needs_fs_refresh: false,
        }
    }

    pub fn reset(&mut self, cwd: PathBuf) {
        self.close_all();
        self.is_open = false;
        self.focused = false;
        self.default_cwd = cwd.clone();
        self.tabs = vec![TerminalTab::new(detect_shell_name(), cwd)];
        self.active_idx = 0;
        self.tab_scroll_x = 0;
    }

    pub fn active_tab(&self) -> Option<&TerminalTab> {
        self.tabs.get(self.active_idx)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut TerminalTab> {
        self.tabs.get_mut(self.active_idx)
    }

    pub fn total_tabs_width(&self, char_w: usize) -> usize {
        self.tabs.iter().map(|t| t.width(char_w)).sum()
    }

    pub fn clamp_tab_scroll(&mut self, char_w: usize, available_w: usize) {
        let total_w = self.total_tabs_width(char_w);
        let max_scroll = total_w.saturating_sub(available_w);
        if self.tab_scroll_x > max_scroll {
            self.tab_scroll_x = max_scroll;
        }
    }

    pub fn ensure_active_tab_visible(&mut self, char_w: usize, available_w: usize) {
        if self.tabs.is_empty() {
            self.tab_scroll_x = 0;
            return;
        }
        let mut start_x = 0;
        for i in 0..self.active_idx {
            if let Some(t) = self.tabs.get(i) {
                start_x += t.width(char_w);
            }
        }
        let active_w = self
            .tabs
            .get(self.active_idx)
            .map(|t| t.width(char_w))
            .unwrap_or(0);

        if start_x < self.tab_scroll_x {
            self.tab_scroll_x = start_x;
        } else if start_x + active_w > self.tab_scroll_x + available_w {
            self.tab_scroll_x = (start_x + active_w).saturating_sub(available_w);
        }
        self.clamp_tab_scroll(char_w, available_w);
    }

    pub fn add_terminal(&mut self, char_w: usize, available_w: usize) {
        let tab = TerminalTab::new(detect_shell_name(), self.default_cwd.clone());
        self.tabs.push(tab);
        self.active_idx = self.tabs.len() - 1;
        self.focused = true;
        self.ensure_active_tab_visible(char_w, available_w);
    }

    pub fn remove_terminal(&mut self, idx: usize, char_w: usize, available_w: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs[idx].kill_process();
        self.tabs.remove(idx);

        if self.tabs.is_empty() {
            self.is_open = false;
            self.focused = false;
            self.active_idx = 0;
            self.tab_scroll_x = 0;
        } else {
            if self.active_idx >= self.tabs.len() || self.active_idx == idx {
                self.active_idx = self.tabs.len().saturating_sub(1);
            }
            self.ensure_active_tab_visible(char_w, available_w);
        }
        self.hovered_tab = None;
        self.hovered_close_tab = None;
    }

    pub fn close_all(&mut self) {
        for tab in &mut self.tabs {
            tab.kill_process();
        }
    }

    pub fn has_running_process(&self) -> bool {
        self.tabs.iter().any(|t| t.is_running)
    }

    pub fn poll_output(&mut self, vis_rows: usize) -> bool {
        let mut updated = false;
        for tab in &mut self.tabs {
            if tab.poll_output(vis_rows) {
                updated = true;
            }
            if tab.command_finished {
                tab.command_finished = false;
                self.needs_fs_refresh = true;
            }
        }
        updated
    }

    pub fn vis_rows(&self, line_h: usize) -> usize {
        if line_h == 0 {
            return 0;
        }
        let body_h = self
            .height
            .saturating_sub(crate::config::TERMINAL_TAB_BAR_HEIGHT);
        let track_h = body_h.saturating_sub(crate::config::SCROLLBAR_THICKNESS);
        track_h.saturating_sub(4) / line_h
    }
}
