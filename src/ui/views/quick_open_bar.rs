use crate::config::*;
use crate::editor::TabManager;
use crate::ui::canvas::{
    draw_solid_rect, draw_solid_rect_clipped, draw_string_clipped, draw_string_ellipsis,
};
use crate::ui::font::FontManager;
use crate::ui::layout::ViewportLayout;

pub fn render_quick_open_bar(
    frame: &mut [u32],
    fonts: &mut FontManager,
    tabs: &TabManager,
    layout: &ViewportLayout,
    screen_w: usize,
    screen_h: usize,
    bar_y: usize,
) {
    let cw = fonts.char_width;
    let bar_h = 36;
    let bar_x = layout.content_left;
    let bar_w = screen_w.saturating_sub(bar_x);

    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        bar_x,
        bar_y,
        bar_w,
        1,
        COLOR_SIDEBAR_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        bar_x,
        bar_y + 1,
        bar_w,
        bar_h - 1,
        COLOR_TABBAR_BG,
    );

    let input_h: usize = 24;
    let input_y = bar_y + 6;
    let cap_h = (fonts.baseline_offset * 73) / 100;
    let text_y =
        input_y as i32 + (input_h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;

    let close_w = ("Close".len() * cw + 16) as i32;
    let close_btn_x = (bar_x + bar_w).saturating_sub(close_w as usize + 6);
    let is_close_hovered = tabs.quick_open.hovered_close;
    let close_bg = if is_close_hovered {
        COLOR_BTN_HOVER
    } else {
        COLOR_BTN_BG
    };

    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        close_btn_x,
        input_y,
        close_w as usize,
        input_h,
        close_bg,
    );
    let close_tx = close_btn_x as i32 + (close_w - ("Close".len() * cw) as i32) / 2;
    draw_string_clipped(
        fonts,
        frame,
        "Close",
        close_tx,
        text_y,
        close_btn_x,
        close_btn_x + close_w as usize,
        screen_w,
        screen_h,
        COLOR_TAB_TEXT_ACTIVE,
    );

    let input_x = bar_x + 6;
    let input_w = close_btn_x.saturating_sub(input_x + 6);
    let is_focused = tabs.quick_open.focused;
    let border_color = if is_focused {
        COLOR_CURSOR
    } else {
        COLOR_FIND_INPUT_BORDER
    };

    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        input_x,
        input_y,
        input_w,
        input_h,
        COLOR_FIND_INPUT_BG,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        input_x,
        input_y,
        input_w,
        1,
        border_color,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        input_x,
        input_y + input_h - 1,
        input_w,
        1,
        border_color,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        input_x,
        input_y,
        1,
        input_h,
        border_color,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        input_x + input_w - 1,
        input_y,
        1,
        input_h,
        border_color,
    );

    let padding = 6;
    let text_clip_left = input_x + padding;
    let text_clip_right = (input_x + input_w).saturating_sub(padding);
    let max_vis_chars = if cw > 0 {
        input_w.saturating_sub(padding * 2) / cw
    } else {
        10
    };

    if tabs.quick_open.query.is_empty() {
        draw_string_clipped(
            fonts,
            frame,
            "Search files by name (e.g. main.rs)...",
            (input_x + padding) as i32,
            text_y,
            text_clip_left,
            text_clip_right,
            screen_w,
            screen_h,
            COLOR_LINE_NUMBER_MUTED,
        );
        if is_focused {
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                input_x + padding,
                input_y + 3,
                2,
                18,
                COLOR_CURSOR,
            );
        }
    } else {
        let q_len = tabs.quick_open.query.chars().count();
        let cur = tabs.quick_open.cursor.min(q_len);
        let scroll_offset = tabs
            .quick_open
            .query_scroll
            .min(q_len.saturating_sub(max_vis_chars));
        let draw_x = (input_x + padding) as i32 - (scroll_offset * cw) as i32;

        if let Some(anchor) = tabs.quick_open.selection_anchor {
            let start = anchor.min(cur);
            let end = anchor.max(cur);
            if start < end {
                let sel_x = (input_x + padding) as i32
                    + ((start as i32 - scroll_offset as i32) * cw as i32);
                let sel_w = (end - start) * cw;
                draw_solid_rect_clipped(
                    frame,
                    screen_w,
                    screen_h,
                    sel_x,
                    input_y as i32 + 3,
                    sel_w,
                    18,
                    text_clip_left,
                    text_clip_right,
                    COLOR_SELECTION,
                );
            }
        }

        draw_string_clipped(
            fonts,
            frame,
            &tabs.quick_open.query,
            draw_x,
            text_y,
            text_clip_left,
            text_clip_right,
            screen_w,
            screen_h,
            COLOR_TAB_TEXT_ACTIVE,
        );

        if is_focused {
            let cx = (input_x + padding) as i32 + ((cur as i32 - scroll_offset as i32) * cw as i32);
            if cx >= text_clip_left as i32 && (cx + 2) <= text_clip_right as i32 {
                draw_solid_rect(
                    frame,
                    screen_w,
                    screen_h,
                    cx as usize,
                    input_y + 3,
                    2,
                    18,
                    COLOR_CURSOR,
                );
            }
        }
    }

    let max_visible_items = 8;
    let item_count = tabs.quick_open.matches.len().min(max_visible_items);
    if item_count > 0 {
        let row_h = 28;
        let popup_h = item_count * row_h;
        let popup_y = bar_y.saturating_sub(popup_h);
        let popup_x = bar_x;
        let popup_w = bar_w;

        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y,
            popup_w,
            popup_h,
            COLOR_MODAL_BG,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y,
            popup_w,
            1,
            COLOR_MODAL_BORDER,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y + popup_h - 1,
            popup_w,
            1,
            COLOR_MODAL_BORDER,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y,
            1,
            popup_h,
            COLOR_MODAL_BORDER,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x + popup_w - 1,
            popup_y,
            1,
            popup_h,
            COLOR_MODAL_BORDER,
        );

        let selected = tabs.quick_open.selected_match;
        let start_idx = if selected >= max_visible_items {
            selected - max_visible_items + 1
        } else {
            0
        };

        for i in 0..item_count {
            let idx = start_idx + i;
            if let Some(item) = tabs.quick_open.matches.get(idx) {
                let row_y = popup_y + i * row_h;
                let is_selected = idx == selected;
                let is_hovered = tabs.quick_open.hovered_match == Some(idx);
                let row_bg = if is_selected {
                    COLOR_SIDEBAR_ROW_ACTIVE
                } else if is_hovered {
                    COLOR_SIDEBAR_ROW_HOVER
                } else {
                    COLOR_MODAL_BG
                };

                if row_bg != COLOR_MODAL_BG {
                    draw_solid_rect(
                        frame,
                        screen_w,
                        screen_h,
                        popup_x + 1,
                        row_y + 1,
                        popup_w.saturating_sub(2),
                        row_h - 1,
                        row_bg,
                    );
                }

                let text_color = if is_selected {
                    COLOR_TAB_TEXT_ACTIVE
                } else {
                    COLOR_SIDEBAR_TEXT
                };
                let row_text_y =
                    row_y as i32 + (row_h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;

                draw_string_ellipsis(
                    fonts,
                    frame,
                    &item.relative_path,
                    (popup_x + 14) as i32,
                    row_text_y,
                    popup_x + popup_w - 14,
                    screen_w,
                    screen_h,
                    text_color,
                );
            }
        }
    } else if !tabs.quick_open.query.is_empty() {
        let row_h = 28;
        let popup_y = bar_y.saturating_sub(row_h);
        let popup_x = bar_x;
        let popup_w = bar_w;

        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y,
            popup_w,
            row_h,
            COLOR_MODAL_BG,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y,
            popup_w,
            1,
            COLOR_MODAL_BORDER,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y + row_h - 1,
            popup_w,
            1,
            COLOR_MODAL_BORDER,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x,
            popup_y,
            1,
            row_h,
            COLOR_MODAL_BORDER,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            popup_x + popup_w - 1,
            popup_y,
            1,
            row_h,
            COLOR_MODAL_BORDER,
        );

        let row_text_y =
            popup_y as i32 + (row_h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;
        draw_string_clipped(
            fonts,
            frame,
            "No matching files",
            (popup_x + 14) as i32,
            row_text_y,
            popup_x + 14,
            popup_x + popup_w - 14,
            screen_w,
            screen_h,
            COLOR_LINE_NUMBER_MUTED,
        );
    }
}
