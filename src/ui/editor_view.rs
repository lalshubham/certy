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

            if tabs.find.is_open && !tabs.find.matches.is_empty() {
                for (m_idx, &(m_start, m_end)) in tabs.find.matches.iter().enumerate() {
                    if char_idx >= m_start && char_idx < m_end {
                        let color = if tabs.find.active_match_idx == Some(m_idx) {
                            COLOR_FIND_ACTIVE
                        } else {
                            COLOR_FIND_MATCH
                        };
                        draw_solid_rect(frame, screen_w, screen_h, text_x, y, cw, lh, color);
                        break;
                    }
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
    let virtual_total_lines = total_lines + layout.visible_lines.saturating_sub(1);

    let vert_thumb = calc_thumb(
        virtual_total_lines,
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

    if tabs.find.is_open {
        render_find_bar(frame, fonts, tabs, layout, screen_w, screen_h);
    }
}

fn render_find_bar(
    frame: &mut [u32],
    fonts: &mut FontManager,
    tabs: &TabManager,
    layout: &ViewportLayout,
    screen_w: usize,
    screen_h: usize,
) {
    let cw = fonts.char_width;
    let bar_h = if tabs.find.is_replace { 66 } else { 36 };
    let bar_x = layout.content_left;
    let bar_w = screen_w.saturating_sub(bar_x);
    let bar_y = layout.content_bottom + SCROLLBAR_THICKNESS;

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
    let bottom_row_y = if tabs.find.is_replace {
        bar_y + 36
    } else {
        bar_y + 6
    };

    let cap_h = (fonts.baseline_offset * 73) / 100;
    let text_y_top =
        input_y as i32 + (input_h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;
    let text_y_bottom =
        bottom_row_y as i32 + (input_h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;

    let close_w = ("Close".len() * cw + 16) as i32;
    let close_btn_x = (bar_x + bar_w).saturating_sub(close_w as usize + 6);
    let is_close_hovered = tabs.find.hovered_btn == Some(16);
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
        bottom_row_y,
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
        text_y_bottom,
        close_btn_x,
        close_btn_x + close_w as usize,
        screen_w,
        screen_h,
        COLOR_TAB_TEXT_ACTIVE,
    );

    let strip_min_x = bar_x;
    let strip_max_x = close_btn_x.saturating_sub(6);

    let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
    let toggle_w = (toggle_label.len() * cw + 6) as i32;
    let toggle_x = strip_min_x as i32 + 6;
    let toggle_tx = toggle_x + (toggle_w - (toggle_label.len() * cw) as i32) / 2;
    draw_string_clipped(
        fonts,
        frame,
        toggle_label,
        toggle_tx,
        text_y_bottom,
        strip_min_x,
        strip_max_x,
        screen_w,
        screen_h,
        COLOR_LINE_NUMBER_ACTIVE,
    );

    let scrollable_min_x = toggle_x as usize + toggle_w as usize + 6;
    let mut cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
    let find_input_w = 240;

    let is_find_focused =
        tabs.find.focused && tabs.find.active_field == crate::editor::find::FindField::Find;
    let border_find = if is_find_focused {
        COLOR_CURSOR
    } else {
        COLOR_FIND_INPUT_BORDER
    };

    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        find_input_w,
        input_h,
        scrollable_min_x,
        strip_max_x,
        COLOR_FIND_INPUT_BG,
    );
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        find_input_w,
        1,
        scrollable_min_x,
        strip_max_x,
        border_find,
    );
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        (input_y + input_h - 1) as i32,
        find_input_w,
        1,
        scrollable_min_x,
        strip_max_x,
        border_find,
    );
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        1,
        input_h,
        scrollable_min_x,
        strip_max_x,
        border_find,
    );
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x + find_input_w as i32 - 1,
        input_y as i32,
        1,
        input_h,
        scrollable_min_x,
        strip_max_x,
        border_find,
    );

    let padding = 4;
    let text_clip_left = (cur_x + padding as i32).max(scrollable_min_x as i32) as usize;
    let text_clip_right = (cur_x + find_input_w as i32 - padding as i32)
        .min(strip_max_x as i32)
        .max(0) as usize;
    let max_vis_chars = if cw > 0 {
        find_input_w.saturating_sub(padding * 2) / cw
    } else {
        10
    };

    if tabs.find.query.is_empty() {
        draw_string_clipped(
            fonts,
            frame,
            "Find",
            cur_x + padding as i32,
            text_y_top,
            text_clip_left,
            text_clip_right,
            screen_w,
            screen_h,
            COLOR_LINE_NUMBER_MUTED,
        );
        if is_find_focused
            && (cur_x + padding as i32) >= scrollable_min_x as i32
            && (cur_x + padding as i32 + 2) <= strip_max_x as i32
        {
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                (cur_x + padding as i32) as usize,
                input_y + 3,
                2,
                18,
                COLOR_CURSOR,
            );
        }
    } else {
        let q_len = tabs.find.query.chars().count();
        let cur = tabs.find.query_cursor.min(q_len);
        let scroll_offset = tabs
            .find
            .query_scroll
            .min(q_len.saturating_sub(max_vis_chars));

        let draw_x = cur_x + padding as i32 - (scroll_offset * cw) as i32;

        if let Some(anchor) = tabs.find.query_selection_anchor {
            let start = anchor.min(cur);
            let end = anchor.max(cur);
            if start < end {
                let sel_x =
                    cur_x + padding as i32 + ((start as i32 - scroll_offset as i32) * cw as i32);
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
            &tabs.find.query,
            draw_x,
            text_y_top,
            text_clip_left,
            text_clip_right,
            screen_w,
            screen_h,
            COLOR_TAB_TEXT_ACTIVE,
        );

        if is_find_focused {
            let cx = cur_x + padding as i32 + ((cur as i32 - scroll_offset as i32) * cw as i32);
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

    cur_x += find_input_w as i32 + 6;

    let mc_label = "Match Case";
    let mc_w = (mc_label.len() * cw + 16) as i32;
    let mc_active = tabs.find.match_case;
    let mc_hovered = tabs.find.hovered_btn == Some(11);
    let mc_bg = if mc_active {
        COLOR_FIND_TOGGLE_ACTIVE_BG
    } else if mc_hovered {
        COLOR_BTN_HOVER
    } else {
        COLOR_BTN_BG
    };
    let mc_fg = if mc_active {
        0xFFFFFFFF
    } else {
        COLOR_SIDEBAR_TEXT
    };
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        mc_w as usize,
        input_h,
        scrollable_min_x,
        strip_max_x,
        mc_bg,
    );
    let mc_tx = cur_x + (mc_w - (mc_label.len() * cw) as i32) / 2;
    draw_string_clipped(
        fonts,
        frame,
        mc_label,
        mc_tx,
        text_y_top,
        scrollable_min_x,
        strip_max_x,
        screen_w,
        screen_h,
        mc_fg,
    );
    cur_x += mc_w + 6;

    let ww_label = "Whole Word";
    let ww_w = (ww_label.len() * cw + 16) as i32;
    let ww_active = tabs.find.whole_word;
    let ww_hovered = tabs.find.hovered_btn == Some(12);
    let ww_bg = if ww_active {
        COLOR_FIND_TOGGLE_ACTIVE_BG
    } else if ww_hovered {
        COLOR_BTN_HOVER
    } else {
        COLOR_BTN_BG
    };
    let ww_fg = if ww_active {
        0xFFFFFFFF
    } else {
        COLOR_SIDEBAR_TEXT
    };
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        ww_w as usize,
        input_h,
        scrollable_min_x,
        strip_max_x,
        ww_bg,
    );
    let ww_tx = cur_x + (ww_w - (ww_label.len() * cw) as i32) / 2;
    draw_string_clipped(
        fonts,
        frame,
        ww_label,
        ww_tx,
        text_y_top,
        scrollable_min_x,
        strip_max_x,
        screen_w,
        screen_h,
        ww_fg,
    );
    cur_x += ww_w + 6;

    let re_label = "Regex";
    let re_w = (re_label.len() * cw + 16) as i32;
    let re_active = tabs.find.use_regex;
    let re_hovered = tabs.find.hovered_btn == Some(13);
    let re_bg = if re_active {
        COLOR_FIND_TOGGLE_ACTIVE_BG
    } else if re_hovered {
        COLOR_BTN_HOVER
    } else {
        COLOR_BTN_BG
    };
    let re_fg = if re_active {
        0xFFFFFFFF
    } else {
        COLOR_SIDEBAR_TEXT
    };
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        re_w as usize,
        input_h,
        scrollable_min_x,
        strip_max_x,
        re_bg,
    );
    let re_tx = cur_x + (re_w - (re_label.len() * cw) as i32) / 2;
    draw_string_clipped(
        fonts,
        frame,
        re_label,
        re_tx,
        text_y_top,
        scrollable_min_x,
        strip_max_x,
        screen_w,
        screen_h,
        re_fg,
    );
    cur_x += re_w + 6;

    let has_matches = !tabs.find.matches.is_empty();

    let prev_w = ("Previous".len() * cw + 16) as i32;
    let prev_hovered = tabs.find.hovered_btn == Some(14) && has_matches;
    let prev_bg = if prev_hovered {
        COLOR_BTN_HOVER
    } else if has_matches {
        COLOR_BTN_BG
    } else {
        0xFF222222
    };
    let prev_fg = if has_matches {
        COLOR_TAB_TEXT_ACTIVE
    } else {
        COLOR_LINE_NUMBER_MUTED
    };
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        prev_w as usize,
        input_h,
        scrollable_min_x,
        strip_max_x,
        prev_bg,
    );
    let prev_tx = cur_x + (prev_w - ("Previous".len() * cw) as i32) / 2;
    draw_string_clipped(
        fonts,
        frame,
        "Previous",
        prev_tx,
        text_y_top,
        scrollable_min_x,
        strip_max_x,
        screen_w,
        screen_h,
        prev_fg,
    );
    cur_x += prev_w + 6;

    let next_w = ("Next".len() * cw + 16) as i32;
    let next_hovered = tabs.find.hovered_btn == Some(15) && has_matches;
    let next_bg = if next_hovered {
        COLOR_BTN_HOVER
    } else if has_matches {
        COLOR_BTN_BG
    } else {
        0xFF222222
    };
    let next_fg = if has_matches {
        COLOR_TAB_TEXT_ACTIVE
    } else {
        COLOR_LINE_NUMBER_MUTED
    };
    draw_solid_rect_clipped(
        frame,
        screen_w,
        screen_h,
        cur_x,
        input_y as i32,
        next_w as usize,
        input_h,
        scrollable_min_x,
        strip_max_x,
        next_bg,
    );
    let next_tx = cur_x + (next_w - ("Next".len() * cw) as i32) / 2;
    draw_string_clipped(
        fonts,
        frame,
        "Next",
        next_tx,
        text_y_top,
        scrollable_min_x,
        strip_max_x,
        screen_w,
        screen_h,
        next_fg,
    );
    cur_x += next_w + 6;

    let counter_str = tabs.find.counter_text();
    if !counter_str.is_empty() {
        let counter_fg = if !has_matches {
            0xFFFF7B72
        } else {
            COLOR_SIDEBAR_TEXT
        };
        draw_string_clipped(
            fonts,
            frame,
            &counter_str,
            cur_x,
            text_y_top,
            scrollable_min_x,
            strip_max_x,
            screen_w,
            screen_h,
            counter_fg,
        );
    }

    if tabs.find.is_replace {
        let rep_input_y = bottom_row_y;
        let mut r_cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
        let rep_input_w = 240;

        let is_rep_focused =
            tabs.find.focused && tabs.find.active_field == crate::editor::find::FindField::Replace;
        let border_rep = if is_rep_focused {
            COLOR_CURSOR
        } else {
            COLOR_FIND_INPUT_BORDER
        };

        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            r_cur_x,
            rep_input_y as i32,
            rep_input_w,
            input_h,
            scrollable_min_x,
            strip_max_x,
            COLOR_FIND_INPUT_BG,
        );
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            r_cur_x,
            rep_input_y as i32,
            rep_input_w,
            1,
            scrollable_min_x,
            strip_max_x,
            border_rep,
        );
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            r_cur_x,
            rep_input_y as i32 + input_h as i32 - 1,
            rep_input_w,
            1,
            scrollable_min_x,
            strip_max_x,
            border_rep,
        );
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            r_cur_x,
            rep_input_y as i32,
            1,
            input_h,
            scrollable_min_x,
            strip_max_x,
            border_rep,
        );
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            r_cur_x + rep_input_w as i32 - 1,
            rep_input_y as i32,
            1,
            input_h,
            scrollable_min_x,
            strip_max_x,
            border_rep,
        );

        let padding = 4;
        let rep_clip_left = (r_cur_x + padding as i32).max(scrollable_min_x as i32) as usize;
        let rep_clip_right = (r_cur_x + rep_input_w as i32 - padding as i32)
            .min(strip_max_x as i32)
            .max(0) as usize;
        let max_vis_chars = if cw > 0 {
            rep_input_w.saturating_sub(padding * 2) / cw
        } else {
            10
        };

        if tabs.find.replace_text.is_empty() {
            draw_string_clipped(
                fonts,
                frame,
                "Replace",
                r_cur_x + padding as i32,
                text_y_bottom,
                rep_clip_left,
                rep_clip_right,
                screen_w,
                screen_h,
                COLOR_LINE_NUMBER_MUTED,
            );
            if is_rep_focused
                && (r_cur_x + padding as i32) >= scrollable_min_x as i32
                && (r_cur_x + padding as i32 + 2) <= strip_max_x as i32
            {
                draw_solid_rect(
                    frame,
                    screen_w,
                    screen_h,
                    (r_cur_x + padding as i32) as usize,
                    rep_input_y + 3,
                    2,
                    18,
                    COLOR_CURSOR,
                );
            }
        } else {
            let r_len = tabs.find.replace_text.chars().count();
            let cur = tabs.find.replace_cursor.min(r_len);
            let scroll_offset = tabs
                .find
                .replace_scroll
                .min(r_len.saturating_sub(max_vis_chars));

            let draw_x = r_cur_x + padding as i32 - (scroll_offset * cw) as i32;

            if let Some(anchor) = tabs.find.replace_selection_anchor {
                let start = anchor.min(cur);
                let end = anchor.max(cur);
                if start < end {
                    let sel_x = r_cur_x
                        + padding as i32
                        + ((start as i32 - scroll_offset as i32) * cw as i32);
                    let sel_w = (end - start) * cw;
                    draw_solid_rect_clipped(
                        frame,
                        screen_w,
                        screen_h,
                        sel_x,
                        rep_input_y as i32 + 3,
                        sel_w,
                        18,
                        rep_clip_left,
                        rep_clip_right,
                        COLOR_SELECTION,
                    );
                }
            }

            draw_string_clipped(
                fonts,
                frame,
                &tabs.find.replace_text,
                draw_x,
                text_y_bottom,
                rep_clip_left,
                rep_clip_right,
                screen_w,
                screen_h,
                COLOR_TAB_TEXT_ACTIVE,
            );

            if is_rep_focused {
                let cx =
                    r_cur_x + padding as i32 + ((cur as i32 - scroll_offset as i32) * cw as i32);
                if cx >= rep_clip_left as i32 && (cx + 2) <= rep_clip_right as i32 {
                    draw_solid_rect(
                        frame,
                        screen_w,
                        screen_h,
                        cx as usize,
                        rep_input_y + 3,
                        2,
                        18,
                        COLOR_CURSOR,
                    );
                }
            }
        }

        r_cur_x += rep_input_w as i32 + 6;

        let rep_w = ("Replace".len() * cw + 16) as i32;
        let rep_hovered = tabs.find.hovered_btn == Some(20) && has_matches;
        let rep_bg = if rep_hovered {
            COLOR_BTN_HOVER
        } else if has_matches {
            COLOR_BTN_BG
        } else {
            0xFF222222
        };
        let rep_fg = if has_matches {
            COLOR_TAB_TEXT_ACTIVE
        } else {
            COLOR_LINE_NUMBER_MUTED
        };
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            r_cur_x,
            rep_input_y as i32,
            rep_w as usize,
            input_h,
            scrollable_min_x,
            strip_max_x,
            rep_bg,
        );
        let rep_tx = r_cur_x + (rep_w - ("Replace".len() * cw) as i32) / 2;
        draw_string_clipped(
            fonts,
            frame,
            "Replace",
            rep_tx,
            text_y_bottom,
            scrollable_min_x,
            strip_max_x,
            screen_w,
            screen_h,
            rep_fg,
        );
        r_cur_x += rep_w + 6;

        let all_w = ("Replace All".len() * cw + 16) as i32;
        let all_hovered = tabs.find.hovered_btn == Some(21) && has_matches;
        let all_bg = if all_hovered {
            COLOR_BTN_HOVER
        } else if has_matches {
            COLOR_BTN_BG
        } else {
            0xFF222222
        };
        let all_fg = if has_matches {
            COLOR_TAB_TEXT_ACTIVE
        } else {
            COLOR_LINE_NUMBER_MUTED
        };
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            r_cur_x,
            rep_input_y as i32,
            all_w as usize,
            input_h,
            scrollable_min_x,
            strip_max_x,
            all_bg,
        );
        let all_tx = r_cur_x + (all_w - ("Replace All".len() * cw) as i32) / 2;
        draw_string_clipped(
            fonts,
            frame,
            "Replace All",
            all_tx,
            text_y_bottom,
            scrollable_min_x,
            strip_max_x,
            screen_w,
            screen_h,
            all_fg,
        );
    }
}
