use crate::config::*;
use crate::editor::TabManager;
use crate::ui::canvas::{
    draw_close_icon_clipped, draw_solid_rect, draw_solid_rect_clipped, draw_string_clipped,
};
use crate::ui::font::FontManager;
use crate::ui::layout::ViewportLayout;

pub fn render_editor_tabs(
    frame: &mut [u32],
    fonts: &mut FontManager,
    tabs: &TabManager,
    layout: &ViewportLayout,
    screen_w: usize,
    screen_h: usize,
) {
    if tabs.tabs.is_empty() {
        return;
    }
    let cw = fonts.char_width;
    let lh = fonts.line_height;
    let tabbar_x = layout.content_left;
    let tabbar_w = screen_w.saturating_sub(tabbar_x);

    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        tabbar_x,
        0,
        tabbar_w,
        TAB_BAR_HEIGHT,
        COLOR_TABBAR_BG,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        tabbar_x,
        TAB_BAR_HEIGHT - 1,
        tabbar_w,
        1,
        COLOR_TAB_BORDER,
    );

    let mut cur_x = tabbar_x as i32 - tabs.scroll_x as i32;
    let tab_text_offset_y = (TAB_BAR_HEIGHT.saturating_sub(lh)) / 2;

    for (idx, tab) in tabs.tabs.iter().enumerate() {
        let is_active = Some(idx) == tabs.active_idx;
        let is_tab_hovered = tabs.hovered_tab == Some(idx);
        let is_close_hovered = tabs.hovered_close == Some(idx);
        let tw = tab.width(cw);
        let tab_x0 = cur_x;
        let tab_x1 = cur_x + tw as i32;
        cur_x += tw as i32;

        if tab_x1 <= tabbar_x as i32 || tab_x0 >= screen_w as i32 {
            continue;
        }

        let bg = if is_active {
            COLOR_TAB_ACTIVE_BG
        } else if is_tab_hovered {
            COLOR_TAB_INACTIVE_BG
        } else {
            COLOR_TABBAR_BG
        };

        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            tab_x0,
            0,
            tw,
            TAB_BAR_HEIGHT - 1,
            tabbar_x,
            screen_w,
            bg,
        );
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            tab_x1 - 1,
            0,
            1,
            TAB_BAR_HEIGHT - 1,
            tabbar_x,
            screen_w,
            COLOR_TAB_BORDER,
        );

        let text_color = if is_active {
            COLOR_TAB_TEXT_ACTIVE
        } else {
            COLOR_TAB_TEXT_INACTIVE
        };
        let dirty = if tab.buffer.is_modified { "* " } else { "" };
        let title_text = format!("{dirty}{}", tab.title);
        let text_clip_max = screen_w.min((tab_x1 - 26).max(0) as usize);

        draw_string_clipped(
            fonts,
            frame,
            &title_text,
            tab_x0 + 14,
            tab_text_offset_y as i32,
            tabbar_x,
            text_clip_max,
            screen_w,
            screen_h,
            text_color,
        );

        let icon_size = 8;
        let close_x = tab_x1 - 22;
        let close_y = (TAB_BAR_HEIGHT.saturating_sub(icon_size)) / 2;
        let close_color = if is_close_hovered {
            COLOR_TAB_CLOSE_HOVER
        } else {
            text_color
        };

        draw_close_icon_clipped(
            frame,
            screen_w,
            screen_h,
            close_x,
            close_y as i32,
            icon_size,
            tabbar_x,
            screen_w,
            close_color,
        );
    }
}
