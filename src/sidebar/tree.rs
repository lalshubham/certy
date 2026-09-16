use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct SidebarNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
}

pub fn should_ignore(name: &str) -> bool {
    if name.starts_with('.') && name != ".env" && name != ".gitignore" {
        return true;
    }
    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | ".vscode"
            | ".idea"
            | "dist"
            | "build"
            | "vendor"
            | ".certy_session"
    )
}

pub fn read_dir_nodes(dir: &Path, depth: usize) -> Vec<SidebarNode> {
    let mut nodes = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return nodes;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();

        if should_ignore(&name) {
            continue;
        }

        let is_dir = path.is_dir();
        nodes.push(SidebarNode {
            path,
            name,
            is_dir,
            is_expanded: false,
            depth,
        });
    }

    nodes.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    nodes
}

pub fn rebuild_tree(root: &Path, expanded: &HashSet<PathBuf>) -> Vec<SidebarNode> {
    let mut result = Vec::new();
    populate_tree(root, 0, expanded, &mut result);
    result
}

fn populate_tree(
    dir: &Path,
    depth: usize,
    expanded: &HashSet<PathBuf>,
    result: &mut Vec<SidebarNode>,
) {
    let mut children = read_dir_nodes(dir, depth);
    for mut child in children.drain(..) {
        if child.is_dir && expanded.contains(&child.path) {
            child.is_expanded = true;
            let child_path = child.path.clone();
            result.push(child);
            populate_tree(&child_path, depth + 1, expanded, result);
        } else {
            result.push(child);
        }
    }
}
