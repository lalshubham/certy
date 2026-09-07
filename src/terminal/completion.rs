use super::tab::{TerminalLine, TerminalTab};
use std::path::PathBuf;

impl TerminalTab {
    pub fn tab_complete(&mut self, vis_rows: usize) {
        if self.is_running {
            return;
        }

        let chars: Vec<char> = self.current_input.chars().collect();
        let col = self.cursor_col.min(chars.len());

        let mut start = col;
        while start > 0 {
            let ch = chars[start - 1];
            if ch.is_whitespace() || ch == '"' || ch == '\'' {
                break;
            }
            start -= 1;
        }

        let token: String = chars[start..col].iter().collect();

        let (dir_part, prefix) = match token.rfind('/') {
            Some(idx) => (&token[..=idx], &token[idx + 1..]),
            None => ("", token.as_str()),
        };

        let search_dir = if dir_part.starts_with("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                PathBuf::from(home).join(&dir_part[2..])
            } else {
                self.cwd.join(dir_part)
            }
        } else if dir_part.starts_with('/') {
            PathBuf::from(dir_part)
        } else if !dir_part.is_empty() {
            self.cwd.join(dir_part)
        } else {
            self.cwd.clone()
        };

        let mut matches: Vec<(String, bool)> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if !prefix.starts_with('.') && name.starts_with('.') {
                    continue;
                }
                let is_match = if name.starts_with(prefix) {
                    true
                } else {
                    name.to_lowercase().starts_with(&prefix.to_lowercase())
                };

                if is_match {
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    matches.push((name, is_dir));
                }
            }
        }

        if matches.is_empty() {
            return;
        }

        matches.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        if matches.len() == 1 {
            let (match_name, is_dir) = &matches[0];
            let suffix = if *is_dir { "/" } else { " " };
            let replacement = format!("{dir_part}{match_name}{suffix}");
            let rep_chars: Vec<char> = replacement.chars().collect();

            let mut new_chars = Vec::new();
            new_chars.extend_from_slice(&chars[..start]);
            new_chars.extend_from_slice(&rep_chars);
            new_chars.extend_from_slice(&chars[col..]);

            self.cursor_col = start + rep_chars.len();
            self.current_input = new_chars.into_iter().collect();
        } else {
            let first_name = &matches[0].0;
            let mut common_prefix = first_name.clone();

            for (name, _) in &matches[1..] {
                let mut matched_len = 0;
                for (c1, c2) in common_prefix.chars().zip(name.chars()) {
                    if c1 == c2 {
                        matched_len += c1.len_utf8();
                    } else {
                        break;
                    }
                }
                common_prefix.truncate(matched_len);
                if common_prefix.is_empty() {
                    break;
                }
            }

            if common_prefix.len() > prefix.len() {
                let replacement = format!("{dir_part}{common_prefix}");
                let rep_chars: Vec<char> = replacement.chars().collect();

                let mut new_chars = Vec::new();
                new_chars.extend_from_slice(&chars[..start]);
                new_chars.extend_from_slice(&rep_chars);
                new_chars.extend_from_slice(&chars[col..]);

                self.cursor_col = start + rep_chars.len();
                self.current_input = new_chars.into_iter().collect();
            } else {
                let prompt_text = self.prompt();
                let full_cmd = format!("{prompt_text}{}", self.current_input);
                if full_cmd.chars().count() > self.max_line_len {
                    self.max_line_len = full_cmd.chars().count();
                }
                self.lines.push(TerminalLine {
                    prompt: None,
                    content: full_cmd,
                });

                let list = matches
                    .iter()
                    .map(|(name, is_dir)| {
                        if *is_dir {
                            format!("{name}/")
                        } else {
                            name.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("  ");

                if list.chars().count() > self.max_line_len {
                    self.max_line_len = list.chars().count();
                }
                self.lines.push(TerminalLine {
                    prompt: None,
                    content: list,
                });
                self.auto_scroll_to_bottom(vis_rows);
            }
        }
    }
}
