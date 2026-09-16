pub mod diff;
pub mod repo;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileGitStatus {
    Modified,
    Untracked,
    Added,
    Deleted,
    Renamed,
}

impl FileGitStatus {
    #[inline]
    pub fn badge_char(&self) -> &'static str {
        match self {
            FileGitStatus::Modified => "M",
            FileGitStatus::Untracked => "U",
            FileGitStatus::Added => "A",
            FileGitStatus::Deleted => "D",
            FileGitStatus::Renamed => "R",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitFileItem {
    pub path: PathBuf,
    pub relative_path: String,
    pub status: FileGitStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirGitStatus {
    Untracked,
    Modified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineChangeKind {
    Added,
    Modified,
    DeletedAbove,
}

#[derive(Default, Clone, Debug)]
pub struct GutterDecorations {
    pub lines: HashMap<usize, LineChangeKind>,
}

#[derive(Default, Clone, Debug)]
pub struct GitStatusSnapshot {
    pub has_github_dir: bool,
    pub files: Vec<GitFileItem>,
    pub dir_statuses: HashMap<PathBuf, DirGitStatus>,
}

impl GitStatusSnapshot {
    pub fn status_for_file(&self, path: &Path) -> Option<FileGitStatus> {
        self.files
            .iter()
            .find(|item| item.path == path)
            .map(|item| item.status)
    }

    pub fn status_for_dir(&self, dir_path: &Path) -> Option<DirGitStatus> {
        self.dir_statuses.get(dir_path).copied()
    }
}
