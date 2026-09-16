use crate::config::*;
use crate::ui::canvas::{draw_solid_rect, draw_string};
use crate::ui::font::FontManager;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ContextMenu {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub hovered_idx: Option<usize>,
}

pub fn render_context_menu(
    frame: &mut [u32],
    fonts: &mut FontManager,
    menu: &ContextMenu,
    sidebar_visible: bool,
    tab_count: usize,
    screen_w: usize,
    screen_h: usize,
) {
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y,
        menu.width,
        menu.height,
        COLOR_MODAL_BG,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y,
        menu.width,
        1,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y + menu.height - 1,
        menu.width,
        1,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y,
        1,
        menu.height,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x + menu.width - 1,
        menu.y,
        1,
        menu.height,
        COLOR_MODAL_BORDER,
    );

    let row_h = menu.height / 2;
    let sidebar_label = if sidebar_visible {
        "Close Sidebar"
    } else {
        "Open Sidebar"
    };
    let close_files_label = if tab_count <= 1 {
        "Close File"
    } else {
        "Close Files"
    };
    let has_files = tab_count > 0;
    let items = [(sidebar_label, true), (close_files_label, has_files)];
    let cap_h = (fonts.baseline_offset * 73) / 100;
    for (i, (label, enabled)) in items.iter().enumerate() {
        let item_y = menu.y + i * row_h;
        if *enabled && menu.hovered_idx == Some(i) {
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                menu.x + 1,
                item_y + 1,
                menu.width.saturating_sub(2),
                row_h.saturating_sub(1),
                COLOR_SIDEBAR_ROW_HOVER,
            );
        }
        let ty = item_y as i32 + (row_h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;
        let text_color = if *enabled {
            COLOR_TAB_TEXT_ACTIVE
        } else {
            COLOR_LINE_NUMBER_MUTED
        };
        draw_string(
            fonts,
            frame,
            label,
            menu.x as i32 + 12,
            ty,
            screen_w,
            screen_h,
            text_color,
        );
    }
}
