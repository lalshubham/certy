pub mod canvas;
pub mod editor_view;
pub mod font;
pub mod layout;
pub mod overlays;
pub mod sidebar_view;
pub mod terminal_view;

pub use font::FontManager;
pub use layout::{compute_layout, ViewportLayout};
pub use overlays::ContextMenu;

use crate::config::COLOR_BACKGROUND;
use crate::editor::TabManager;
use crate::sidebar::Sidebar;
use crate::terminal::Terminal;
use editor_view::{render_editor_buffer, render_editor_tabs};
use overlays::{compute_modal_layout, render_context_menu, render_modal};
use sidebar_view::render_sidebar;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::sync::Arc;
use terminal_view::render_terminal;
use winit::window::Window;

pub struct Renderer {
    _ctx: Context<Arc<Window>>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    pub width: usize,
    pub height: usize,
    pub font_manager: FontManager,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let ctx = Context::new(window.clone()).expect("Failed to init softbuffer");
        let mut surface = Surface::new(&ctx, window.clone()).expect("Failed to create surface");
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        if let (Some(wnz), Some(hnz)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            surface
                .resize(wnz, hnz)
                .expect("Failed to set surface size");
        }
        Self {
            _ctx: ctx,
            surface,
            width: w as usize,
            height: h as usize,
            font_manager: FontManager::new(),
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.width = w as usize;
        self.height = h as usize;
        if let (Some(wnz), Some(hnz)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            self.surface
                .resize(wnz, hnz)
                .expect("Failed to resize surface");
        }
    }

    pub fn layout(
        &self,
        total_lines: usize,
        sidebar_w: usize,
        terminal_h: usize,
        bottom_bars_h: usize,
    ) -> ViewportLayout {
        let effective_h = self.height.saturating_sub(terminal_h + bottom_bars_h);
        compute_layout(
            self.width,
            effective_h,
            self.font_manager.char_width,
            self.font_manager.line_height,
            total_lines,
            sidebar_w,
        )
    }

    pub fn render(
        &mut self,
        tabs: &TabManager,
        sidebar: &Sidebar,
        terminal: &Terminal,
        context_menu: Option<&ContextMenu>,
    ) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let screen_w = self.width;
        let screen_h = self.height;
        let term_h = if terminal.is_open { terminal.height } else { 0 };
        let find_h = if tabs.find.is_open {
            if tabs.find.is_replace {
                66
            } else {
                36
            }
        } else {
            0
        };
        let quick_open_h = if tabs.quick_open.is_open { 36 } else { 0 };
        let total_lines = tabs
            .active_tab()
            .map(|t| t.buffer.text().len_lines())
            .unwrap_or(0);
        let sidebar_w = if sidebar.visible { sidebar.width } else { 0 };
        let layout = self.layout(total_lines, sidebar_w, term_h, find_h + quick_open_h);
        let mut frame = self.surface.buffer_mut().expect("Failed to get buffer");
        frame.fill(COLOR_BACKGROUND);
        render_sidebar(
            &mut frame,
            &mut self.font_manager,
            sidebar,
            tabs,
            terminal,
            screen_w,
            screen_h,
        );
        render_editor_tabs(
            &mut frame,
            &mut self.font_manager,
            tabs,
            &layout,
            screen_w,
            screen_h,
        );
        render_editor_buffer(
            &mut frame,
            &mut self.font_manager,
            tabs,
            &layout,
            total_lines,
            screen_w,
            screen_h,
        );
        render_terminal(
            &mut frame,
            &mut self.font_manager,
            terminal,
            &layout,
            screen_w,
            screen_h,
        );
        if let Some(modal) = compute_modal_layout(
            tabs,
            screen_w,
            screen_h,
            self.font_manager.char_width,
            self.font_manager.line_height,
        ) {
            render_modal(
                &mut frame,
                &mut self.font_manager,
                &modal,
                tabs.hovered_modal_btn,
                screen_w,
                screen_h,
            );
        }
        if let Some(menu) = context_menu {
            render_context_menu(
                &mut frame,
                &mut self.font_manager,
                menu,
                sidebar.visible,
                tabs.tabs.len(),
                screen_w,
                screen_h,
            );
        }
        frame.present().expect("Failed to present frame");
    }
}
