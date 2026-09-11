mod keywords;
mod languages;

pub use languages::Language;

pub const SYNTAX_KEYWORD: u32 = 0xFFC586C0;
pub const SYNTAX_TYPE: u32 = 0xFF4EC9B0;
pub const SYNTAX_FUNCTION: u32 = 0xFFDCDCAA;
pub const SYNTAX_STRING: u32 = 0xFFCE9178;
pub const SYNTAX_NUMBER: u32 = 0xFFB5CEA8;
pub const SYNTAX_COMMENT: u32 = 0xFF6A9955;
pub const SYNTAX_PREPROCESSOR: u32 = 0xFF569CD6;

pub fn compute_initial_comment_state(
    text: &ropey::Rope,
    target_line: usize,
    lang: Language,
) -> bool {
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

pub fn highlight_line(
    chars: &[char],
    lang: Language,
    mut in_block_comment: bool,
    default_color: u32,
) -> (Vec<u32>, bool) {
    let len = chars.len();
    let mut colors = vec![default_color; len];
    if lang == Language::PlainText || len == 0 {
        return (colors, false);
    }
    if lang == Language::Markdown {
        let trimmed_start = chars.iter().take_while(|&&c| c == ' ' || c == '\t').count();
        if in_block_comment {
            if trimmed_start + 2 < len
                && chars[trimmed_start] == '`'
                && chars[trimmed_start + 1] == '`'
                && chars[trimmed_start + 2] == '`'
            {
                colors.fill(SYNTAX_STRING);
                return (colors, false);
            }
            colors.fill(SYNTAX_STRING);
            return (colors, true);
        }
        if trimmed_start < len && chars[trimmed_start] == '#' {
            colors.fill(SYNTAX_PREPROCESSOR);
            return (colors, false);
        }
        if trimmed_start + 2 < len
            && chars[trimmed_start] == '`'
            && chars[trimmed_start + 1] == '`'
            && chars[trimmed_start + 2] == '`'
        {
            colors.fill(SYNTAX_STRING);
            return (colors, true);
        }
        if trimmed_start < len && (chars[trimmed_start] == '>' || chars[trimmed_start] == '-') {
            colors[trimmed_start] = SYNTAX_KEYWORD;
        }
        let mut i = 0;
        while i < len {
            if chars[i] == '`' {
                let start = i;
                i += 1;
                while i < len && chars[i] != '`' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                for k in start..i {
                    colors[k] = SYNTAX_STRING;
                }
                continue;
            }
            if chars[i] == '*' {
                colors[i] = SYNTAX_KEYWORD;
            }
            if chars[i] == '[' || chars[i] == ']' || chars[i] == '(' || chars[i] == ')' {
                colors[i] = SYNTAX_TYPE;
            }
            i += 1;
        }
        return (colors, false);
    }
    let mut i = 0;
    while i < len {
        if in_block_comment {
            if lang == Language::Python {
                if i + 2 < len && chars[i] == '"' && chars[i + 1] == '"' && chars[i + 2] == '"' {
                    colors[i] = SYNTAX_COMMENT;
                    colors[i + 1] = SYNTAX_COMMENT;
                    colors[i + 2] = SYNTAX_COMMENT;
                    i += 3;
                    in_block_comment = false;
                } else {
                    colors[i] = SYNTAX_COMMENT;
                    i += 1;
                }
            } else if lang == Language::Html {
                if i + 2 < len && chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>' {
                    colors[i] = SYNTAX_COMMENT;
                    colors[i + 1] = SYNTAX_COMMENT;
                    colors[i + 2] = SYNTAX_COMMENT;
                    i += 3;
                    in_block_comment = false;
                } else {
                    colors[i] = SYNTAX_COMMENT;
                    i += 1;
                }
            } else if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                colors[i] = SYNTAX_COMMENT;
                colors[i + 1] = SYNTAX_COMMENT;
                i += 2;
                in_block_comment = false;
            } else {
                colors[i] = SYNTAX_COMMENT;
                i += 1;
            }
            continue;
        }
        if (lang == Language::Python || lang == Language::Bash || lang == Language::Php)
            && chars[i] == '#'
        {
            for k in i..len {
                colors[k] = SYNTAX_COMMENT;
            }
            break;
        }
        if lang == Language::Python
            && i + 2 < len
            && chars[i] == '"'
            && chars[i + 1] == '"'
            && chars[i + 2] == '"'
        {
            colors[i] = SYNTAX_COMMENT;
            colors[i + 1] = SYNTAX_COMMENT;
            colors[i + 2] = SYNTAX_COMMENT;
            i += 3;
            in_block_comment = true;
            continue;
        }
        if lang == Language::Html
            && i + 3 < len
            && chars[i] == '<'
            && chars[i + 1] == '!'
            && chars[i + 2] == '-'
            && chars[i + 3] == '-'
        {
            for k in i..i + 4 {
                colors[k] = SYNTAX_COMMENT;
            }
            i += 4;
            in_block_comment = true;
            continue;
        }
        if chars[i] == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                for k in i..len {
                    colors[k] = SYNTAX_COMMENT;
                }
                break;
            }
            if chars[i + 1] == '*' {
                colors[i] = SYNTAX_COMMENT;
                colors[i + 1] = SYNTAX_COMMENT;
                i += 2;
                in_block_comment = true;
                continue;
            }
        }
        if chars[i] == '"'
            || (matches!(lang, Language::JavaScript | Language::TypeScript) && chars[i] == '`')
            || (matches!(
                lang,
                Language::Python
                    | Language::Bash
                    | Language::Php
                    | Language::JavaScript
                    | Language::TypeScript
            ) && chars[i] == '\'')
        {
            let quote = chars[i];
            colors[i] = SYNTAX_STRING;
            i += 1;
            while i < len {
                colors[i] = SYNTAX_STRING;
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                    colors[i] = SYNTAX_STRING;
                } else if chars[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if (lang == Language::C || lang == Language::Cpp) && chars[i] == '#' {
            let start = i;
            while i < len && (chars[i].is_alphabetic() || chars[i] == '_' || chars[i] == '#') {
                i += 1;
            }
            for k in start..i {
                colors[k] = SYNTAX_PREPROCESSOR;
            }
            continue;
        }
        if lang == Language::Html && (chars[i] == '<' || chars[i] == '>') {
            colors[i] = SYNTAX_PREPROCESSOR;
            i += 1;
            continue;
        }
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < len
                && (chars[i].is_ascii_hexdigit()
                    || chars[i] == '.'
                    || chars[i] == '_'
                    || chars[i] == 'x'
                    || chars[i] == 'X')
            {
                i += 1;
            }
            for k in start..i {
                colors[k] = SYNTAX_NUMBER;
            }
            continue;
        }
        if chars[i].is_alphabetic() || chars[i] == '_' || chars[i] == '$' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let mut lookahead = i;
            while lookahead < len && (chars[lookahead] == ' ' || chars[lookahead] == '\t') {
                lookahead += 1;
            }
            let is_func = lookahead < len && chars[lookahead] == '(';
            let color = if keywords::is_keyword(&word, lang) {
                SYNTAX_KEYWORD
            } else if keywords::is_builtin_type(&word, lang) {
                SYNTAX_TYPE
            } else if is_func {
                SYNTAX_FUNCTION
            } else if word
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                SYNTAX_TYPE
            } else {
                default_color
            };
            for k in start..i {
                colors[k] = color;
            }
            continue;
        }
        i += 1;
    }
    (colors, in_block_comment)
}
