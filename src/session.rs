use crate::editor::TabManager;
use crate::sidebar::Sidebar;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write as IoWrite;
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
    let Some(path) = session_path() else { return };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let rec_dir = path.parent().map(|d| d.join("recovery"));
    if let Some(ref rd) = rec_dir {
        let _ = fs::create_dir_all(rd);
    }

    let mut content = String::with_capacity(512);
    let (win_w, win_h, win_max) = get_window_session_state();
    let _ = writeln!(content, "window_width:{win_w}");
    let _ = writeln!(content, "window_height:{win_h}");
    let _ = writeln!(content, "window_maximized:{win_max}");
    let _ = writeln!(content, "sidebar_width:{}", sidebar.width);
    let _ = writeln!(content, "sidebar_visible:{}", sidebar.visible);

    if let Some(ref root) = sidebar.root_folder {
        let _ = writeln!(content, "folder:{}", root.display());
    }
    if let Some(active) = tabs.active_idx {
        let _ = writeln!(content, "active:{active}");
    }

    for tab in &tabs.tabs {
        if let Some(ref p) = tab.buffer.file_path {
            let _ = writeln!(content, "file:{}", p.display());
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
                    let _ = writeln!(content, "recovery:{rec_name}");
                } else if rec_file.exists() {
                    let _ = fs::remove_file(&rec_file);
                }
            }
        }
    }

    let _ = fs::write(path, content);
}

pub struct LoadedTab {
    pub path: PathBuf,
    pub recovery: Option<String>,
}

pub struct LoadedSession {
    pub folder: Option<PathBuf>,
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
        } else if let Some(a) = line.strip_prefix("active:") {
            session.active_idx = a.parse().ok();
        } else if let Some(f) = line.strip_prefix("file:") {
            if let Some(tab) = current_tab.take() {
                session.tabs.push(tab);
            }
            current_tab = Some(LoadedTab {
                path: PathBuf::from(f),
                recovery: None,
            });
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
