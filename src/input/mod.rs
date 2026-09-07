pub mod actions;
pub mod keyboard;
pub mod mouse;

pub use crate::ui::ContextMenu;
pub use actions::{ActionEvent, AppEvent};

use winit::keyboard::ModifiersState;

#[derive(Default, PartialEq)]
pub enum DragState {
    #[default]
    None,
    Vertical {
        start_y: f64,
        start_line: usize,
    },
    Horizontal {
        start_x: f64,
        start_col: usize,
    },
    SelectingText,
    SidebarResize {
        start_x: f64,
        start_w: usize,
    },
    SidebarScroll {
        start_y: f64,
        start_scroll: usize,
    },
    TerminalResize {
        start_y: f64,
        start_h: usize,
    },
    TerminalVertical {
        start_y: f64,
        start_line: usize,
    },
    TerminalHorizontal {
        start_x: f64,
        start_col: usize,
    },
    TerminalSelecting,
}

#[derive(Default)]
pub struct InputHandler {
    pub drag: DragState,
    pub is_left_down: bool,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub scroll_accum_y: f64,
    pub scroll_accum_x: f64,
    pub modifiers: ModifiersState,
    pub ctrl_down: bool,
    pub shift_down: bool,
    pub context_menu: Option<ContextMenu>,
}
