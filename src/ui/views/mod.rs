pub mod diff_view;
pub mod editor_buffer;
pub mod editor_tabs;
pub mod find_bar;
pub mod quick_open_bar;
pub mod sidebar_view;
pub mod terminal_view;

pub use diff_view::render_diff_view;
pub use editor_buffer::render_editor_buffer;
pub use editor_tabs::render_editor_tabs;
pub use sidebar_view::render_sidebar;
pub use terminal_view::render_terminal;
