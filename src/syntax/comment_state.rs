use super::languages::Language;
use ropey::Rope;

pub fn compute_initial_comment_state(text: &Rope, target_line: usize, lang: Language) -> bool {
    if matches!(lang, Language::PlainText | Language::Bash | Language::Json) {
        return false;
    }
    let limit = target_line.min(text.len_lines());
    let mut in_comment = false;
    let mut chars = Vec::with_capacity(128);
    for line_idx in 0..limit {
        chars.clear();
        let line = text.line(line_idx);
        chars.extend(line.chars().take_while(|&c| c != '\n' && c != '\r'));
        let len = chars.len();
        let mut i = 0;
        while i < len {
            if in_comment {
                if lang == Language::Python {
                    if i + 2 < len && chars[i] == '"' && chars[i + 1] == '"' && chars[i + 2] == '"'
                    {
                        in_comment = false;
                        i += 3;
                        continue;
                    }
                } else if lang == Language::Html {
                    if i + 2 < len && chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>'
                    {
                        in_comment = false;
                        i += 3;
                        continue;
                    }
                } else if lang == Language::Markdown {
                    if i + 2 < len && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`'
                    {
                        in_comment = false;
                        i += 3;
                        continue;
                    }
                } else if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                    in_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
            } else {
                if lang == Language::Python {
                    if chars[i] == '#' {
                        break;
                    }
                    if i + 2 < len && chars[i] == '"' && chars[i + 1] == '"' && chars[i + 2] == '"'
                    {
                        in_comment = true;
                        i += 3;
                        continue;
                    }
                } else if lang == Language::Html {
                    if i + 3 < len
                        && chars[i] == '<'
                        && chars[i + 1] == '!'
                        && chars[i + 2] == '-'
                        && chars[i + 3] == '-'
                    {
                        in_comment = true;
                        i += 4;
                        continue;
                    }
                } else if lang == Language::Markdown {
                    if i + 2 < len && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`'
                    {
                        in_comment = true;
                        i += 3;
                        continue;
                    }
                } else if chars[i] == '/' && i + 1 < len {
                    if chars[i + 1] == '/' {
                        break;
                    }
                    if chars[i + 1] == '*' {
                        in_comment = true;
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
        }
    }
    in_comment
}
