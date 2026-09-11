use super::canvas::{
    draw_close_icon_clipped, draw_solid_rect, draw_solid_rect_clipped, draw_string,
    draw_string_clipped,
};
use super::font::FontManager;
use crate::config::*;
use crate::terminal::Terminal;
use crate::ui::layout::{calc_thumb, ViewportLayout};

pub fn render_terminal(
    frame: &mut [u32],
    fonts: &mut FontManager,
    terminal: &Terminal,
    layout: &ViewportLayout,
    screen_w: usize,
    screen_h: usize,
) {
    if !terminal.is_open {
        return;
    }
    let cw = fonts.char_width;
    let lh = fonts.line_height;
    let term_y = screen_h.saturating_sub(terminal.height);
    let term_w = screen_w.saturating_sub(layout.content_left);
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        layout.content_left,
        term_y,
        term_w,
        1,
        COLOR_SIDEBAR_BORDER,
    );
    let tabbar_y = term_y + 1;
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        layout.content_left,
        tabbar_y,
        term_w,
        TERMINAL_TAB_BAR_HEIGHT,
        COLOR_TABBAR_BG,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        layout.content_left,
        tabbar_y + TERMINAL_TAB_BAR_HEIGHT - 1,
        term_w,
        1,
        COLOR_TAB_BORDER,
    );
    let tab_text_offset_y = tabbar_y + (TERMINAL_TAB_BAR_HEIGHT.saturating_sub(lh)) / 2;
    let new_btn_label = "NEW";
    let new_btn_w = new_btn_label.len() * cw + 20;
    let new_text_color = if terminal.hovered_new {
        COLOR_TAB_TEXT_ACTIVE
    } else {
        COLOR_TAB_TEXT_INACTIVE
    };
    draw_string(
        fonts,
        frame,
        new_btn_label,
        layout.content_left as i32 + 10,
        tab_text_offset_y as i32,
        screen_w,
        screen_h,
        new_text_color,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        layout.content_left + new_btn_w - 1,
        tabbar_y,
        1,
        TERMINAL_TAB_BAR_HEIGHT - 1,
        COLOR_TAB_BORDER,
    );
    let strip_min_x = layout.content_left + new_btn_w;
    let strip_max_x = screen_w;
    let mut cur_x = strip_min_x as i32 - terminal.tab_scroll_x as i32;
    for (idx, tab) in terminal.tabs.iter().enumerate() {
        let is_active = idx == terminal.active_idx;
        let is_tab_hovered = terminal.hovered_tab == Some(idx);
        let is_close_hovered = terminal.hovered_close_tab == Some(idx);
        let tw = tab.width(cw);
        let tab_x0 = cur_x;
        let tab_x1 = cur_x + tw as i32;
        cur_x += tw as i32;
        if tab_x1 <= strip_min_x as i32 || tab_x0 >= strip_max_x as i32 {
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
            tabbar_y as i32,
            tw,
            TERMINAL_TAB_BAR_HEIGHT - 1,
            strip_min_x,
            strip_max_x,
            bg,
        );
        draw_solid_rect_clipped(
            frame,
            screen_w,
            screen_h,
            tab_x1 - 1,
            tabbar_y as i32,
            1,
            TERMINAL_TAB_BAR_HEIGHT - 1,
            strip_min_x,
            strip_max_x,
            COLOR_TAB_BORDER,
        );
        let text_color = if is_active {
            COLOR_TAB_TEXT_ACTIVE
        } else {
            COLOR_TAB_TEXT_INACTIVE
        };
        let text_clip_max = strip_max_x.min((tab_x1 - 26).max(0) as usize);
        draw_string_clipped(
            fonts,
            frame,
            &tab.name,
            tab_x0 + 14,
            tab_text_offset_y as i32,
            strip_min_x,
            text_clip_max,
            screen_w,
            screen_h,
            text_color,
        );
        let close_color = if is_close_hovered {
            COLOR_TAB_CLOSE_HOVER
        } else {
            text_color
        };
        let icon_size = 8;
        let close_x = tab_x1 - 22;
        let close_y = tabbar_y + (TERMINAL_TAB_BAR_HEIGHT.saturating_sub(icon_size)) / 2;
        draw_close_icon_clipped(
            frame,
            screen_w,
            screen_h,
            close_x,
            close_y as i32,
            icon_size,
            strip_min_x,
            strip_max_x,
            close_color,
        );
    }
    let shell_y = tabbar_y + TERMINAL_TAB_BAR_HEIGHT;
    let shell_h = screen_h.saturating_sub(shell_y);
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        layout.content_left,
        shell_y,
        term_w,
        shell_h,
        0xFF141414,
    );
    if let Some(active_tab) = terminal.active_tab() {
        let vbar_x = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
        let track_h = shell_h;
        let vis_lines = if lh > 0 {
            track_h.saturating_sub(4) / lh
        } else {
            0
        };
        let text_left = layout.content_left + 14;
        let text_right = vbar_x;
        let total_term_lines = active_tab.total_lines();
        let start_line = active_tab.scroll_line;
        for row in 0..vis_lines {
            let line_idx = start_line + row;
            let py = shell_y + 4 + row * lh;
            if py + lh > screen_h {
                break;
            }
            if let Some(((s_line, s_col), (e_line, e_col))) = active_tab.selection_range() {
                if line_idx >= s_line && line_idx <= e_line {
                    let text_len = active_tab
                        .get_row(line_idx)
                        .map(|r| r.cells.len())
                        .unwrap_or(0);
                    let line_s_col = if line_idx == s_line { s_col } else { 0 };
                    let line_e_col = if line_idx == e_line { e_col } else { text_len };
                    let c0 = line_s_col.min(line_e_col);
                    let c1 = line_s_col.max(line_e_col);
                    if c0 < c1 {
                        let sel_x0 = text_left + c0 * cw;
                        let sel_x1 = (text_left + c1 * cw).min(text_right);
                        if sel_x0 < sel_x1 {
                            draw_solid_rect(
                                frame,
                                screen_w,
                                screen_h,
                                sel_x0,
                                py,
                                sel_x1 - sel_x0,
                                lh,
                                COLOR_SELECTION,
                            );
                        }
                    }
                }
            }
            if let Some(t_row) = active_tab.get_row(line_idx) {
                for (col_idx, cell) in t_row.cells.iter().enumerate() {
                    let cell_px = text_left as i32 + (col_idx * cw) as i32;
                    if cell_px >= text_left as i32 && (cell_px as usize + cw) <= text_right {
                        if cell.ch != ' ' && cell.ch != '\0' {
                            fonts.draw_char(
                                frame, cell.ch, cell_px, py as i32, screen_w, screen_h, cell.fg,
                            );
                        }
                    }
                }
            }
        }
        if active_tab.screen.cursor_visible {
            let active_cursor_row =
                if active_tab.screen.is_alt || active_tab.screen.scrollback.is_empty() {
                    active_tab.screen.cursor_row
                } else {
                    active_tab.screen.scrollback.len() + active_tab.screen.cursor_row
                };
            if active_cursor_row >= active_tab.scroll_line
                && active_cursor_row < active_tab.scroll_line + vis_lines
            {
                let row_on_screen = active_cursor_row - active_tab.scroll_line;
                let py = shell_y + 4 + row_on_screen * lh;
                let cur_px = text_left as i32 + (active_tab.screen.cursor_col * cw) as i32;
                if cur_px >= text_left as i32 && (cur_px as usize + 2) <= text_right {
                    let cursor_color = if terminal.focused {
                        COLOR_CURSOR
                    } else {
                        COLOR_LINE_NUMBER_MUTED
                    };
                    draw_solid_rect(
                        frame,
                        screen_w,
                        screen_h,
                        cur_px as usize,
                        py,
                        2,
                        lh,
                        cursor_color,
                    );
                }
            }
        }
        if let Some((ty, th)) =
            calc_thumb(total_term_lines, vis_lines, active_tab.scroll_line, track_h)
        {
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                vbar_x,
                shell_y,
                SCROLLBAR_THICKNESS,
                track_h,
                COLOR_SCROLLBAR_TRACK,
            );
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                vbar_x,
                shell_y + ty,
                SCROLLBAR_THICKNESS,
                th,
                COLOR_SCROLLBAR_THUMB,
            );
        }
    }
}
