pub mod handler;
pub mod lifecycle;

use crate::editor::TabManager;
use crate::input::{AppEvent, InputHandler};
use crate::sidebar::Sidebar;
use crate::terminal::Terminal;
use crate::ui::Renderer;
use std::sync::Arc;
use winit::event_loop::EventLoopProxy;
use winit::window::{CursorIcon, Window};

pub struct App {
    pub window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    pub tabs: TabManager,
    pub sidebar: Sidebar,
    pub terminal: Terminal,
    pub input: InputHandler,
    pub clipboard: Option<arboard::Clipboard>,
    pub active_cursor_icon: CursorIcon,
    pub current_title: String,
    pub event_proxy: EventLoopProxy<AppEvent>,
}

impl App {
    pub fn new(event_proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            window: None,
            renderer: None,
            tabs: TabManager::new(),
            sidebar: Sidebar::new(),
            terminal: Terminal::new(),
            input: InputHandler::default(),
            clipboard: arboard::Clipboard::new().ok(),
            active_cursor_icon: CursorIcon::Default,
            current_title: String::new(),
            event_proxy,
        }
    }
}
