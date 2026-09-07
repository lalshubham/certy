use std::path::Path;

pub const SYNTAX_KEYWORD: u32 = 0xFFC586C0;
pub const SYNTAX_TYPE: u32 = 0xFF4EC9B0;
pub const SYNTAX_FUNCTION: u32 = 0xFFDCDCAA;
pub const SYNTAX_STRING: u32 = 0xFFCE9178;
pub const SYNTAX_NUMBER: u32 = 0xFFB5CEA8;
pub const SYNTAX_COMMENT: u32 = 0xFF6A9955;
pub const SYNTAX_PREPROCESSOR: u32 = 0xFF569CD6;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    C,
    Cpp,
    CSharp,
    Java,
    JavaScript,
    TypeScript,
    Go,
    Php,
    Python,
    Markdown,
    Html,
    Json,
    Bash,
    PlainText,
}

impl Language {
    pub fn from_path(path: Option<&Path>) -> Self {
        let path = match path {
            Some(p) => p,
            None => return Language::PlainText,
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "rs" => Language::Rust,
            "c" | "h" => Language::C,
            "cpp" | "hpp" | "cc" | "cxx" => Language::Cpp,
            "cs" => Language::CSharp,
            "java" => Language::Java,
            "js" | "mjs" | "cjs" | "jsx" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "go" => Language::Go,
            "php" => Language::Php,
            "py" => Language::Python,
            "md" | "markdown" => Language::Markdown,
            "html" | "htm" | "xml" | "svg" => Language::Html,
            "json" => Language::Json,
            "sh" | "bash" | "zsh" => Language::Bash,
            _ => Language::PlainText,
        }
    }
}

pub fn is_keyword(word: &str, lang: Language) -> bool {
    match lang {
        Language::Rust => matches!(
            word,
            "as" | "async"
                | "await"
                | "break"
                | "const"
                | "continue"
                | "crate"
                | "dyn"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "Self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "type"
                | "unsafe"
                | "use"
                | "where"
                | "while"
                | "yield"
        ),
        Language::C | Language::Cpp => matches!(
            word,
            "auto"
                | "break"
                | "case"
                | "class"
                | "const"
                | "continue"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "enum"
                | "explicit"
                | "export"
                | "extern"
                | "false"
                | "for"
                | "friend"
                | "goto"
                | "if"
                | "inline"
                | "mutable"
                | "namespace"
                | "new"
                | "noexcept"
                | "nullptr"
                | "operator"
                | "override"
                | "private"
                | "protected"
                | "public"
                | "register"
                | "reinterpret_cast"
                | "return"
                | "sizeof"
                | "static"
                | "static_assert"
                | "static_cast"
                | "struct"
                | "switch"
                | "template"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typedef"
                | "typeid"
                | "typename"
                | "union"
                | "using"
                | "virtual"
                | "volatile"
                | "while"
        ),
        Language::CSharp => matches!(
            word,
            "abstract"
                | "as"
                | "async"
                | "await"
                | "base"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "default"
                | "delegate"
                | "do"
                | "else"
                | "enum"
                | "event"
                | "explicit"
                | "extern"
                | "false"
                | "finally"
                | "fixed"
                | "for"
                | "foreach"
                | "goto"
                | "if"
                | "implicit"
                | "in"
                | "interface"
                | "internal"
                | "is"
                | "lock"
                | "namespace"
                | "new"
                | "null"
                | "operator"
                | "out"
                | "override"
                | "params"
                | "private"
                | "protected"
                | "public"
                | "readonly"
                | "ref"
                | "return"
                | "sealed"
                | "sizeof"
                | "stackalloc"
                | "static"
                | "struct"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typeof"
                | "unchecked"
                | "unsafe"
                | "using"
                | "virtual"
                | "void"
                | "volatile"
                | "while"
        ),
        Language::Java => matches!(
            word,
            "abstract"
                | "assert"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "default"
                | "do"
                | "else"
                | "enum"
                | "extends"
                | "final"
                | "finally"
                | "for"
                | "goto"
                | "if"
                | "implements"
                | "import"
                | "instanceof"
                | "interface"
                | "native"
                | "new"
                | "null"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "return"
                | "static"
                | "strictfp"
                | "super"
                | "switch"
                | "synchronized"
                | "this"
                | "throw"
                | "throws"
                | "transient"
                | "try"
                | "void"
                | "volatile"
                | "while"
                | "true"
                | "false"
        ),
        Language::JavaScript | Language::TypeScript => matches!(
            word,
            "async"
                | "await"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "debugger"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "export"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "from"
                | "function"
                | "get"
                | "if"
                | "implements"
                | "import"
                | "in"
                | "instanceof"
                | "interface"
                | "let"
                | "new"
                | "null"
                | "of"
                | "return"
                | "set"
                | "static"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "type"
                | "typeof"
                | "undefined"
                | "var"
                | "void"
                | "while"
                | "with"
                | "yield"
        ),
        Language::Go => matches!(
            word,
            "break"
                | "case"
                | "chan"
                | "const"
                | "continue"
                | "default"
                | "defer"
                | "else"
                | "fallthrough"
                | "for"
                | "func"
                | "go"
                | "goto"
                | "if"
                | "import"
                | "interface"
                | "map"
                | "package"
                | "range"
                | "return"
                | "select"
                | "struct"
                | "switch"
                | "type"
                | "var"
                | "true"
                | "false"
                | "nil"
                | "iota"
        ),
        Language::Php => matches!(
            word,
            "abstract"
                | "and"
                | "array"
                | "as"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "clone"
                | "const"
                | "continue"
                | "declare"
                | "default"
                | "do"
                | "echo"
                | "else"
                | "elseif"
                | "empty"
                | "enddeclare"
                | "endfor"
                | "endforeach"
                | "endif"
                | "endswitch"
                | "endwhile"
                | "eval"
                | "exit"
                | "extends"
                | "final"
                | "finally"
                | "fn"
                | "for"
                | "foreach"
                | "function"
                | "global"
                | "goto"
                | "if"
                | "implements"
                | "include"
                | "include_once"
                | "instanceof"
                | "insteadof"
                | "interface"
                | "isset"
                | "list"
                | "match"
                | "namespace"
                | "new"
                | "or"
                | "print"
                | "private"
                | "protected"
                | "public"
                | "require"
                | "require_once"
                | "return"
                | "static"
                | "switch"
                | "throw"
                | "trait"
                | "try"
                | "use"
                | "var"
                | "while"
                | "xor"
                | "yield"
                | "true"
                | "false"
                | "null"
        ),
        Language::Python => matches!(
            word,
            "and"
                | "as"
                | "assert"
                | "async"
                | "await"
                | "break"
                | "class"
                | "continue"
                | "def"
                | "del"
                | "elif"
                | "else"
                | "except"
                | "False"
                | "finally"
                | "for"
                | "from"
                | "global"
                | "if"
                | "import"
                | "in"
                | "is"
                | "lambda"
                | "None"
                | "nonlocal"
                | "not"
                | "or"
                | "pass"
                | "raise"
                | "return"
                | "True"
                | "try"
                | "while"
                | "with"
                | "yield"
        ),
        Language::Bash => matches!(
            word,
            "case"
                | "do"
                | "done"
                | "elif"
                | "else"
                | "esac"
                | "fi"
                | "for"
                | "function"
                | "if"
                | "in"
                | "select"
                | "then"
                | "until"
                | "while"
                | "echo"
                | "exit"
                | "export"
                | "local"
                | "return"
                | "set"
                | "shift"
                | "source"
        ),
        Language::Json => matches!(word, "true" | "false" | "null"),
        _ => false,
    }
}

