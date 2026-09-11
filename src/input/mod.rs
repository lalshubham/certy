pub mod actions;
pub mod keyboard;
pub mod mouse;

use crate::ui::ContextMenu;
pub use actions::{ActionEvent, AppEvent};
use std::time::Instant;
use winit::keyboard::ModifiersState;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DragState {
    None,
    SidebarResize { start_x: f64, start_w: usize },
    TerminalResize { start_y: f64, start_h: usize },
    TerminalSelecting,
    TerminalVertical { start_y: f64, start_line: usize },
    SidebarScroll { start_y: f64, start_scroll: usize },
    SelectingText,
    Vertical { start_y: f64, start_line: usize },
    Horizontal { start_x: f64, start_col: usize },
    FindSelecting,
    QuickOpenSelecting,
}

pub struct InputHandler {
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub is_left_down: bool,
    pub drag: DragState,
    pub ctrl_down: bool,
    pub shift_down: bool,
    pub modifiers: ModifiersState,
    pub context_menu: Option<ContextMenu>,
    pub last_click_time: Option<Instant>,
    pub last_click_pos: (f64, f64),
    pub click_count: usize,
    pub scroll_accum_x: f64,
    pub scroll_accum_y: f64,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self {
            mouse_x: 0.0,
            mouse_y: 0.0,
            is_left_down: false,
            drag: DragState::None,
            ctrl_down: false,
            shift_down: false,
            modifiers: ModifiersState::default(),
            context_menu: None,
            last_click_time: None,
            last_click_pos: (0.0, 0.0),
            click_count: 0,
            scroll_accum_x: 0.0,
            scroll_accum_y: 0.0,
        }
    }
}
