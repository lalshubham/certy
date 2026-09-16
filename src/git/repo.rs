use super::{DirGitStatus, FileGitStatus, GitFileItem, GitStatusSnapshot};
use git2::{Repository, Status, StatusOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn scan_git_status(workspace_root: &Path) -> GitStatusSnapshot {
    let has_github_dir = workspace_root.join(".github").is_dir();
    let repo = match Repository::discover(workspace_root) {
        Ok(r) => r,
        Err(_) => {
            return GitStatusSnapshot {
                has_github_dir,
                files: Vec::new(),
                dir_statuses: HashMap::new(),
            };
        }
    };

    let workdir = match repo.workdir() {
        Some(w) => w,
        None => {
            return GitStatusSnapshot {
                has_github_dir,
                files: Vec::new(),
                dir_statuses: HashMap::new(),
            };
        }
    };

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .include_ignored(false);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => {
            return GitStatusSnapshot {
                has_github_dir,
                files: Vec::new(),
                dir_statuses: HashMap::new(),
            };
        }
    };

    let mut files = Vec::new();
    let mut dir_statuses: HashMap<PathBuf, DirGitStatus> = HashMap::new();

    for entry in statuses.iter() {
        let status = entry.status();
        if status.contains(Status::IGNORED) {
            continue;
        }

        let file_status = if status.contains(Status::WT_NEW) {
            FileGitStatus::Untracked
        } else if status.contains(Status::INDEX_NEW) {
            FileGitStatus::Added
        } else if status.contains(Status::WT_MODIFIED) || status.contains(Status::INDEX_MODIFIED) {
            FileGitStatus::Modified
        } else if status.contains(Status::WT_DELETED) || status.contains(Status::INDEX_DELETED) {
            FileGitStatus::Deleted
        } else if status.contains(Status::WT_RENAMED) || status.contains(Status::INDEX_RENAMED) {
            FileGitStatus::Renamed
        } else {
            continue;
        };

        let Ok(rel_str) = entry.path() else {
            continue;
        };

        let full_path = workdir.join(rel_str);

        if !full_path.starts_with(workspace_root) && file_status != FileGitStatus::Deleted {
            continue;
        }

        let display_path = full_path
            .strip_prefix(workspace_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| rel_str.to_string());

        let is_modified_type = matches!(
            file_status,
            FileGitStatus::Modified | FileGitStatus::Renamed | FileGitStatus::Deleted
        );

        let mut cur_parent = full_path.parent();
        while let Some(parent) = cur_parent {
            if !parent.starts_with(workspace_root) || parent == workspace_root {
                break;
            }
            let status_entry = dir_statuses
                .entry(parent.to_path_buf())
                .or_insert(DirGitStatus::Untracked);
            if is_modified_type {
                *status_entry = DirGitStatus::Modified;
            }
            cur_parent = parent.parent();
        }

        files.push(GitFileItem {
            path: full_path,
            relative_path: display_path,
            status: file_status,
        });
    }

    files.sort_by(|a, b| {
        a.relative_path
            .to_lowercase()
            .cmp(&b.relative_path.to_lowercase())
    });

    GitStatusSnapshot {
        has_github_dir,
        files,
        dir_statuses,
    }
}

pub fn get_head_file_content(workspace_root: &Path, file_path: &Path) -> Option<String> {
    let repo = Repository::discover(workspace_root).ok()?;
    let workdir = repo.workdir()?;
    let rel_path = file_path.strip_prefix(workdir).ok()?;
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    let tree = commit.tree().ok()?;
    let entry = tree.get_path(rel_path).ok()?;
    let object = entry.to_object(&repo).ok()?;
    let blob = object.into_blob().ok()?;
    Some(String::from_utf8_lossy(blob.content()).to_string())
}
