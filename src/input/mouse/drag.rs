use super::{DragState, InputHandler};
use crate::config::*;
use crate::editor::find::FindField;
use crate::editor::TabManager;
use crate::sidebar::Sidebar;
use crate::terminal::Terminal;
use crate::ui::layout::{calc_thumb, ViewportLayout};

pub fn process_drag(
    input: &mut InputHandler,
    tabs: &mut TabManager,
    sidebar: &mut Sidebar,
    terminal: &mut Terminal,
    layout: &ViewportLayout,
    char_w: usize,
    line_h: usize,
    screen_w: usize,
    screen_h: usize,
) -> bool {
    let mut changed = false;
    let mx = input.mouse_x.max(0.0) as usize;
    let my = input.mouse_y.max(0.0) as usize;

    match input.drag {
        DragState::SidebarResize { start_x, start_w } => {
            let delta = input.mouse_x - start_x;
            let min_editor_w = 120;
            let max_sidebar_w = screen_w.saturating_sub(min_editor_w);
            let min_sidebar_w = SIDEBAR_MIN_WIDTH.min(max_sidebar_w);
            let target_max_w =
                ((screen_w as f64 * 0.7) as usize).clamp(min_sidebar_w, max_sidebar_w);
            let new_w = if target_max_w <= min_sidebar_w {
                min_sidebar_w
            } else {
                ((start_w as f64 + delta) as usize).clamp(min_sidebar_w, target_max_w)
            };
            if sidebar.width != new_w {
                sidebar.width = new_w;
                changed = true;
            }
        }
        DragState::TerminalResize { start_y, start_h } => {
            let delta = start_y - input.mouse_y;
            let min_editor_h = TAB_BAR_HEIGHT + 40;
            let max_term_h = screen_h.saturating_sub(min_editor_h);
            let min_term_h = (TERMINAL_TAB_BAR_HEIGHT + 40).min(max_term_h);
            let new_h = if max_term_h <= min_term_h {
                min_term_h
            } else {
                ((start_h as f64 + delta) as usize).clamp(min_term_h, max_term_h)
            };
            if terminal.height != new_h {
                terminal.height = new_h;
                let vis_rows = terminal.vis_rows(line_h);
                let text_left = layout.content_left + 14;
                let text_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
                let vis_cols = if char_w > 0 {
                    text_right.saturating_sub(text_left) / char_w
                } else {
                    80
                };
                terminal.resize_active_pty(vis_rows, vis_cols);
                changed = true;
            }
        }
        DragState::TerminalSelecting => {
            let text_left = layout.content_left + 14;
            let term_y = screen_h.saturating_sub(terminal.height);
            let shell_y = term_y + 1 + TERMINAL_TAB_BAR_HEIGHT;
            let row = my.saturating_sub(shell_y + 4) / line_h.max(1);
            if let Some(tab) = terminal.active_tab_mut() {
                let total_l = tab.total_lines();
                let line_idx = (tab.scroll_line + row).min(total_l.saturating_sub(1));
                let col = if (input.mouse_x as i32) <= (text_left as i32) {
                    0
                } else {
                    let rel_x = mx.saturating_sub(text_left);
                    rel_x / char_w.max(1)
                };
                let line_len = tab.get_row(line_idx).map(|r| r.cells.len()).unwrap_or(0);
                let clamped_col = col.min(line_len);
                tab.selection_end = Some((line_idx, clamped_col));
                changed = true;
            }
        }
        DragState::TerminalVertical {
            start_y,
            start_line,
        } => {
            let term_y = screen_h.saturating_sub(terminal.height);
            let shell_y = term_y + 1 + TERMINAL_TAB_BAR_HEIGHT;
            let shell_h = screen_h.saturating_sub(shell_y);
            let track_h = shell_h;
            let vis_lines = terminal.vis_rows(line_h);
            if let Some(tab) = terminal.active_tab_mut() {
                let total = tab.total_lines();
                if let Some((_, th)) = calc_thumb(total, vis_lines, tab.scroll_line, track_h) {
                    let travel = track_h.saturating_sub(th) as f64;
                    if travel > 0.0 {
                        let max_s = (total - vis_lines) as f64;
                        let target = (start_line as f64
                            + ((input.mouse_y - start_y) / travel) * max_s)
                            .clamp(0.0, max_s) as usize;
                        if tab.scroll_line != target {
                            tab.scroll_line = target;
                            changed = true;
                        }
                    }
                }
            }
        }
        DragState::SidebarScroll {
            start_y,
            start_scroll,
        } => {
            let total_h = sidebar.total_content_height();
            if let Some((_, th)) = calc_thumb(total_h, screen_h, sidebar.scroll_y, screen_h) {
                let travel = screen_h.saturating_sub(th) as f64;
                if travel > 0.0 {
                    let max_s = (total_h.saturating_sub(screen_h)) as f64;
                    let target = (start_scroll as f64
                        + ((input.mouse_y - start_y) / travel) * max_s)
                        .clamp(0.0, max_s) as usize;
                    if sidebar.scroll_y != target {
                        sidebar.scroll_y = target;
                        changed = true;
                    }
                }
            }
        }
        DragState::SelectingText => {
            if let Some(active_tab) = tabs.active_tab_mut() {
                let active_buf = &mut active_tab.buffer;
                if line_h > 0 && char_w > 0 {
                    let top_y = (TAB_BAR_HEIGHT + TOP_PADDING) as f64;
                    let bottom_y = layout.content_bottom as f64;
                    let left_x = layout.code_x as f64;
                    let right_x = layout.content_right as f64;

                    if input.mouse_y < top_y {
                        let lines_above = ((top_y - input.mouse_y) / line_h as f64).ceil() as usize;
                        active_buf.scroll_line =
                            active_buf.scroll_line.saturating_sub(lines_above.max(1));
                    } else if input.mouse_y > bottom_y {
                        let lines_below =
                            ((input.mouse_y - bottom_y) / line_h as f64).ceil() as usize;
                        let max_scroll = active_buf.text().len_lines().saturating_sub(1);
                        active_buf.scroll_line =
                            (active_buf.scroll_line + lines_below.max(1)).min(max_scroll);
                    }

                    if input.mouse_x < left_x {
                        let cols_left = ((left_x - input.mouse_x) / char_w as f64).ceil() as usize;
                        active_buf.scroll_col =
                            active_buf.scroll_col.saturating_sub(cols_left.max(1));
                    } else if input.mouse_x > right_x {
                        let cols_right =
                            ((input.mouse_x - right_x) / char_w as f64).ceil() as usize;
                        let max_col_scroll =
                            active_buf.max_line_len.saturating_sub(layout.visible_cols);
                        active_buf.scroll_col =
                            (active_buf.scroll_col + cols_right.max(1)).min(max_col_scroll);
                    }

                    let target_line = if input.mouse_y < top_y {
                        active_buf.scroll_line
                    } else {
                        let row = ((input.mouse_y - top_y) as usize) / line_h;
                        active_buf.scroll_line + row
                    };
                    let target_vcol = if input.mouse_x < left_x {
                        active_buf.scroll_col
                    } else {
                        let col = ((input.mouse_x - left_x) as usize) / char_w;
                        active_buf.scroll_col + col
                    };

                    active_buf.set_cursor_at_visual(target_line, target_vcol);
                    active_buf.fit_view(layout.visible_lines, layout.visible_cols);
                    changed = true;
                }
            }
        }
        DragState::Vertical {
            start_y,
            start_line,
        } => {
            if let Some(active_tab) = tabs.active_tab_mut() {
                let total = if active_tab.is_diff {
                    active_tab.diff.as_ref().map(|d| d.lines.len()).unwrap_or(0)
                } else {
                    active_tab.buffer.text().len_lines()
                };
                let active_buf = &mut active_tab.buffer;
                let usable_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT);
                let virtual_total = total + layout.visible_lines.saturating_sub(1);
                if let Some((_, th)) = calc_thumb(
                    virtual_total,
                    layout.visible_lines,
                    active_buf.scroll_line,
                    usable_h,
                ) {
                    let travel = usable_h.saturating_sub(th) as f64;
                    if travel > 0.0 {
                        let max_s = virtual_total.saturating_sub(layout.visible_lines) as f64;
                        let target = (start_line as f64
                            + ((input.mouse_y - start_y) / travel) * max_s)
                            .clamp(0.0, max_s) as usize;
                        if active_buf.scroll_line != target {
                            active_buf.scroll_line = target;
                            changed = true;
                        }
                    }
                }
            }
        }
        DragState::Horizontal { start_x, start_col } => {
            if let Some(active_tab) = tabs.active_tab_mut() {
                let active_buf = &mut active_tab.buffer;
                let max_c = active_buf.max_line_len;
                let track_w = layout.content_right.saturating_sub(layout.bar_start_x);
                if let Some((_, tw)) =
                    calc_thumb(max_c, layout.visible_cols, active_buf.scroll_col, track_w)
                {
                    let travel = track_w.saturating_sub(tw) as f64;
                    if travel > 0.0 {
                        let max_s = (max_c - layout.visible_cols) as f64;
                        let target = (start_col as f64
                            + ((input.mouse_x - start_x) / travel) * max_s)
                            .clamp(0.0, max_s) as usize;
                        if active_buf.scroll_col != target {
                            active_buf.scroll_col = target;
                            changed = true;
                        }
                    }
                }
            }
        }
        DragState::QuickOpenSelecting => {
            let cw = char_w.max(1);
            let bar_x = layout.content_left;
            let bar_w = screen_w.saturating_sub(bar_x);
            let close_w = "Close".len() * cw + 16;
            let close_btn_x = (bar_x + bar_w).saturating_sub(close_w + 6);
            let input_x = bar_x + 6;
            let input_w = close_btn_x.saturating_sub(input_x + 6);
            let padding = 6;
            let input_left_x = (input_x + padding) as f64;
            let input_right_x = (input_x + input_w.saturating_sub(padding)) as f64;
            let max_vis_chars = if cw > 0 {
                input_w.saturating_sub(padding * 2) / cw
            } else {
                10
            };
            let q_len = tabs.quick_open.query.chars().count();

            if input.mouse_x < input_left_x {
                if tabs.quick_open.query_scroll > 0 {
                    tabs.quick_open.query_scroll = tabs.quick_open.query_scroll.saturating_sub(1);
                }
                tabs.quick_open.cursor = tabs.quick_open.query_scroll;
                changed = true;
            } else if input.mouse_x > input_right_x {
                let max_scroll = q_len.saturating_sub(max_vis_chars);
                if tabs.quick_open.query_scroll < max_scroll {
                    tabs.quick_open.query_scroll =
                        (tabs.quick_open.query_scroll + 1).min(max_scroll);
                }
                tabs.quick_open.cursor = (tabs.quick_open.query_scroll + max_vis_chars).min(q_len);
                changed = true;
            } else {
                let char_offset = ((input.mouse_x - input_left_x) as usize) / cw;
                let scroll_offset = tabs
                    .quick_open
                    .query_scroll
                    .min(q_len.saturating_sub(max_vis_chars));
                let new_cur = (scroll_offset + char_offset).min(q_len);
                if tabs.quick_open.cursor != new_cur {
                    tabs.quick_open.cursor = new_cur;
                    tabs.quick_open.ensure_query_visible(max_vis_chars);
                    changed = true;
                }
            }
        }
        DragState::FindSelecting => {
            let cw = char_w.max(1);
            let padding = 4;
            let find_input_w: usize = 240;
            let max_vis_chars = if cw > 0 {
                find_input_w.saturating_sub(padding * 2) / cw
            } else {
                10
            };
            let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
            let toggle_w = (toggle_label.len() * cw + 6) as i32;
            let toggle_x = layout.content_left as i32 + 6;
            let scrollable_min_x = toggle_x as usize + toggle_w as usize + 6;
            let cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
            let input_left_x = (cur_x + padding as i32) as f64;
            let input_right_x = (cur_x + find_input_w as i32 - padding as i32) as f64;

            match tabs.find.active_field {
                FindField::Find => {
                    let q_len = tabs.find.query.chars().count();
                    if input.mouse_x < input_left_x {
                        if tabs.find.query_scroll > 0 {
                            tabs.find.query_scroll = tabs.find.query_scroll.saturating_sub(1);
                        }
                        tabs.find.query_cursor = tabs.find.query_scroll;
                        changed = true;
                    } else if input.mouse_x > input_right_x {
                        let max_scroll = q_len.saturating_sub(max_vis_chars);
                        if tabs.find.query_scroll < max_scroll {
                            tabs.find.query_scroll = (tabs.find.query_scroll + 1).min(max_scroll);
                        }
                        tabs.find.query_cursor =
                            (tabs.find.query_scroll + max_vis_chars).min(q_len);
                        changed = true;
                    } else {
                        let char_offset = ((input.mouse_x - input_left_x) as usize) / cw;
                        let scroll_offset = tabs
                            .find
                            .query_scroll
                            .min(q_len.saturating_sub(max_vis_chars));
                        let new_cur = (scroll_offset + char_offset).min(q_len);
                        if tabs.find.query_cursor != new_cur {
                            tabs.find.query_cursor = new_cur;
                            tabs.find.ensure_query_visible(max_vis_chars);
                            changed = true;
                        }
                    }
                }
                FindField::Replace => {
                    let r_len = tabs.find.replace_text.chars().count();
                    if input.mouse_x < input_left_x {
                        if tabs.find.replace_scroll > 0 {
                            tabs.find.replace_scroll = tabs.find.replace_scroll.saturating_sub(1);
                        }
                        tabs.find.replace_cursor = tabs.find.replace_scroll;
                        changed = true;
                    } else if input.mouse_x > input_right_x {
                        let max_scroll = r_len.saturating_sub(max_vis_chars);
                        if tabs.find.replace_scroll < max_scroll {
                            tabs.find.replace_scroll =
                                (tabs.find.replace_scroll + 1).min(max_scroll);
                        }
                        tabs.find.replace_cursor =
                            (tabs.find.replace_scroll + max_vis_chars).min(r_len);
                        changed = true;
                    } else {
                        let char_offset = ((input.mouse_x - input_left_x) as usize) / cw;
                        let scroll_offset = tabs
                            .find
                            .replace_scroll
                            .min(r_len.saturating_sub(max_vis_chars));
                        let new_cur = (scroll_offset + char_offset).min(r_len);
                        if tabs.find.replace_cursor != new_cur {
                            tabs.find.replace_cursor = new_cur;
                            tabs.find.ensure_replace_visible(max_vis_chars);
                            changed = true;
                        }
                    }
                }
            }
        }
        DragState::None => {}
    }
    changed
}
