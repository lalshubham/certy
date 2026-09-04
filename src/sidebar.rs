use crate::config::{SIDEBAR_INITIAL_WIDTH, SIDEBAR_ROW_HEIGHT, TAB_BAR_HEIGHT};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    NewFile,
    NewFolder,
    OpenFile,
    OpenFolder,
    Save,
}

pub const MENU_ITEMS: [(MenuItem, &str); 5] = [
    (MenuItem::NewFile, "New File"),
    (MenuItem::NewFolder, "New Folder"),
    (MenuItem::OpenFile, "Open File"),
    (MenuItem::OpenFolder, "Open Folder"),
    (MenuItem::Save, "Save"),
];

#[derive(Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

pub struct Sidebar {
    pub width: usize,
    pub menu_expanded: bool,
    pub root_folder: Option<PathBuf>,
    pub nodes: Vec<FileNode>,
    pub scroll_y: usize,
    pub hovered_menu_header: bool,
    pub hovered_menu_item: Option<MenuItem>,
    pub hovered_tree_row: Option<usize>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            width: SIDEBAR_INITIAL_WIDTH,
            menu_expanded: true,
            root_folder: None,
            nodes: Vec::new(),
            scroll_y: 0,
            hovered_menu_header: false,
            hovered_menu_item: None,
            hovered_tree_row: None,
        }
    }

    pub fn menu_total_height(&self) -> usize {
        if self.menu_expanded {
            TAB_BAR_HEIGHT + MENU_ITEMS.len() * SIDEBAR_ROW_HEIGHT
        } else {
            TAB_BAR_HEIGHT
        }
    }

    pub fn total_content_height(&self) -> usize {
        let menu_h = self.menu_total_height();
        if self.root_folder.is_some() {
            menu_h + self.nodes.len() * SIDEBAR_ROW_HEIGHT
        } else {
            menu_h
        }
    }

    pub fn clamp_scroll(&mut self, screen_h: usize) {
        let max_scroll = self.total_content_height().saturating_sub(screen_h);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
        }
    }

    pub fn toggle_menu(&mut self) {
        self.menu_expanded = !self.menu_expanded;
    }

    pub fn open_folder(&mut self, path: PathBuf) {
        self.root_folder = Some(path.clone());
        self.nodes = read_dir_nodes(&path, 0);
        self.scroll_y = 0;
    }

    pub fn refresh_folder(&mut self) {
        if let Some(root) = self.root_folder.clone() {
            self.nodes = read_dir_nodes(&root, 0);
        }
    }

    pub fn toggle_dir(&mut self, idx: usize) {
        if idx >= self.nodes.len() || !self.nodes[idx].is_dir {
            return;
        }

        if self.nodes[idx].is_expanded {
            self.nodes[idx].is_expanded = false;
            let target_depth = self.nodes[idx].depth;
            let mut remove_count = 0;
            for node in &self.nodes[idx + 1..] {
                if node.depth > target_depth {
                    remove_count += 1;
                } else {
                    break;
                }
            }
            self.nodes.drain(idx + 1..idx + 1 + remove_count);
        } else {
            self.nodes[idx].is_expanded = true;
            let children = read_dir_nodes(&self.nodes[idx].path, self.nodes[idx].depth + 1);
            let mut insert_idx = idx + 1;
            for child in children {
                self.nodes.insert(insert_idx, child);
                insert_idx += 1;
            }
        }
    }
}

fn read_dir_nodes(dir: &PathBuf, depth: usize) -> Vec<FileNode> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.filter_map(|e| e.ok()) {
            let path = entry.path();
            let is_dir = path.is_dir();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            entries.push(FileNode {
                path,
                name,
                is_dir,
                depth,
                is_expanded: false,
            });
        }
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}
