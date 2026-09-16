use super::{GutterDecorations, LineChangeKind};
use similar::{ChangeTag, DiffTag, TextDiff};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Deleted,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct FileDiff {
    pub lines: Vec<DiffLine>,
}

pub fn compute_gutter_decorations(old_str: &str, new_str: &str) -> GutterDecorations {
    let diff = TextDiff::from_lines(old_str, new_str);
    let mut lines = HashMap::new();

    for op in diff.ops() {
        match op.tag() {
            DiffTag::Equal => {}
            DiffTag::Insert => {
                for line_idx in op.new_range() {
                    lines.insert(line_idx, LineChangeKind::Added);
                }
            }
            DiffTag::Delete => {
                let target_line = op.new_range().start;
                lines
                    .entry(target_line)
                    .or_insert(LineChangeKind::DeletedAbove);
            }
            DiffTag::Replace => {
                let new_range = op.new_range();
                let old_range = op.old_range();
                let old_len = old_range.len();
                let new_len = new_range.len();

                let min_len = old_len.min(new_len);
                for i in 0..min_len {
                    lines.insert(new_range.start + i, LineChangeKind::Modified);
                }
                for i in min_len..new_len {
                    lines.insert(new_range.start + i, LineChangeKind::Added);
                }
                if old_len > new_len && !new_range.is_empty() {
                    lines.insert(new_range.end.saturating_sub(1), LineChangeKind::Modified);
                }
            }
        }
    }

    GutterDecorations { lines }
}

pub fn compute_file_diff(old_content: &str, new_content: &str) -> FileDiff {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut lines = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change
            .value()
            .trim_end_matches(&['\r', '\n'][..])
            .to_string();
        match change.tag() {
            ChangeTag::Equal => {
                lines.push(DiffLine {
                    kind: DiffLineKind::Context,
                    old_lineno: change.old_index().map(|i| i + 1),
                    new_lineno: change.new_index().map(|i| i + 1),
                    text,
                });
            }
            ChangeTag::Delete => {
                lines.push(DiffLine {
                    kind: DiffLineKind::Deleted,
                    old_lineno: change.old_index().map(|i| i + 1),
                    new_lineno: None,
                    text,
                });
            }
            ChangeTag::Insert => {
                lines.push(DiffLine {
                    kind: DiffLineKind::Added,
                    old_lineno: None,
                    new_lineno: change.new_index().map(|i| i + 1),
                    text,
                });
            }
        }
    }

    FileDiff { lines }
}
