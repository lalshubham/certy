use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct QuickOpenFileItem {
    pub path: PathBuf,
    pub relative_path: String,
}

pub fn fuzzy_score(q_lower: &[char], target: &str) -> Option<i32> {
    if q_lower.is_empty() {
        return Some(0);
    }
    let mut q_idx = 0;
    let mut score = 0;
    let mut prev_idx: Option<usize> = None;
    let mut prev_ch: Option<char> = None;
    for (t_idx, ch) in target.chars().enumerate() {
        let ch_lower = ch.to_ascii_lowercase();
        if q_idx < q_lower.len() && ch_lower == q_lower[q_idx] {
            let mut char_score = 10;
            if let Some(p) = prev_idx {
                if p + 1 == t_idx {
                    char_score += 15;
                }
            }
            if prev_ch.map_or(true, |p| matches!(p, '/' | '\\' | '_' | '-' | '.')) {
                char_score += 20;
            }
            score += char_score;
            prev_idx = Some(t_idx);
            q_idx += 1;
        }
        prev_ch = Some(ch);
    }
    if q_idx == q_lower.len() {
        let filename_start = target
            .rfind(|c| c == '/' || c == '\\')
            .map(|i| i + 1)
            .unwrap_or(0);
        if let Some(last_match) = prev_idx {
            if last_match >= filename_start {
                score += 35;
            }
        }
        score -= (target.chars().count() as i32) / 4;
        Some(score)
    } else {
        None
    }
}

pub fn collect_workspace_files(root: &Path, max_files: usize) -> Vec<QuickOpenFileItem> {
    let mut items = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if items.len() >= max_files {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if name.starts_with('.') && name != ".env" && name != ".gitignore" {
                if path.is_dir() {
                    continue;
                }
            }
            if path.is_dir() {
                if matches!(
                    name.as_ref(),
                    ".git"
                        | "target"
                        | "node_modules"
                        | ".vscode"
                        | ".idea"
                        | "dist"
                        | "build"
                        | "vendor"
                        | ".certy_session"
                ) {
                    continue;
                }
                stack.push(path);
            } else if path.is_file() {
                let relative_path = path
                    .strip_prefix(root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| name.to_string());
                items.push(QuickOpenFileItem {
                    path,
                    relative_path,
                });
                if items.len() >= max_files {
                    break;
                }
            }
        }
    }
    items.sort_by(|a, b| {
        a.relative_path
            .to_lowercase()
            .cmp(&b.relative_path.to_lowercase())
    });
    items
}
