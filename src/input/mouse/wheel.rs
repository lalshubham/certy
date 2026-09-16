use super::hover::compute_bottom_bars_y;
use super::hover::update_find_hover;
use super::hover::update_quick_open_hover;
use super::InputHandler;
use crate::config::*;
use crate::editor::find::FindField;
use crate::editor::TabManager;
use crate::sidebar::Sidebar;
use crate::terminal::Terminal;
use crate::ui::layout::ViewportLayout;
use winit::event::MouseScrollDelta;

impl InputHandler {
    pub fn handle_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        tabs: &mut TabManager,
        sidebar: &mut Sidebar,
        terminal: &mut Terminal,
        layout: &ViewportLayout,
        char_w: usize,
        line_h: usize,
        screen_w: usize,
        screen_h: usize,
    ) -> bool {
        let (lines, cols) = match delta {
            MouseScrollDelta::LineDelta(h, v) => (
                (v * WHEEL_SCROLL_SPEED) as i32,
                (h * WHEEL_SCROLL_SPEED) as i32,
            ),
            MouseScrollDelta::PixelDelta(pos) => {
                self.scroll_accum_y += pos.y * TOUCHPAD_SCROLL_SPEED;
                self.scroll_accum_x += pos.x * TOUCHPAD_SCROLL_SPEED;
                let l = (self.scroll_accum_y / line_h as f64) as i32;
                let c = (self.scroll_accum_x / char_w as f64) as i32;
                if l != 0 {
                    self.scroll_accum_y -= l as f64 * line_h as f64;
                }
                if c != 0 {
                    self.scroll_accum_x -= c as f64 * char_w as f64;
                }
                (l, c)
            }
        };
        let mx = self.mouse_x.max(0.0) as usize;
        let my = self.mouse_y.max(0.0) as usize;
        if sidebar.visible && mx < sidebar.width {
            let total_h = sidebar.total_content_height();
            if total_h > screen_h {
                let max_scroll = total_h.saturating_sub(screen_h);
                let scroll_px = lines * (SIDEBAR_ROW_HEIGHT as i32 * 2);
                let next_scroll =
                    (sidebar.scroll_y as i32 - scroll_px).clamp(0, max_scroll as i32) as usize;
                if sidebar.scroll_y != next_scroll {
                    sidebar.scroll_y = next_scroll;
                    return true;
                }
            }
            return false;
        }
        if terminal.is_open && mx >= layout.content_left {
            let term_y = screen_h.saturating_sub(terminal.height);
            let tabbar_y = term_y + 1;
            let tabbar_h = TERMINAL_TAB_BAR_HEIGHT;
            if my >= tabbar_y && my < tabbar_y + tabbar_h {
                let new_btn_w = "NEW".len() * char_w + 20;
                let strip_min_x = layout.content_left + new_btn_w;
                let strip_max_x = screen_w;
                let available_w = strip_max_x.saturating_sub(strip_min_x);
                let scroll_delta = if cols != 0 { -cols } else { -lines };
                let scroll_amount = scroll_delta * (char_w as i32 * 3);
                let total_w = terminal.total_tabs_width(char_w);
                let max_scroll = total_w.saturating_sub(available_w);
                let next_scroll = (terminal.tab_scroll_x as i32 + scroll_amount)
                    .clamp(0, max_scroll as i32) as usize;
                if terminal.tab_scroll_x != next_scroll {
                    terminal.tab_scroll_x = next_scroll;
                    return true;
                }
                return false;
            }
            let shell_y = tabbar_y + TERMINAL_TAB_BAR_HEIGHT;
            if my >= shell_y {
                let vis_lines = terminal.vis_rows(line_h);
                if let Some(tab) = terminal.active_tab_mut() {
                    let total_l = tab.total_lines();
                    let max_l = total_l.saturating_sub(vis_lines);
                    let next_l = (tab.scroll_line as i32 - lines).clamp(0, max_l as i32) as usize;
                    if tab.scroll_line != next_l {
                        tab.scroll_line = next_l;
                        return true;
                    }
                }
                return false;
            }
        }
        if my < TAB_BAR_HEIGHT {
            let available_w = screen_w.saturating_sub(layout.content_left);
            let scroll_delta = if cols != 0 { -cols } else { -lines };
            let scroll_amount = scroll_delta * (char_w as i32 * 3);
            let total_w = tabs.total_tabs_width(char_w);
            let max_scroll = total_w.saturating_sub(available_w);
            let next_scroll =
                (tabs.scroll_x as i32 + scroll_amount).clamp(0, max_scroll as i32) as usize;
            if tabs.scroll_x != next_scroll {
                tabs.scroll_x = next_scroll;
                super::hover::update_tab_hover(tabs, layout, char_w, mx, my);
                return true;
            }
            return false;
        }
        let (find_y, qo_y) = compute_bottom_bars_y(tabs, layout);
        if tabs.quick_open.is_open && mx >= layout.content_left {
            let max_visible_items = 8;
            let qo_item_count = tabs.quick_open.matches.len().min(max_visible_items);
            let qo_popup_h = qo_item_count * 28;
            let qo_popup_y = qo_y.saturating_sub(qo_popup_h);
            if qo_item_count > 0 && my >= qo_popup_y && my < qo_y {
                if lines > 0 {
                    tabs.quick_open.prev_match();
                    update_quick_open_hover(tabs, layout, screen_w, char_w, mx, my, qo_y);
                    return true;
                } else if lines < 0 {
                    tabs.quick_open.next_match();
                    update_quick_open_hover(tabs, layout, screen_w, char_w, mx, my, qo_y);
                    return true;
                }
            }
        }
        if tabs.is_find_visible() {
            let bar_h = if tabs.find.is_replace { 66 } else { 36 };
            let bar_y = find_y;
            if my >= bar_y && my < bar_y + bar_h && mx >= layout.content_left {
                let cw = char_w.max(1);
                let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
                let toggle_w = toggle_label.len() * cw + 6;
                let toggle_x = layout.content_left + 6;
                let scrollable_min_x = toggle_x + toggle_w + 6;
                let cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
                let mx_i = mx as i32;
                let scroll_delta = if cols != 0 { -cols } else { -lines };
                let padding = 4;
                let max_vis_chars = if cw > 0 {
                    240usize.saturating_sub(padding * 2) / cw
                } else {
                    10
                };
                let query_hovered =
                    my >= bar_y + 6 && my < bar_y + 30 && mx_i >= cur_x && mx_i < cur_x + 240;
                if tabs.find.focused && tabs.find.active_field == FindField::Find && query_hovered {
                    let q_len = tabs.find.query.chars().count();
                    let max_scroll = q_len.saturating_sub(max_vis_chars);
                    let next_scroll = (tabs.find.query_scroll as i32 + scroll_delta)
                        .clamp(0, max_scroll as i32) as usize;
                    if tabs.find.query_scroll != next_scroll {
                        tabs.find.query_scroll = next_scroll;
                        return true;
                    }
                    return false;
                }
                let rep_hovered = tabs.find.is_replace
                    && my >= bar_y + 36
                    && my < bar_y + 60
                    && mx_i >= cur_x
                    && mx_i < cur_x + 240;
                if tabs.find.focused && tabs.find.active_field == FindField::Replace && rep_hovered
                {
                    let r_len = tabs.find.replace_text.chars().count();
                    let max_scroll = r_len.saturating_sub(max_vis_chars);
                    let next_scroll = (tabs.find.replace_scroll as i32 + scroll_delta)
                        .clamp(0, max_scroll as i32) as usize;
                    if tabs.find.replace_scroll != next_scroll {
                        tabs.find.replace_scroll = next_scroll;
                        return true;
                    }
                    return false;
                }
                let close_w = "Close".len() * char_w + 16;
                let fixed_w = 6 + toggle_w + 6 + close_w + 6;
                let available_w = screen_w.saturating_sub(layout.content_left + fixed_w);
                let scroll_amount = scroll_delta * (char_w as i32 * 3);
                let total_w = tabs.find.total_content_width(char_w);
                let max_scroll = total_w.saturating_sub(available_w);
                let next_scroll = (tabs.find.scroll_x as i32 + scroll_amount)
                    .clamp(0, max_scroll as i32) as usize;
                if tabs.find.scroll_x != next_scroll {
                    tabs.find.scroll_x = next_scroll;
                    update_find_hover(tabs, layout, screen_w, char_w, mx, my, find_y);
                    return true;
                }
                return false;
            }
        }
        if let Some(active_tab) = tabs.active_tab_mut() {
            let total = if active_tab.is_diff {
                active_tab.diff.as_ref().map(|d| d.lines.len()).unwrap_or(0)
            } else {
                active_tab.buffer.text().len_lines()
            };
            let active_buf = &mut active_tab.buffer;
            let max_l = total.saturating_sub(1);
            let max_c = active_buf.max_line_len.saturating_sub(layout.visible_cols);
            let next_l = (active_buf.scroll_line as i32 - lines).clamp(0, max_l as i32) as usize;
            let next_c = (active_buf.scroll_col as i32 - cols).clamp(0, max_c as i32) as usize;
            if active_buf.scroll_line != next_l || active_buf.scroll_col != next_c {
                active_buf.scroll_line = next_l;
                active_buf.scroll_col = next_c;
                return true;
            }
        }
        false
    }
}
