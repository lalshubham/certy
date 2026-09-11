use std::path::Path;

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
    pub fn line_comment_prefix(&self) -> Option<&'static str> {
        match self {
            Language::Rust
            | Language::C
            | Language::Cpp
            | Language::CSharp
            | Language::Java
            | Language::JavaScript
            | Language::TypeScript
            | Language::Go
            | Language::Php => Some("//"),
            Language::Python | Language::Bash => Some("#"),
            _ => None,
        }
    }

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
