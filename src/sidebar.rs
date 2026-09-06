use crate::config::{SIDEBAR_INITIAL_WIDTH, SIDEBAR_ROW_HEIGHT, TAB_BAR_HEIGHT};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Save,
    NewFile,
    NewFolder,
    OpenFile,
    OpenFolder,
    CloseFolder,
    Exit,
}

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
    pub root_expanded: bool,
    pub nodes: Vec<FileNode>,
    pub scroll_y: usize,
    pub hovered_menu_header: bool,
    pub hovered_menu_item: Option<MenuItem>,
    pub hovered_terminal_header: bool,
    pub hovered_root_header: bool,
    pub hovered_tree_row: Option<usize>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            width: SIDEBAR_INITIAL_WIDTH,
            menu_expanded: false,
            root_folder: None,
            root_expanded: true,
            nodes: Vec::new(),
            scroll_y: 0,
            hovered_menu_header: false,
            hovered_menu_item: None,
            hovered_terminal_header: false,
            hovered_root_header: false,
            hovered_tree_row: None,
        }
    }

    pub fn menu_items(&self) -> [(MenuItem, &'static str); 7] {
        [
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
        if self.menu_expanded {
            TAB_BAR_HEIGHT + 7 * SIDEBAR_ROW_HEIGHT
        } else {
            TAB_BAR_HEIGHT
        }
    }

    pub fn total_content_height(&self) -> usize {
        let menu_h = self.menu_total_height();
        let terminal_header_h = TAB_BAR_HEIGHT;
        if self.root_folder.is_some() {
            let tree_h = if self.root_expanded {
                self.nodes.len() * SIDEBAR_ROW_HEIGHT
            } else {
                0
            };
            menu_h + terminal_header_h + TAB_BAR_HEIGHT + tree_h
        } else {
            menu_h + terminal_header_h
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

    pub fn toggle_root(&mut self) {
        self.root_expanded = !self.root_expanded;
        if !self.root_expanded {
            if let Some(ref root) = self.root_folder.clone() {
                self.nodes = read_dir_nodes(root, 0);
            }
        }
    }

    pub fn root_name(&self) -> Option<String> {
        self.root_folder.as_ref().map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| p.to_str().unwrap_or("FOLDER"))
                .to_uppercase()
        })
    }

    pub fn open_folder(&mut self, path: PathBuf) {
        self.root_folder = Some(path.clone());
        self.root_expanded = true;
        self.nodes = read_dir_nodes(&path, 0);
        self.scroll_y = 0;
    }

    pub fn open_folder_with_expanded(&mut self, path: PathBuf, expanded: &HashSet<PathBuf>) {
        self.root_folder = Some(path.clone());
        self.root_expanded = true;
        self.nodes = build_dir_tree(&path, 0, expanded);
        self.scroll_y = 0;
    }

    pub fn close_folder(&mut self) {
        self.root_folder = None;
        self.nodes.clear();
        self.scroll_y = 0;
    }

    pub fn refresh_folder(&mut self) {
        if let Some(root) = self.root_folder.clone() {
            let expanded: HashSet<PathBuf> = self
                .nodes
                .iter()
                .filter(|n| n.is_dir && n.is_expanded)
                .map(|n| n.path.clone())
                .collect();
            self.nodes = build_dir_tree(&root, 0, &expanded);
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

fn build_dir_tree(dir: &PathBuf, depth: usize, expanded: &HashSet<PathBuf>) -> Vec<FileNode> {
    let mut result = Vec::new();
    let entries = read_dir_nodes(dir, depth);
    for mut node in entries {
        if node.is_dir && expanded.contains(&node.path) {
            node.is_expanded = true;
            let children = build_dir_tree(&node.path, depth + 1, expanded);
            result.push(node);
            result.extend(children);
        } else {
            result.push(node);
        }
    }
    result
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
