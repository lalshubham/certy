use crate::config::{SIDEBAR_INITIAL_WIDTH, SIDEBAR_ROW_HEIGHT, TAB_BAR_HEIGHT};
use crate::git::repo::scan_git_status;
use crate::git::GitStatusSnapshot;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuItem {
    Save,
    NewFile,
    NewFolder,
    OpenFile,
    OpenFolder,
    CloseFolder,
    Exit,
}

#[derive(Clone, Debug)]
pub struct FileTreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
}

pub struct Sidebar {
    pub visible: bool,
    pub width: usize,
    pub scroll_y: usize,
    pub menu_expanded: bool,
    pub root_folder: Option<PathBuf>,
    pub root_expanded: bool,
    pub git_expanded: bool,
    pub nodes: Vec<FileTreeNode>,
    pub git_snapshot: GitStatusSnapshot,
    pub hovered_menu_header: bool,
    pub hovered_menu_item: Option<MenuItem>,
    pub hovered_terminal_header: bool,
    pub hovered_git_header: bool,
    pub hovered_git_row: Option<usize>,
    pub hovered_root_header: bool,
    pub hovered_tree_row: Option<usize>,
    expanded_dirs: HashSet<PathBuf>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            visible: true,
            width: SIDEBAR_INITIAL_WIDTH,
            scroll_y: 0,
            menu_expanded: false,
            root_folder: None,
            root_expanded: true,
            git_expanded: false,
            nodes: Vec::new(),
            git_snapshot: GitStatusSnapshot::default(),
            hovered_menu_header: false,
            hovered_menu_item: None,
            hovered_terminal_header: false,
            hovered_git_header: false,
            hovered_git_row: None,
            hovered_root_header: false,
            hovered_tree_row: None,
            expanded_dirs: HashSet::new(),
        }
    }

    pub fn menu_items(&self) -> &[(MenuItem, &'static str)] {
        &[
            (MenuItem::Save, "Save"),
            (MenuItem::NewFile, "New File"),
            (MenuItem::NewFolder, "New Folder"),
            (MenuItem::OpenFile, "Open File"),
            (MenuItem::OpenFolder, "Open Folder"),
            (MenuItem::CloseFolder, "Close Folder"),
            (MenuItem::Exit, "Exit"),
        ]
    }

    pub fn menu_total_height(&self) -> usize {
        TAB_BAR_HEIGHT
            + if self.menu_expanded {
                self.menu_items().len() * SIDEBAR_ROW_HEIGHT
            } else {
                0
            }
    }

    pub fn git_total_height(&self) -> usize {
        if self.root_folder.is_none() {
            return 0;
        }
        let mut h = TAB_BAR_HEIGHT;
        if self.git_expanded {
            let row_count = if self.git_snapshot.files.is_empty() {
                1
            } else {
                self.git_snapshot.files.len()
            };
            h += row_count * SIDEBAR_ROW_HEIGHT;
        }
        h
    }

    pub fn total_content_height(&self) -> usize {
        let mut h = self.menu_total_height() + TAB_BAR_HEIGHT;
        if self.root_folder.is_some() {
            h += self.git_total_height();
            h += TAB_BAR_HEIGHT;
            if self.root_expanded {
                h += self.nodes.len() * SIDEBAR_ROW_HEIGHT;
            }
        }
        h
    }

    pub fn root_name(&self) -> Option<String> {
        self.root_folder.as_ref().map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("FOLDER")
                .to_uppercase()
        })
    }

    pub fn clamp_scroll(&mut self, screen_h: usize) {
        let total = self.total_content_height();
        let max_scroll = total.saturating_sub(screen_h);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
        }
    }

    pub fn toggle_menu(&mut self) {
        self.menu_expanded = !self.menu_expanded;
    }

    pub fn toggle_root(&mut self) {
        if self.root_expanded {
            self.expanded_dirs.clear();
            self.root_expanded = false;
        } else {
            self.root_expanded = true;
        }
        self.rebuild_tree();
    }

    pub fn toggle_git(&mut self) {
        self.git_expanded = !self.git_expanded;
    }

    pub fn toggle_dir(&mut self, idx: usize) {
        if let Some(node) = self.nodes.get(idx) {
            if node.is_dir {
                let path = node.path.clone();
                if self.expanded_dirs.contains(&path) {
                    self.expanded_dirs.retain(|p| !p.starts_with(&path));
                } else {
                    self.expanded_dirs.insert(path);
                }
                self.rebuild_tree();
            }
        }
    }

    pub fn refresh_git(&mut self) {
        if let Some(ref root) = self.root_folder {
            self.git_snapshot = scan_git_status(root);
        } else {
            self.git_snapshot = GitStatusSnapshot::default();
        }
    }

    pub fn open_folder(&mut self, path: PathBuf) {
        let canon_path = fs::canonicalize(&path).unwrap_or(path);
        self.root_folder = Some(canon_path);
        self.expanded_dirs.clear();
        self.root_expanded = true;
        self.git_expanded = false;
        self.refresh_git();
        self.rebuild_tree();
    }

    pub fn close_folder(&mut self) {
        self.root_folder = None;
        self.nodes.clear();
        self.expanded_dirs.clear();
        self.git_snapshot = GitStatusSnapshot::default();
        self.scroll_y = 0;
    }

    pub fn refresh_folder(&mut self) {
        if self.root_folder.is_some() {
            self.refresh_git();
            self.rebuild_tree();
        }
    }

    fn rebuild_tree(&mut self) {
        let Some(root) = self.root_folder.clone() else {
            self.nodes.clear();
            return;
        };

        let mut nodes = Vec::new();
        Self::scan_dir(&root, 0, &self.expanded_dirs, &mut nodes);
        self.nodes = nodes;
    }

    fn scan_dir(
        dir: &Path,
        depth: usize,
        expanded_dirs: &HashSet<PathBuf>,
        out: &mut Vec<FileTreeNode>,
    ) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') && name != ".env" && name != ".gitignore" && name != ".github"
            {
                continue;
            }
            if matches!(
                name.as_str(),
                ".git" | "target" | "node_modules" | ".certy_session"
            ) {
                continue;
            }

            if path.is_dir() {
                dirs.push((name, path));
            } else {
                files.push((name, path));
            }
        }

        dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        for (name, path) in dirs {
            let is_expanded = expanded_dirs.contains(&path);
            out.push(FileTreeNode {
                path: path.clone(),
                name,
                is_dir: true,
                is_expanded,
                depth,
            });
            if is_expanded {
                Self::scan_dir(&path, depth + 1, expanded_dirs, out);
            }
        }

        for (name, path) in files {
            out.push(FileTreeNode {
                path,
                name,
                is_dir: false,
                is_expanded: false,
                depth,
            });
        }
    }
}
