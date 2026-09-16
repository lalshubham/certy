use super::App;
use crate::config::SIDEBAR_MIN_WIDTH;
use crate::editor::TabManager;
use crate::session::{load_session, recovery_dir, recovery_file_name, save_session};
use crate::sidebar::Sidebar;
use crate::terminal::Terminal;
use std::fs;
use std::path::{Path, PathBuf};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

pub fn to_full_path(path: &Path) -> String {
    if let Ok(canon) = fs::canonicalize(path) {
        canon.display().to_string()
    } else if path.is_absolute() {
        path.display().to_string()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path).display().to_string()
    } else {
        path.display().to_string()
    }
}

pub fn trigger_app_close(
    tabs: &mut TabManager,
    sidebar: &Sidebar,
    terminal: &mut Terminal,
    window: &Window,
    event_loop: &ActiveEventLoop,
) {
    if tabs.has_modified() {
        tabs.closing_app = true;
        tabs.closing_files = false;
        tabs.pending_close = None;
        window.request_redraw();
    } else {
        terminal.close_all();
        save_session(sidebar, tabs);
        event_loop.exit();
    }
}

pub fn update_window_title(
    window: &Window,
    tabs: &TabManager,
    sidebar: &Sidebar,
    last_title: &mut String,
) {
    let title = if let Some(tab) = tabs.active_tab() {
        let path_str = if let Some(p) = &tab.buffer.file_path {
            to_full_path(p)
        } else {
            tab.title.clone()
        };
        format!("{path_str} - Certy")
    } else if let Some(root) = &sidebar.root_folder {
        format!("{} - Certy", to_full_path(root))
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        format!("{} - Certy", to_full_path(&cwd))
    };
    if *last_title != title {
        window.set_title(&title);
        *last_title = title;
    }
}

impl App {
    pub fn apply_loaded_session(&mut self) {
        let session = match load_session() {
            Some(s) => s,
            None => return,
        };
        if let Some(w) = session.sidebar_width {
            self.sidebar.width = w.max(SIDEBAR_MIN_WIDTH);
        }
        if let Some(v) = session.sidebar_visible {
            self.sidebar.visible = v;
        }
        let rec_dir = recovery_dir();
        if let Some(p) = session.folder {
            if p.is_dir() {
                self.terminal.default_cwd = p.clone();
                for tab in &mut self.terminal.tabs {
                    tab.cwd = p.clone();
                }
                self.sidebar.open_folder(p);
            }
        }
        for stab in session.tabs {
            if !stab.path.is_file() {
                continue;
            }
            let mut opened = false;
            if let Some(ref r) = stab.recovery {
                if let Some(ref rd) = rec_dir {
                    let rf = rd.join(r);
                    if rf.is_file() {
                        self.tabs.open_recovered(stab.path.clone(), &rf);
                        opened = true;
                    }
                }
            }
            if !opened {
                self.tabs.open_file(stab.path);
            }
        }
        if let Some(act) = session.active_idx {
            if act < self.tabs.tabs.len() {
                self.tabs.active_idx = Some(act);
            }
        }

        self.tabs
            .refresh_all_git_decorations(self.sidebar.root_folder.as_deref());
    }

    pub fn sync_filesystem(&mut self) -> bool {
        let mut changed = false;
        let root_opt = self.sidebar.root_folder.clone();
        let root_to_close = if let Some(root) = root_opt {
            if !root.exists() {
                Some(root)
            } else {
                self.sidebar.refresh_folder();
                self.tabs.refresh_active_git_decorations(Some(&root));
                changed = true;
                None
            }
        } else {
            None
        };
        if let Some(root) = root_to_close {
            if let Some(rec_dir) = recovery_dir() {
                for tab in &self.tabs.tabs {
                    if let Some(ref p) = tab.buffer.file_path {
                        if p.starts_with(&root) {
                            let _ = fs::remove_file(rec_dir.join(recovery_file_name(p)));
                        }
                    }
                }
            }
            self.tabs.close_folder_tabs(&root);
            self.sidebar.close_folder();
            self.tabs.refresh_all_git_decorations(None);
            let fallback_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            if !self.terminal.default_cwd.exists() {
                self.terminal.default_cwd = fallback_cwd.clone();
            }
            for tab in &mut self.terminal.tabs {
                if !tab.cwd.exists() {
                    tab.cwd = fallback_cwd.clone();
                }
            }
            changed = true;
        }
        if let Some(rec_dir) = recovery_dir() {
            for tab in &self.tabs.tabs {
                if let Some(ref p) = tab.buffer.file_path {
                    if !p.exists() {
                        let _ = fs::remove_file(rec_dir.join(recovery_file_name(p)));
                    }
                }
            }
        }
        if self.tabs.close_missing_files() {
            changed = true;
        }
        if changed {
            save_session(&self.sidebar, &self.tabs);
            if let Some(ref r) = self.renderer {
                let effective_sidebar_w = if self.sidebar.visible {
                    self.sidebar.width
                } else {
                    0
                };
                let avail_w = r.width.saturating_sub(effective_sidebar_w);
                self.tabs.clamp_scroll(r.font_manager.char_width, avail_w);
                self.sidebar.clamp_scroll(r.height);
            }
            if let Some(w) = &self.window {
                update_window_title(w, &self.tabs, &self.sidebar, &mut self.current_title);
            }
        }
        changed
    }
}
