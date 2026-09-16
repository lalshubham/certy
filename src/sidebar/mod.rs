pub mod menu;
pub mod tree;

pub use menu::{MenuItem, MENU_ITEMS};
pub use tree::{rebuild_tree, SidebarNode};

use crate::config::{SIDEBAR_INITIAL_WIDTH, SIDEBAR_ROW_HEIGHT, TAB_BAR_HEIGHT};
use std::collections::HashSet;
use std::path::PathBuf;

pub struct Sidebar {
    pub width: usize,
    pub visible: bool,
    pub scroll_y: usize,
    pub menu_expanded: bool,
    pub root_folder: Option<PathBuf>,
    pub root_expanded: bool,
    pub nodes: Vec<SidebarNode>,
    pub hovered_menu_header: bool,
    pub hovered_menu_item: Option<MenuItem>,
    pub hovered_terminal_header: bool,
    pub hovered_root_header: bool,
    pub hovered_tree_row: Option<usize>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            width: SIDEBAR_INITIAL_WIDTH,
            visible: true,
            scroll_y: 0,
            menu_expanded: false,
            root_folder: None,
            root_expanded: true,
            nodes: Vec::new(),
            hovered_menu_header: false,
            hovered_menu_item: None,
            hovered_terminal_header: false,
            hovered_root_header: false,
            hovered_tree_row: None,
        }
    }

    #[inline(always)]
    pub fn menu_items(&self) -> &[(MenuItem, &'static str)] {
        &MENU_ITEMS
    }

    pub fn menu_total_height(&self) -> usize {
        if self.menu_expanded {
            TAB_BAR_HEIGHT + self.menu_items().len() * SIDEBAR_ROW_HEIGHT
        } else {
            TAB_BAR_HEIGHT
        }
    }

    pub fn total_content_height(&self) -> usize {
        let mut h = self.menu_total_height() + TAB_BAR_HEIGHT;
        if self.root_folder.is_some() {
            h += TAB_BAR_HEIGHT;
            if self.root_expanded {
                h += self.nodes.len() * SIDEBAR_ROW_HEIGHT;
            }
        }
        h
    }

    pub fn root_name(&self) -> Option<String> {
        self.root_folder
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    pub fn clamp_scroll(&mut self, screen_h: usize) {
        let total_h = self.total_content_height();
        let max_scroll = total_h.saturating_sub(screen_h);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
        }
    }

    pub fn toggle_menu(&mut self) {
        self.menu_expanded = !self.menu_expanded;
    }

    pub fn toggle_root(&mut self) {
        self.root_expanded = !self.root_expanded;
    }

    pub fn open_folder(&mut self, path: PathBuf) {
        self.root_folder = Some(path);
        self.root_expanded = true;
        self.scroll_y = 0;
        self.refresh_folder();
    }

    pub fn close_folder(&mut self) {
        self.root_folder = None;
        self.nodes.clear();
        self.root_expanded = true;
        self.scroll_y = 0;
    }

    pub fn refresh_folder(&mut self) {
        let Some(ref root) = self.root_folder else {
            self.nodes.clear();
            return;
        };
        let expanded: HashSet<PathBuf> = self
            .nodes
            .iter()
            .filter(|n| n.is_dir && n.is_expanded)
            .map(|n| n.path.clone())
            .collect();
        self.nodes = rebuild_tree(root, &expanded);
    }

    pub fn toggle_dir(&mut self, node_idx: usize) {
        if node_idx >= self.nodes.len() || !self.nodes[node_idx].is_dir {
            return;
        }

        if self.nodes[node_idx].is_expanded {
            self.nodes[node_idx].is_expanded = false;
            let depth = self.nodes[node_idx].depth;
            let mut remove_count = 0;
            for next_node in &self.nodes[node_idx + 1..] {
                if next_node.depth > depth {
                    remove_count += 1;
                } else {
                    break;
                }
            }
            if remove_count > 0 {
                self.nodes.drain(node_idx + 1..node_idx + 1 + remove_count);
            }
        } else {
            self.nodes[node_idx].is_expanded = true;
            let dir_path = self.nodes[node_idx].path.clone();
            let depth = self.nodes[node_idx].depth + 1;
            let children = tree::read_dir_nodes(&dir_path, depth);
            self.nodes.splice(node_idx + 1..node_idx + 1, children);
        }
    }
}
