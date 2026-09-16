pub mod comment_state;
pub mod highlighter;
pub mod keywords;
pub mod languages;

pub use comment_state::compute_initial_comment_state;
pub use highlighter::highlight_line;
pub use languages::Language;

pub const SYNTAX_KEYWORD: u32 = 0xFFC586C0;
pub const SYNTAX_TYPE: u32 = 0xFF4EC9B0;
pub const SYNTAX_FUNCTION: u32 = 0xFFDCDCAA;
pub const SYNTAX_STRING: u32 = 0xFFCE9178;
pub const SYNTAX_NUMBER: u32 = 0xFFB5CEA8;
pub const SYNTAX_COMMENT: u32 = 0xFF6A9955;
pub const SYNTAX_PREPROCESSOR: u32 = 0xFF569CD6;