pub fn is_builtin_type(word: &str, lang: Language) -> bool {
    match lang {
        Language::Rust => matches!(
            word,
            "bool"
                | "char"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "str"
                | "String"
                | "Option"
                | "Some"
                | "None"
                | "Result"
                | "Ok"
                | "Err"
                | "Vec"
                | "Box"
                | "Rc"
                | "Arc"
                | "Cell"
                | "RefCell"
                | "HashMap"
                | "HashSet"
        ),
        Language::C | Language::Cpp => matches!(
            word,
            "bool"
                | "char"
                | "char16_t"
                | "char32_t"
                | "wchar_t"
                | "short"
                | "int"
                | "long"
                | "signed"
                | "unsigned"
                | "float"
                | "double"
                | "void"
                | "size_t"
                | "int8_t"
                | "int16_t"
                | "int32_t"
                | "int64_t"
                | "uint8_t"
                | "uint16_t"
                | "uint32_t"
                | "uint64_t"
                | "string"
                | "vector"
                | "map"
                | "set"
                | "pair"
        ),
        Language::CSharp => matches!(
            word,
            "bool"
                | "byte"
                | "sbyte"
                | "char"
                | "decimal"
                | "double"
                | "float"
                | "int"
                | "uint"
                | "nint"
                | "nuint"
                | "long"
                | "ulong"
                | "short"
                | "ushort"
                | "object"
                | "string"
                | "dynamic"
                | "var"
        ),
        Language::Java => matches!(
            word,
            "boolean"
                | "byte"
                | "char"
                | "short"
                | "int"
                | "long"
                | "float"
                | "double"
                | "String"
                | "Integer"
                | "Boolean"
                | "Long"
                | "Double"
                | "Float"
                | "List"
                | "Map"
                | "Set"
                | "ArrayList"
                | "HashMap"
        ),
        Language::JavaScript | Language::TypeScript => matches!(
            word,
            "Array"
                | "Boolean"
                | "Date"
                | "Error"
                | "Function"
                | "JSON"
                | "Map"
                | "Math"
                | "Number"
                | "Object"
                | "Promise"
                | "RegExp"
                | "Set"
                | "String"
                | "Symbol"
                | "console"
                | "document"
                | "window"
                | "any"
                | "boolean"
                | "never"
                | "number"
                | "string"
                | "unknown"
                | "void"
        ),
        Language::Go => matches!(
            word,
            "bool"
                | "byte"
                | "complex64"
                | "complex128"
                | "error"
                | "float32"
                | "float64"
                | "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "rune"
                | "string"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uintptr"
        ),
        Language::Python => matches!(
            word,
            "int"
                | "float"
                | "str"
                | "bool"
                | "list"
                | "dict"
                | "set"
                | "tuple"
                | "bytes"
                | "object"
                | "type"
                | "self"
        ),
        _ => false,
    }
}

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

    for line_idx in 0..limit {
        let line = text.line(line_idx);
        let chars: Vec<char> = line
            .chars()
            .take_while(|&c| c != '\n' && c != '\r')
            .collect();
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
                } else {
                    if chars[i] == '/' && i + 1 < len {
                        if chars[i + 1] == '/' {
                            break;
                        }
                        if chars[i + 1] == '*' {
                            in_comment = true;
                            i += 2;
                            continue;
                        }
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
            if lang == Language::Rust {
            } else {
                for k in i..len {
                    colors[k] = SYNTAX_COMMENT;
                }
                break;
            }
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
            let start = i;
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

            let color = if is_keyword(&word, lang) {
                SYNTAX_KEYWORD
            } else if is_builtin_type(&word, lang) {
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
