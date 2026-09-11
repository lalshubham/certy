use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

pub(super) fn build_dir_tree_internal(
    dir: &Path,
    depth: usize,
    expanded: &HashSet<PathBuf>,
) -> Vec<FileNode> {
    let mut result = Vec::new();
    let entries = read_dir_nodes(dir, depth);
    for mut node in entries {
        if node.is_dir && expanded.contains(&node.path) {
            node.is_expanded = true;
            let children = build_dir_tree_internal(&node.path, depth + 1, expanded);
            result.push(node);
            result.extend(children);
        } else {
            result.push(node);
        }
    }
    result
}

pub fn read_dir_nodes(dir: &Path, depth: usize) -> Vec<FileNode> {
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
