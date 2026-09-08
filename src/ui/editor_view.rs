use super::canvas::{
    draw_close_icon_clipped, draw_solid_rect, draw_solid_rect_clipped, draw_string_clipped,
};
use super::font::FontManager;
use crate::config::*;
use crate::editor::TabManager;
use crate::syntax::{self, Language};
use crate::ui::layout::{calc_thumb, ViewportLayout};

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

pub fn render_editor_buffer(
    frame: &mut [u32],
    fonts: &mut FontManager,
    tabs: &TabManager,
    layout: &ViewportLayout,
    total_lines: usize,
    screen_w: usize,
    screen_h: usize,
) {
    let tab = match tabs.active_tab() {
        Some(t) => t,
        None => return,
    };

    let cw = fonts.char_width;
    let lh = fonts.line_height;
    let buffer = &tab.buffer;
    let (cur_line, cur_col) = buffer.cursor_pos();
    let sel_range = buffer.selection_range();

    let gutter_x = layout.content_left;
    let gutter_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT) + SCROLLBAR_THICKNESS;
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        gutter_x,
        TAB_BAR_HEIGHT,
        layout.gutter_width,
        gutter_h,
        COLOR_GUTTER_BACKGROUND,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        gutter_x + layout.gutter_width,
        TAB_BAR_HEIGHT,
        1,
        gutter_h,
        COLOR_GUTTER_SEPARATOR,
    );

    let language = Language::from_path(buffer.file_path.as_deref());
    let mut in_comment_state =
        syntax::compute_initial_comment_state(buffer.text(), buffer.scroll_line, language);

    let digits = total_lines.to_string().len().max(3);
    for row in 0..=layout.visible_lines {
        let line_idx = buffer.scroll_line + row;
        if line_idx >= total_lines {
            break;
        }

        let y = TAB_BAR_HEIGHT + TOP_PADDING + row * lh;
        if y + lh > layout.content_bottom {
            break;
        }

        let num_str = format!("{:>width$}", line_idx + 1, width = digits);
        let num_color = if line_idx == cur_line {
            COLOR_LINE_NUMBER_ACTIVE
        } else {
            COLOR_LINE_NUMBER_MUTED
        };
        let mut nx = gutter_x + GUTTER_PADDING;
        for ch in num_str.chars() {
            fonts.draw_char(
                frame, ch, nx as i32, y as i32, screen_w, screen_h, num_color,
            );
            nx += cw;
        }

        let line = buffer.text().line(line_idx);
        let line_start_char = buffer.text().line_to_char(line_idx);
        let line_chars: Vec<char> = line
            .chars()
            .take_while(|&c| c != '\n' && c != '\r')
            .collect();

        let (syntax_colors, next_comment_state) =
            syntax::highlight_line(&line_chars, language, in_comment_state, COLOR_TEXT_DEFAULT);
        in_comment_state = next_comment_state;

        for (col_idx, &ch) in line_chars.iter().enumerate() {
            if col_idx < buffer.scroll_col {
                continue;
            }

            let text_x = layout.code_x + (col_idx - buffer.scroll_col) * cw;
            if text_x + cw > layout.content_right {
                break;
            }

            let char_idx = line_start_char + col_idx;
            if let Some((start, end)) = sel_range {
                if char_idx >= start && char_idx < end {
                    draw_solid_rect(
                        frame,
                        screen_w,
                        screen_h,
                        text_x,
                        y,
                        cw,
                        lh,
                        COLOR_SELECTION,
                    );
                }
            }

            let char_color = syntax_colors
                .get(col_idx)
                .copied()
                .unwrap_or(COLOR_TEXT_DEFAULT);

            fonts.draw_char(
                frame,
                ch,
                text_x as i32,
                y as i32,
                screen_w,
                screen_h,
                char_color,
            );
        }
    }

    if cur_line >= buffer.scroll_line
        && cur_line < buffer.scroll_line + layout.visible_lines
        && cur_col >= buffer.scroll_col
        && cur_col <= buffer.scroll_col + layout.visible_cols
    {
        let cx = layout.code_x + (cur_col - buffer.scroll_col) * cw;
        let cy = TAB_BAR_HEIGHT + TOP_PADDING + (cur_line - buffer.scroll_line) * lh;
        let max_y = (cy + lh).min(layout.content_bottom).min(screen_h);
        let max_x = (cx + 2).min(layout.content_right).min(screen_w);
        for y in cy.min(screen_h)..max_y {
            for x in cx.min(screen_w)..max_x {
                frame[y * screen_w + x] = COLOR_CURSOR;
            }
        }
    }

    let usable_track_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT);
    let vert_thumb = calc_thumb(
        total_lines,
        layout.visible_lines,
        buffer.scroll_line,
        usable_track_h,
    );
    let horiz_track_w = layout.content_right.saturating_sub(layout.bar_start_x);
    let horiz_thumb = calc_thumb(
        buffer.max_line_len,
        layout.visible_cols,
        buffer.scroll_col,
        horiz_track_w,
    );

    if let Some((ty, th)) = vert_thumb {
        let thumb_y = TAB_BAR_HEIGHT + ty;
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            layout.content_right,
            TAB_BAR_HEIGHT,
            SCROLLBAR_THICKNESS,
            usable_track_h,
            COLOR_BACKGROUND,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            layout.content_right,
            thumb_y,
            SCROLLBAR_THICKNESS,
            th,
            COLOR_SCROLLBAR_THUMB,
        );
    }
    if let Some((tx_offset, tw)) = horiz_thumb {
        let tx = layout.bar_start_x + tx_offset;
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            layout.bar_start_x,
            layout.content_bottom,
            horiz_track_w,
            SCROLLBAR_THICKNESS,
            COLOR_BACKGROUND,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            tx,
            layout.content_bottom,
            tw,
            SCROLLBAR_THICKNESS,
            COLOR_SCROLLBAR_THUMB,
        );
    }
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        layout.content_right,
        layout.content_bottom,
        SCROLLBAR_THICKNESS,
        SCROLLBAR_THICKNESS,
        COLOR_BACKGROUND,
    );
}
