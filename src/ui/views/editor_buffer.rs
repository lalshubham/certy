use super::find_bar::render_find_bar;
use super::quick_open_bar::render_quick_open_bar;
use crate::config::*;
use crate::editor::TabManager;
use crate::syntax::{self, Language};
use crate::ui::canvas::draw_solid_rect;
use crate::ui::font::FontManager;
use crate::ui::layout::{calc_thumb, ViewportLayout};

pub fn render_editor_buffer(
    frame: &mut [u32],
    fonts: &mut FontManager,
    tabs: &TabManager,
    layout: &ViewportLayout,
    total_lines: usize,
    screen_w: usize,
    screen_h: usize,
) {
    if let Some(tab) = tabs.active_tab() {
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
        let digits = if total_lines == 0 {
            1
        } else {
            (total_lines.ilog10() + 1) as usize
        }
        .max(3);

        let mut line_chars = Vec::with_capacity(128);
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
            line_chars.clear();
            line_chars.extend(line.chars().take_while(|&c| c != '\n' && c != '\r'));
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
    }

    let find_h = if tabs.find.is_open {
        if tabs.find.is_replace {
            66
        } else {
            36
        }
    } else {
        0
    };
    let qo_h = if tabs.quick_open.is_open { 36 } else { 0 };
    let base_y = layout.content_bottom + SCROLLBAR_THICKNESS;
    let (find_y, qo_y) = if tabs.find.is_open && tabs.quick_open.is_open {
        if tabs.quick_open_above_find {
            (base_y + qo_h, base_y)
        } else {
            (base_y, base_y + find_h)
        }
    } else {
        (base_y, base_y)
    };

    if tabs.find.is_open {
        render_find_bar(frame, fonts, tabs, layout, screen_w, screen_h, find_y);
    }
    if tabs.quick_open.is_open {
        render_quick_open_bar(frame, fonts, tabs, layout, screen_w, screen_h, qo_y);
    }
}
