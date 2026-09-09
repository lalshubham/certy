use crate::editor::TabManager;
use crate::sidebar::Sidebar;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static WINDOW_STATE: Mutex<(f64, f64, bool)> = Mutex::new((1024.0, 768.0, true));

pub fn update_window_size(w: f64, h: f64, is_maximized: bool) {
    if let Ok(mut state) = WINDOW_STATE.lock() {
        if is_maximized {
            state.2 = true;
        } else {
            *state = (w, h, false);
        }
    }
}

pub fn get_window_session_state() -> (f64, f64, bool) {
    WINDOW_STATE
        .lock()
        .map(|s| *s)
        .unwrap_or((1024.0, 768.0, true))
}

pub fn session_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(PathBuf::from(appdata).join("certy").join("session.txt"));
        }
    }
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(config_home).join("certy").join("session.txt"))
    } else if let Some(home) = std::env::var_os("HOME") {
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("certy")
                .join("session.txt"),
        )
    } else {
        std::env::current_dir()
            .ok()
            .map(|p| p.join(".certy_session"))
    }
}

pub fn recovery_dir() -> Option<PathBuf> {
    session_path().and_then(|p| p.parent().map(|d| d.join("recovery")))
}

pub fn recovery_file_name(path: &Path) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}.bak")
}

pub fn save_session(sidebar: &Sidebar, tabs: &TabManager) {
    if let Some(path) = session_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let rec_dir = path.parent().map(|d| d.join("recovery"));
        if let Some(ref rd) = rec_dir {
            let _ = fs::create_dir_all(rd);
        }

        let mut content = String::new();
        let (win_w, win_h, win_max) = get_window_session_state();
        content.push_str(&format!("window_width:{}\n", win_w));
        content.push_str(&format!("window_height:{}\n", win_h));
        content.push_str(&format!("window_maximized:{}\n", win_max));
        content.push_str(&format!("sidebar_width:{}\n", sidebar.width));
        content.push_str(&format!("sidebar_visible:{}\n", sidebar.visible));

        if let Some(ref root) = sidebar.root_folder {
            content.push_str(&format!("folder:{}\n", root.display()));
        }
        for node in &sidebar.nodes {
            if node.is_dir && node.is_expanded {
                content.push_str(&format!("expanded:{}\n", node.path.display()));
            }
        }
        if let Some(active) = tabs.active_idx {
            content.push_str(&format!("active:{}\n", active));
        }
        for tab in &tabs.tabs {
            if let Some(ref p) = tab.buffer.file_path {
                content.push_str(&format!("file:{}\n", p.display()));
                content.push_str(&format!(
                    "cursor:{},{},{}\n",
                    tab.buffer.cursor_char, tab.buffer.scroll_line, tab.buffer.scroll_col
                ));
                if let Some(ref rd) = rec_dir {
                    let rec_name = recovery_file_name(p);
                    let rec_file = rd.join(&rec_name);
                    if tab.buffer.is_modified {
                        if let Ok(file) = fs::File::create(&rec_file) {
                            let mut writer = std::io::BufWriter::new(file);
                            for chunk in tab.buffer.text().chunks() {
                                let _ = writer.write_all(chunk.as_bytes());
                            }
                            let _ = writer.flush();
                        }
                        content.push_str(&format!("recovery:{}\n", rec_name));
                    } else if rec_file.exists() {
                        let _ = fs::remove_file(&rec_file);
                    }
                }
            }
        }
        let _ = fs::write(path, content);
    }
}

pub struct LoadedTab {
    pub path: PathBuf,
    pub cursor: usize,
    pub scroll_line: usize,
    pub scroll_col: usize,
    pub recovery: Option<String>,
}

pub struct LoadedSession {
    pub folder: Option<PathBuf>,
    pub expanded: HashSet<PathBuf>,
    pub active_idx: Option<usize>,
    pub tabs: Vec<LoadedTab>,
    pub sidebar_width: Option<usize>,
    pub sidebar_visible: Option<bool>,
    pub window_width: Option<f64>,
    pub window_height: Option<f64>,
    pub window_maximized: Option<bool>,
}

pub fn load_session() -> Option<LoadedSession> {
    let path = session_path()?;
    let content = fs::read_to_string(path).ok()?;

    let mut session = LoadedSession {
        folder: None,
        expanded: HashSet::new(),
        active_idx: None,
        tabs: Vec::new(),
        sidebar_width: None,
        sidebar_visible: None,
        window_width: None,
        window_height: None,
        window_maximized: None,
    };

    let mut current_tab: Option<LoadedTab> = None;

    for line in content.lines() {
        if let Some(w) = line.strip_prefix("sidebar_width:") {
            session.sidebar_width = w.parse().ok();
        } else if let Some(v) = line.strip_prefix("sidebar_visible:") {
            session.sidebar_visible = v.parse().ok();
        } else if let Some(w) = line.strip_prefix("window_width:") {
            session.window_width = w.parse().ok();
        } else if let Some(h) = line.strip_prefix("window_height:") {
            session.window_height = h.parse().ok();
        } else if let Some(m) = line.strip_prefix("window_maximized:") {
            session.window_maximized = m.parse().ok();
        } else if let Some(f) = line.strip_prefix("folder:") {
            session.folder = Some(PathBuf::from(f));
        } else if let Some(e) = line.strip_prefix("expanded:") {
            session.expanded.insert(PathBuf::from(e));
        } else if let Some(a) = line.strip_prefix("active:") {
            session.active_idx = a.parse().ok();
        } else if let Some(f) = line.strip_prefix("file:") {
            if let Some(tab) = current_tab.take() {
                session.tabs.push(tab);
            }
            current_tab = Some(LoadedTab {
                path: PathBuf::from(f),
                cursor: 0,
                scroll_line: 0,
                scroll_col: 0,
                recovery: None,
            });
        } else if let Some(c) = line.strip_prefix("cursor:") {
            if let Some(ref mut tab) = current_tab {
                let parts: Vec<&str> = c.split(',').collect();
                if parts.len() == 3 {
                    tab.cursor = parts[0].parse().unwrap_or(0);
                    tab.scroll_line = parts[1].parse().unwrap_or(0);
                    tab.scroll_col = parts[2].parse().unwrap_or(0);
                }
            }
        } else if let Some(r) = line.strip_prefix("recovery:") {
            if let Some(ref mut tab) = current_tab {
                tab.recovery = Some(r.to_string());
            }
        }
    }

    if let Some(tab) = current_tab.take() {
        session.tabs.push(tab);
    }
    if let (Some(w), Some(h)) = (session.window_width, session.window_height) {
        let max = session.window_maximized.unwrap_or(false);
        update_window_size(w, h, max);
    }
    Some(session)
}
