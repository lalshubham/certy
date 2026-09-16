use super::hover::compute_bottom_bars_y;
use super::hover::update_terminal_tab_hover;
use super::{DragState, InputHandler};
use crate::config::*;
use crate::editor::find::FindField;
use crate::editor::TabManager;
use crate::input::actions::ActionEvent;
use crate::sidebar::{MenuItem, Sidebar};
use crate::terminal::Terminal;
use crate::ui::layout::{calc_thumb, ViewportLayout};
use crate::ui::overlays::compute_modal_layout;
use crate::ui::ContextMenu;
use winit::event::{ElementState, MouseButton};

impl InputHandler {
    pub fn handle_mouse_click(
        &mut self,
        state: ElementState,
        button: MouseButton,
        tabs: &mut TabManager,
        sidebar: &mut Sidebar,
        terminal: &mut Terminal,
        layout: &ViewportLayout,
        char_w: usize,
        line_h: usize,
        screen_w: usize,
        screen_h: usize,
    ) -> ActionEvent {
        if button == MouseButton::Right {
            if state == ElementState::Pressed {
                let item_count = 2;
                let row_h = 30;
                let menu_h = row_h * item_count;
                let menu_w = (15 * char_w + 24).max(150);
                let px = (self.mouse_x as usize).min(screen_w.saturating_sub(menu_w));
                let py = (self.mouse_y as usize).min(screen_h.saturating_sub(menu_h));
                self.context_menu = Some(ContextMenu {
                    x: px,
                    y: py,
                    width: menu_w,
                    height: menu_h,
                    hovered_idx: None,
                });
                return ActionEvent::Redraw;
            }
            return ActionEvent::None;
        }

        if button != MouseButton::Left {
            return ActionEvent::None;
        }

        if state == ElementState::Released {
            self.is_left_down = false;
            let was_dragging = self.drag != DragState::None;
            if let Some(active_tab) = tabs.active_tab_mut() {
                if self.drag == DragState::SelectingText
                    && active_tab.buffer.selection_anchor == Some(active_tab.buffer.cursor_char)
                {
                    active_tab.buffer.selection_anchor = None;
                }
            }
            if let DragState::TerminalSelecting = self.drag {
                if let Some(tab) = terminal.active_tab_mut() {
                    if tab.selection_anchor == tab.selection_end {
                        tab.selection_anchor = None;
                        tab.selection_end = None;
                    }
                }
            }
            if self.drag == DragState::FindSelecting {
                match tabs.find.active_field {
                    FindField::Find => {
                        if tabs.find.query_selection_anchor == Some(tabs.find.query_cursor) {
                            tabs.find.query_selection_anchor = None;
                        }
                    }
                    FindField::Replace => {
                        if tabs.find.replace_selection_anchor == Some(tabs.find.replace_cursor) {
                            tabs.find.replace_selection_anchor = None;
                        }
                    }
                }
            }
            if self.drag == DragState::QuickOpenSelecting {
                if tabs.quick_open.selection_anchor == Some(tabs.quick_open.cursor) {
                    tabs.quick_open.selection_anchor = None;
                }
            }
            self.drag = DragState::None;
            return if was_dragging {
                ActionEvent::Redraw
            } else {
                ActionEvent::None
            };
        }

        self.is_left_down = true;
        let mx = self.mouse_x.max(0.0) as usize;
        let my = self.mouse_y.max(0.0) as usize;

        let (find_y, qo_y) = compute_bottom_bars_y(tabs, layout);
        let find_h = if tabs.find.is_open {
            if tabs.find.is_replace {
                66
            } else {
                36
            }
        } else {
            0
        };
        let is_find_bar =
            tabs.find.is_open && my >= find_y && my < find_y + find_h && mx >= layout.content_left;
        let is_quick_open_bar =
            tabs.quick_open.is_open && my >= qo_y && my < qo_y + 36 && mx >= layout.content_left;
        let max_visible_items = 8;
        let qo_item_count = tabs.quick_open.matches.len().min(max_visible_items);
        let qo_popup_h = qo_item_count * 28;
        let qo_popup_y = qo_y.saturating_sub(qo_popup_h);
        let is_quick_open_popup = tabs.quick_open.is_open
            && qo_item_count > 0
            && my >= qo_popup_y
            && my < qo_y
            && mx >= layout.content_left;
        let is_bottom_bar = is_find_bar || is_quick_open_bar || is_quick_open_popup;

        if my < TAB_BAR_HEIGHT
            || mx < layout.content_left
            || (mx >= layout.content_right && !is_bottom_bar)
            || (my >= layout.content_bottom && !is_bottom_bar)
        {
            self.last_click_time = None;
            self.click_count = 0;
        }

        if let Some(menu) = self.context_menu.take() {
            tabs.focused = false;
            terminal.focused = false;
            if mx >= menu.x && mx < menu.x + menu.width && my >= menu.y && my < menu.y + menu.height
            {
                let row_h = menu.height / 2;
                let idx = (my - menu.y) / row_h.max(1);
                if idx == 0 {
                    return ActionEvent::ToggleSidebar;
                } else if idx == 1 && !tabs.tabs.is_empty() {
                    return ActionEvent::CloseAllFiles;
                }
            }
            return ActionEvent::Redraw;
        }

        if let Some(modal) = compute_modal_layout(tabs, screen_w, screen_h, char_w, line_h) {
            tabs.focused = false;
            terminal.focused = false;
            for btn in &modal.buttons {
                if mx >= btn.x && mx < btn.x + btn.w && my >= btn.y && my < btn.y + btn.h {
                    if tabs.closing_app {
                        return match btn.id {
                            0 => ActionEvent::SaveAllAndExit,
                            1 => ActionEvent::DiscardAllAndExit,
                            _ => ActionEvent::CancelClose,
                        };
                    } else if tabs.closing_files {
                        return match btn.id {
                            0 => ActionEvent::SaveAllFiles,
                            1 => ActionEvent::DiscardAllFiles,
                            _ => ActionEvent::CancelClose,
                        };
                    } else if let Some(close_idx) = tabs.pending_close {
                        return match btn.id {
                            0 => ActionEvent::SaveTab(close_idx),
                            1 => ActionEvent::DiscardTab(close_idx),
                            _ => ActionEvent::CancelClose,
                        };
                    }
                }
            }
            return ActionEvent::None;
        }

        if sidebar.visible && (mx as i32 - sidebar.width as i32).abs() <= 4 {
            self.drag = DragState::SidebarResize {
                start_x: self.mouse_x,
                start_w: sidebar.width,
            };
            tabs.focused = false;
            terminal.focused = false;
            return ActionEvent::Redraw;
        }

        if terminal.is_open && mx >= layout.content_left {
            let term_y = screen_h.saturating_sub(terminal.height);
            let tabbar_y = term_y + 1;
            let tabbar_h = TERMINAL_TAB_BAR_HEIGHT;
            if (my as i32 - term_y as i32).abs() <= 3 {
                self.drag = DragState::TerminalResize {
                    start_y: self.mouse_y,
                    start_h: terminal.height,
                };
                terminal.focused = true;
                tabs.focused = false;
                tabs.find.focused = false;
                return ActionEvent::Redraw;
            }
            if my >= tabbar_y && my < tabbar_y + tabbar_h {
                terminal.focused = true;
                tabs.focused = false;
                tabs.find.focused = false;
                let new_btn_w = "NEW".len() * char_w + 20;
                let strip_min_x = layout.content_left + new_btn_w;
                let strip_max_x = screen_w;
                let available_w = strip_max_x.saturating_sub(strip_min_x);
                if mx < strip_min_x {
                    let vis_rows = terminal.vis_rows(line_h);
                    let text_left = layout.content_left + 14;
                    let text_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
                    let vis_cols = if char_w > 0 {
                        text_right.saturating_sub(text_left) / char_w
                    } else {
                        80
                    };
                    terminal.add_terminal(char_w, available_w, vis_rows, vis_cols);
                    update_terminal_tab_hover(
                        terminal,
                        mx,
                        my,
                        layout.content_left,
                        screen_w,
                        screen_h,
                        char_w,
                    );
                    return ActionEvent::Redraw;
                } else {
                    let mx_i32 = mx as i32;
                    let mut cur_x = strip_min_x as i32 - terminal.tab_scroll_x as i32;
                    for idx in 0..terminal.tabs.len() {
                        let tw = terminal.tabs[idx].width(char_w) as i32;
                        let tab_x0 = cur_x;
                        let tab_x1 = cur_x + tw;
                        cur_x += tw;
                        if mx_i32 >= tab_x0 && mx_i32 < tab_x1 {
                            if mx_i32 >= tab_x1 - 27 && mx_i32 <= tab_x1 - 7 {
                                terminal.remove_terminal(idx, char_w, available_w);
                            } else {
                                terminal.active_idx = idx;
                                terminal.focused = true;
                                terminal.ensure_active_tab_visible(char_w, available_w);
                            }
                            update_terminal_tab_hover(
                                terminal,
                                mx,
                                my,
                                layout.content_left,
                                screen_w,
                                screen_h,
                                char_w,
                            );
                            return ActionEvent::Redraw;
                        }
                    }
                }
                return ActionEvent::Redraw;
            }

            let shell_y = tabbar_y + TERMINAL_TAB_BAR_HEIGHT;
            let shell_h = screen_h.saturating_sub(shell_y);
            let vbar_x = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
            let track_h = shell_h;
            if mx >= vbar_x && mx < screen_w && my >= shell_y && my < shell_y + track_h {
                terminal.focused = true;
                tabs.focused = false;
                tabs.find.focused = false;
                let vis_lines = terminal.vis_rows(line_h);
                if let Some(tab) = terminal.active_tab_mut() {
                    let total = tab.total_lines();
                    if let Some((ty, th)) = calc_thumb(total, vis_lines, tab.scroll_line, track_h) {
                        let thumb_y = shell_y + ty;
                        if my >= thumb_y && my < thumb_y + th {
                            self.drag = DragState::TerminalVertical {
                                start_y: self.mouse_y,
                                start_line: tab.scroll_line,
                            };
                        } else {
                            let ratio = ((my.saturating_sub(shell_y)) as f64 / track_h as f64)
                                .clamp(0.0, 1.0);
                            tab.scroll_line = (ratio * (total - vis_lines) as f64) as usize;
                            self.drag = DragState::TerminalVertical {
                                start_y: self.mouse_y,
                                start_line: tab.scroll_line,
                            };
                            return ActionEvent::Redraw;
                        }
                    }
                }
                return ActionEvent::Redraw;
            }

            if my >= shell_y && mx >= layout.content_left && mx < vbar_x {
                terminal.focused = true;
                tabs.focused = false;
                tabs.find.focused = false;
                let text_left = layout.content_left + 14;
                let row = my.saturating_sub(shell_y + 4) / line_h.max(1);
                if let Some(tab) = terminal.active_tab_mut() {
                    let line_idx = tab.scroll_line + row;
                    let col = if (self.mouse_x as i32) < (text_left as i32) {
                        0
                    } else {
                        (mx - text_left) / char_w.max(1)
                    };
                    let total_l = tab.total_lines();
                    if line_idx < total_l {
                        let line_len = tab.get_row(line_idx).map(|r| r.cells.len()).unwrap_or(0);
                        let clamped_col = col.min(line_len);
                        tab.selection_anchor = Some((line_idx, clamped_col));
                        tab.selection_end = Some((line_idx, clamped_col));
                    } else {
                        tab.selection_anchor = None;
                        tab.selection_end = None;
                    }
                }
                self.drag = DragState::TerminalSelecting;
                return ActionEvent::Redraw;
            }
        }

        let total_sidebar_h = sidebar.total_content_height();
        let has_sidebar_scroll = total_sidebar_h > screen_h;
        let bar_x = sidebar.width.saturating_sub(SCROLLBAR_THICKNESS);
        let can_save = tabs
            .active_tab()
            .map(|t| t.buffer.is_modified)
            .unwrap_or(false);
        let has_folder = sidebar.root_folder.is_some();

        if sidebar.visible && mx < sidebar.width {
            tabs.focused = false;
            terminal.focused = false;
            tabs.find.focused = false;
            if has_sidebar_scroll && mx >= bar_x {
                let max_scroll = total_sidebar_h.saturating_sub(screen_h);
                if let Some((thumb_y, thumb_h)) =
                    calc_thumb(total_sidebar_h, screen_h, sidebar.scroll_y, screen_h)
                {
                    if my >= thumb_y && my < thumb_y + thumb_h {
                        self.drag = DragState::SidebarScroll {
                            start_y: self.mouse_y,
                            start_scroll: sidebar.scroll_y,
                        };
                    } else {
                        let ratio = (my as f64 / screen_h as f64).clamp(0.0, 1.0);
                        sidebar.scroll_y = (ratio * max_scroll as f64) as usize;
                        self.drag = DragState::SidebarScroll {
                            start_y: self.mouse_y,
                            start_scroll: sidebar.scroll_y,
                        };
                        return ActionEvent::Redraw;
                    }
                }
                return ActionEvent::Redraw;
            }

            let content_y = my as i32 + sidebar.scroll_y as i32;
            if content_y >= 0 {
                let cy = content_y as usize;
                let menu_total_h = sidebar.menu_total_height();
                if cy < TAB_BAR_HEIGHT {
                    sidebar.toggle_menu();
                    return ActionEvent::Redraw;
                }
                if sidebar.menu_expanded && cy < menu_total_h {
                    let item_idx = (cy - TAB_BAR_HEIGHT) / SIDEBAR_ROW_HEIGHT;
                    let items = sidebar.menu_items();
                    if item_idx < items.len() {
                        let item = items[item_idx].0;
                        let is_disabled = (item == MenuItem::Save && !can_save)
                            || (item == MenuItem::CloseFolder && !has_folder);
                        if !is_disabled {
                            return ActionEvent::Menu(item);
                        }
                    }
                }
                if cy >= menu_total_h && cy < menu_total_h + TAB_BAR_HEIGHT {
                    return ActionEvent::ToggleTerminal;
                }
                if cy >= menu_total_h + TAB_BAR_HEIGHT {
                    let mut sec_y = menu_total_h + TAB_BAR_HEIGHT;
                    if has_folder {
                        let git_total_h = sidebar.git_total_height();
                        if cy >= sec_y && cy < sec_y + TAB_BAR_HEIGHT {
                            sidebar.toggle_git();
                            sidebar.clamp_scroll(screen_h);
                            return ActionEvent::Redraw;
                        } else if sidebar.git_expanded
                            && cy >= sec_y + TAB_BAR_HEIGHT
                            && cy < sec_y + git_total_h
                        {
                            let rel_row = cy - (sec_y + TAB_BAR_HEIGHT);
                            let row_idx = rel_row / SIDEBAR_ROW_HEIGHT;
                            if row_idx < sidebar.git_snapshot.files.len() {
                                return ActionEvent::OpenDiff(
                                    sidebar.git_snapshot.files[row_idx].path.clone(),
                                );
                            }
                            return ActionEvent::Redraw;
                        }
                        sec_y += git_total_h;
                    }

                    if has_folder && cy >= sec_y {
                        let rel_y = cy - sec_y;
                        if rel_y < TAB_BAR_HEIGHT {
                            sidebar.toggle_root();
                            sidebar.clamp_scroll(screen_h);
                            return ActionEvent::Redraw;
                        } else if sidebar.root_expanded {
                            let tree_y = rel_y - TAB_BAR_HEIGHT;
                            let node_idx = tree_y / SIDEBAR_ROW_HEIGHT;
                            if node_idx < sidebar.nodes.len() {
                                if sidebar.nodes[node_idx].is_dir {
                                    sidebar.toggle_dir(node_idx);
                                    sidebar.clamp_scroll(screen_h);
                                    return ActionEvent::Redraw;
                                } else {
                                    return ActionEvent::OpenFile(
                                        sidebar.nodes[node_idx].path.clone(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            return ActionEvent::Redraw;
        }

        if !tabs.tabs.is_empty() && my < TAB_BAR_HEIGHT {
            if mx >= layout.content_left {
                terminal.focused = false;
                let available_w = screen_w.saturating_sub(layout.content_left);
                if let Some(close_idx) = tabs.hovered_close {
                    tabs.closing_app = false;
                    tabs.request_close(close_idx);
                    tabs.clamp_scroll(char_w, available_w);
                    super::hover::update_tab_hover(tabs, layout, char_w, mx, my);
                    return ActionEvent::Redraw;
                }
                if let Some(tab_idx) = tabs.hovered_tab {
                    if tabs.active_idx != Some(tab_idx) {
                        tabs.active_idx = Some(tab_idx);
                        tabs.update_find_matches();
                    }
                    tabs.focused = true;
                    tabs.ensure_active_tab_visible(char_w, available_w);
                    super::hover::update_tab_hover(tabs, layout, char_w, mx, my);
                    return ActionEvent::Redraw;
                }
                tabs.focused = false;
                return ActionEvent::Redraw;
            }
            return ActionEvent::None;
        }

        if tabs.quick_open.is_open
            && qo_item_count > 0
            && my >= qo_popup_y
            && my < qo_y
            && mx >= layout.content_left
        {
            let bar_x = layout.content_left;
            let bar_w = screen_w.saturating_sub(bar_x);
            if mx >= bar_x && mx < bar_x + bar_w {
                let selected = tabs.quick_open.selected_match;
                let start_idx = if selected >= max_visible_items {
                    selected - max_visible_items + 1
                } else {
                    0
                };
                let row_idx = (my - qo_popup_y) / 28;
                let match_idx = start_idx + row_idx;
                if let Some(item) = tabs.quick_open.matches.get(match_idx) {
                    let path = item.path.clone();
                    tabs.quick_open.close();
                    return ActionEvent::OpenFile(path);
                }
            }
            return ActionEvent::Redraw;
        }

        if tabs.quick_open.is_open && my >= qo_y && my < qo_y + 36 && mx >= layout.content_left {
            terminal.focused = false;
            tabs.find.focused = false;
            tabs.quick_open.focused = true;
            tabs.focused = false;
            let bar_x = layout.content_left;
            let bar_w = screen_w.saturating_sub(bar_x);
            let cw = char_w.max(1);
            let input_h: usize = 24;
            let input_y = qo_y + 6;
            let close_w = "Close".len() * cw + 16;
            let close_btn_x = (bar_x + bar_w).saturating_sub(close_w + 6);
            if mx >= close_btn_x
                && mx < close_btn_x + close_w
                && my >= input_y
                && my < input_y + input_h
            {
                tabs.quick_open.close();
                tabs.focused = true;
                if tabs.find.is_open {
                    tabs.find.focused = true;
                    tabs.focused = false;
                }
                return ActionEvent::Redraw;
            }
            let input_x = bar_x + 6;
            let input_w = close_btn_x.saturating_sub(input_x + 6);
            if mx >= input_x && mx < input_x + input_w && my >= input_y && my < input_y + input_h {
                let padding = 6;
                let click_offset = (mx as i32 - input_x as i32 - padding as i32).max(0) as usize;
                let char_offset = click_offset / cw;
                let max_vis_chars = if cw > 0 {
                    input_w.saturating_sub(padding * 2) / cw
                } else {
                    10
                };
                let q_len = tabs.quick_open.query.chars().count();
                let scroll_offset = tabs
                    .quick_open
                    .query_scroll
                    .min(q_len.saturating_sub(max_vis_chars));
                tabs.quick_open.cursor = (scroll_offset + char_offset).min(q_len);
                tabs.quick_open.selection_anchor = Some(tabs.quick_open.cursor);
                let now = std::time::Instant::now();
                let is_multi = if let Some(last_time) = self.last_click_time {
                    let elapsed = now.duration_since(last_time);
                    let dx = self.mouse_x - self.last_click_pos.0;
                    let dy = self.mouse_y - self.last_click_pos.1;
                    elapsed.as_millis() <= 500 && (dx * dx + dy * dy) <= 36.0
                } else {
                    false
                };
                if is_multi {
                    self.click_count = (self.click_count % 3) + 1;
                } else {
                    self.click_count = 1;
                }
                self.last_click_time = Some(now);
                self.last_click_pos = (self.mouse_x, self.mouse_y);
                match self.click_count {
                    2 | 3 => {
                        tabs.quick_open.select_all();
                        self.drag = DragState::None;
                    }
                    _ => {
                        self.drag = DragState::QuickOpenSelecting;
                    }
                }
                return ActionEvent::Redraw;
            }
            return ActionEvent::Redraw;
        }

        if tabs.find.is_open {
            let bar_h = if tabs.find.is_replace { 66 } else { 36 };
            let bar_y = find_y;
            let bar_w = screen_w.saturating_sub(layout.content_left);
            if my >= bar_y
                && my < bar_y + bar_h
                && mx >= layout.content_left
                && mx < layout.content_left + bar_w
            {
                terminal.focused = false;
                tabs.quick_open.focused = false;
                tabs.find.focused = true;
                tabs.focused = false;
                let cw = char_w.max(1);
                let input_h: usize = 24;
                let input_y = bar_y + 6;
                let bottom_row_y = if tabs.find.is_replace {
                    bar_y + 36
                } else {
                    bar_y + 6
                };
                let close_w = "Close".len() * cw + 16;
                let close_btn_x = (layout.content_left + bar_w).saturating_sub(close_w + 6);
                let now = std::time::Instant::now();
                let is_multi = if let Some(last_time) = self.last_click_time {
                    let elapsed = now.duration_since(last_time);
                    let dx = self.mouse_x - self.last_click_pos.0;
                    let dy = self.mouse_y - self.last_click_pos.1;
                    elapsed.as_millis() <= 500 && (dx * dx + dy * dy) <= 36.0
                } else {
                    false
                };
                if is_multi {
                    self.click_count = (self.click_count % 3) + 1;
                } else {
                    self.click_count = 1;
                }
                self.last_click_time = Some(now);
                self.last_click_pos = (self.mouse_x, self.mouse_y);
                if mx >= close_btn_x
                    && mx < close_btn_x + close_w
                    && my >= bottom_row_y
                    && my < bottom_row_y + input_h
                {
                    tabs.find.close();
                    tabs.focused = true;
                    if tabs.quick_open.is_open {
                        tabs.quick_open.focused = true;
                        tabs.focused = false;
                    }
                    return ActionEvent::Redraw;
                }
                let strip_min_x = layout.content_left;
                let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
                let toggle_w = (toggle_label.len() * cw + 6) as i32;
                let toggle_x = strip_min_x as i32 + 6;
                let mx_i = mx as i32;
                if my >= bottom_row_y && my < bottom_row_y + input_h {
                    if mx_i >= toggle_x && mx_i < toggle_x + toggle_w {
                        tabs.find.is_replace = !tabs.find.is_replace;
                        return ActionEvent::Redraw;
                    }
                }
                let scrollable_min_x = toggle_x as usize + toggle_w as usize + 6;
                let strip_max_x = close_btn_x.saturating_sub(6);
                let mut cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
                let find_input_w: usize = 240;
                if my >= input_y && my < input_y + input_h {
                    if mx_i >= cur_x
                        && mx_i < cur_x + find_input_w as i32
                        && mx >= scrollable_min_x
                        && mx < strip_max_x
                    {
                        tabs.find.active_field = FindField::Find;
                        let padding = 4;
                        let click_offset = (mx_i - cur_x - padding as i32).max(0) as usize;
                        let char_offset = click_offset / cw;
                        let max_vis_chars = if cw > 0 {
                            find_input_w.saturating_sub(padding * 2) / cw
                        } else {
                            10
                        };
                        let q_len = tabs.find.query.chars().count();
                        let scroll_offset = tabs
                            .find
                            .query_scroll
                            .min(q_len.saturating_sub(max_vis_chars));
                        tabs.find.query_cursor = (scroll_offset + char_offset).min(q_len);
                        tabs.find.query_selection_anchor = Some(tabs.find.query_cursor);
                        match self.click_count {
                            2 => {
                                tabs.find.select_word();
                                self.drag = DragState::None;
                            }
                            3 => {
                                tabs.find.select_all();
                                self.drag = DragState::None;
                            }
                            _ => {
                                self.drag = DragState::FindSelecting;
                            }
                        }
                        return ActionEvent::Redraw;
                    }
                }
                cur_x += find_input_w as i32 + 6;
                let mc_w = ("Match Case".len() * cw + 16) as i32;
                let ww_w = ("Whole Word".len() * cw + 16) as i32;
                let re_w = ("Regex".len() * cw + 16) as i32;
                let prev_w = ("Previous".len() * cw + 16) as i32;
                let next_w = ("Next".len() * cw + 16) as i32;
                let has_matches = !tabs.find.matches.is_empty();
                if my >= input_y && my < input_y + input_h {
                    if mx_i >= cur_x
                        && mx_i < cur_x + mc_w
                        && mx >= scrollable_min_x
                        && mx < strip_max_x
                    {
                        tabs.find.match_case = !tabs.find.match_case;
                        if let Some(idx) = tabs.active_idx {
                            if let Some(active_tab) = tabs.tabs.get_mut(idx) {
                                tabs.find.update_matches(&active_tab.buffer);
                                tabs.find.sync_view(
                                    &mut active_tab.buffer,
                                    layout.visible_lines,
                                    layout.visible_cols,
                                );
                            }
                        }
                        return ActionEvent::Redraw;
                    }
                    cur_x += mc_w + 6;
                    if mx_i >= cur_x
                        && mx_i < cur_x + ww_w
                        && mx >= scrollable_min_x
                        && mx < strip_max_x
                    {
                        tabs.find.whole_word = !tabs.find.whole_word;
                        if let Some(idx) = tabs.active_idx {
                            if let Some(active_tab) = tabs.tabs.get_mut(idx) {
                                tabs.find.update_matches(&active_tab.buffer);
                                tabs.find.sync_view(
                                    &mut active_tab.buffer,
                                    layout.visible_lines,
                                    layout.visible_cols,
                                );
                            }
                        }
                        return ActionEvent::Redraw;
                    }
                    cur_x += ww_w + 6;
                    if mx_i >= cur_x
                        && mx_i < cur_x + re_w
                        && mx >= scrollable_min_x
                        && mx < strip_max_x
                    {
                        tabs.find.use_regex = !tabs.find.use_regex;
                        if let Some(idx) = tabs.active_idx {
                            if let Some(active_tab) = tabs.tabs.get_mut(idx) {
                                tabs.find.update_matches(&active_tab.buffer);
                                tabs.find.sync_view(
                                    &mut active_tab.buffer,
                                    layout.visible_lines,
                                    layout.visible_cols,
                                );
                            }
                        }
                        return ActionEvent::Redraw;
                    }
                    cur_x += re_w + 6;
                    if mx_i >= cur_x
                        && mx_i < cur_x + prev_w
                        && mx >= scrollable_min_x
                        && mx < strip_max_x
                    {
                        if has_matches {
                            if let Some(idx) = tabs.active_idx {
                                if let Some(active_tab) = tabs.tabs.get_mut(idx) {
                                    tabs.find.prev_match(
                                        &mut active_tab.buffer,
                                        layout.visible_lines,
                                        layout.visible_cols,
                                    );
                                }
                            }
                        }
                        return ActionEvent::Redraw;
                    }
                    cur_x += prev_w + 6;
                    if mx_i >= cur_x
                        && mx_i < cur_x + next_w
                        && mx >= scrollable_min_x
                        && mx < strip_max_x
                    {
                        if has_matches {
                            if let Some(idx) = tabs.active_idx {
                                if let Some(active_tab) = tabs.tabs.get_mut(idx) {
                                    tabs.find.next_match(
                                        &mut active_tab.buffer,
                                        layout.visible_lines,
                                        layout.visible_cols,
                                    );
                                }
                            }
                        }
                        return ActionEvent::Redraw;
                    }
                }
                if tabs.find.is_replace {
                    let rep_input_y = bottom_row_y;
                    let mut r_cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
                    let rep_input_w: usize = 240;
                    if my >= rep_input_y && my < rep_input_y + input_h {
                        if mx_i >= r_cur_x
                            && mx_i < r_cur_x + rep_input_w as i32
                            && mx >= scrollable_min_x
                            && mx < strip_max_x
                        {
                            tabs.find.active_field = FindField::Replace;
                            let padding = 4;
                            let click_offset = (mx_i - r_cur_x - padding as i32).max(0) as usize;
                            let char_offset = click_offset / cw;
                            let max_vis_chars = if cw > 0 {
                                rep_input_w.saturating_sub(padding * 2) / cw
                            } else {
                                10
                            };
                            let r_len = tabs.find.replace_text.chars().count();
                            let scroll_offset = tabs
                                .find
                                .replace_scroll
                                .min(r_len.saturating_sub(max_vis_chars));
                            tabs.find.replace_cursor = (scroll_offset + char_offset).min(r_len);
                            tabs.find.replace_selection_anchor = Some(tabs.find.replace_cursor);
                            match self.click_count {
                                2 => {
                                    tabs.find.select_word();
                                    self.drag = DragState::None;
                                }
                                3 => {
                                    tabs.find.select_all();
                                    self.drag = DragState::None;
                                }
                                _ => {
                                    self.drag = DragState::FindSelecting;
                                }
                            }
                            return ActionEvent::Redraw;
                        }
                    }
                    r_cur_x += rep_input_w as i32 + 6;
                    let rep_w = ("Replace".len() * cw + 16) as i32;
                    let all_w = ("Replace All".len() * cw + 16) as i32;
                    if my >= rep_input_y && my < rep_input_y + input_h {
                        if mx_i >= r_cur_x
                            && mx_i < r_cur_x + rep_w
                            && mx >= scrollable_min_x
                            && mx < strip_max_x
                        {
                            if has_matches {
                                if let Some(idx) = tabs.active_idx {
                                    if let Some(active_tab) = tabs.tabs.get_mut(idx) {
                                        tabs.find.replace_current(
                                            &mut active_tab.buffer,
                                            layout.visible_lines,
                                            layout.visible_cols,
                                        );
                                    }
                                }
                            }
                            return ActionEvent::Redraw;
                        }
                        r_cur_x += rep_w + 6;
                        if mx_i >= r_cur_x
                            && mx_i < r_cur_x + all_w
                            && mx >= scrollable_min_x
                            && mx < strip_max_x
                        {
                            if has_matches {
                                if let Some(idx) = tabs.active_idx {
                                    if let Some(active_tab) = tabs.tabs.get_mut(idx) {
                                        tabs.find.replace_all(
                                            &mut active_tab.buffer,
                                            layout.visible_lines,
                                            layout.visible_cols,
                                        );
                                    }
                                }
                            }
                            return ActionEvent::Redraw;
                        }
                    }
                }
                return ActionEvent::Redraw;
            }
        }

        terminal.focused = false;
        tabs.find.focused = false;
        if tabs.quick_open.is_open {
            tabs.quick_open.close();
        }

        let is_vert_scroll = mx >= layout.content_right
            && mx < screen_w
            && my >= TAB_BAR_HEIGHT
            && my < layout.content_bottom;
        let is_horiz_scroll = my >= layout.content_bottom
            && my < layout.content_bottom + SCROLLBAR_THICKNESS
            && mx >= layout.bar_start_x
            && mx < layout.content_right;
        let is_text_area = my >= TAB_BAR_HEIGHT
            && my < layout.content_bottom
            && mx >= layout.content_left
            && mx < layout.content_right;

        if is_vert_scroll || is_horiz_scroll || is_text_area {
            tabs.focused = true;
            terminal.focused = false;
            if let Some(active_tab) = tabs.active_tab_mut() {
                let total = if active_tab.is_diff {
                    active_tab.diff.as_ref().map(|d| d.lines.len()).unwrap_or(0)
                } else {
                    active_tab.buffer.text().len_lines()
                };
                let active_buf = &mut active_tab.buffer;
                let usable_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT);
                if is_vert_scroll {
                    let virtual_total = total + layout.visible_lines.saturating_sub(1);
                    if let Some((ty, th)) = calc_thumb(
                        virtual_total,
                        layout.visible_lines,
                        active_buf.scroll_line,
                        usable_h,
                    ) {
                        let thumb_y = TAB_BAR_HEIGHT + ty;
                        if my >= thumb_y && my < thumb_y + th {
                            self.drag = DragState::Vertical {
                                start_y: self.mouse_y,
                                start_line: active_buf.scroll_line,
                            };
                        } else {
                            let ratio =
                                ((my - TAB_BAR_HEIGHT) as f64 / usable_h as f64).clamp(0.0, 1.0);
                            let max_s = virtual_total.saturating_sub(layout.visible_lines);
                            active_buf.scroll_line = (ratio * max_s as f64) as usize;
                            self.drag = DragState::Vertical {
                                start_y: self.mouse_y,
                                start_line: active_buf.scroll_line,
                            };
                            return ActionEvent::Redraw;
                        }
                    }
                } else if is_horiz_scroll {
                    let track_w = layout.content_right.saturating_sub(layout.bar_start_x);
                    if let Some((tx_offset, tw)) = calc_thumb(
                        active_buf.max_line_len,
                        layout.visible_cols,
                        active_buf.scroll_col,
                        track_w,
                    ) {
                        let tx = layout.bar_start_x + tx_offset;
                        if mx >= tx && mx < tx + tw {
                            self.drag = DragState::Horizontal {
                                start_x: self.mouse_x,
                                start_col: active_buf.scroll_col,
                            };
                        } else {
                            let ratio =
                                ((mx - layout.bar_start_x) as f64 / track_w as f64).clamp(0.0, 1.0);
                            active_buf.scroll_col = (ratio
                                * (active_buf.max_line_len - layout.visible_cols) as f64)
                                as usize;
                            self.drag = DragState::Horizontal {
                                start_x: self.mouse_x,
                                start_col: active_buf.scroll_col,
                            };
                            return ActionEvent::Redraw;
                        }
                    }
                } else if is_text_area {
                    if line_h > 0 && char_w > 0 {
                        let row = my.saturating_sub(TAB_BAR_HEIGHT + TOP_PADDING) / line_h;
                        let target_line = active_buf.scroll_line + row;
                        let target_vcol = if mx >= layout.code_x {
                            active_buf.scroll_col + (mx - layout.code_x) / char_w
                        } else {
                            0
                        };
                        let now = std::time::Instant::now();
                        let is_multi = if let Some(last_time) = self.last_click_time {
                            let elapsed = now.duration_since(last_time);
                            let dx = self.mouse_x - self.last_click_pos.0;
                            let dy = self.mouse_y - self.last_click_pos.1;
                            elapsed.as_millis() <= 500 && (dx * dx + dy * dy) <= 36.0
                        } else {
                            false
                        };
                        if is_multi {
                            self.click_count = (self.click_count % 3) + 1;
                        } else {
                            self.click_count = 1;
                        }
                        self.last_click_time = Some(now);
                        self.last_click_pos = (self.mouse_x, self.mouse_y);
                        active_buf.set_cursor_at_visual(target_line, target_vcol);
                        match self.click_count {
                            2 => {
                                active_buf.select_word_at_cursor(target_vcol);
                                self.drag = DragState::None;
                            }
                            3 => {
                                active_buf.select_line_at_cursor();
                                self.drag = DragState::None;
                            }
                            _ => {
                                active_buf.selection_anchor = Some(active_buf.cursor_char);
                                self.drag = DragState::SelectingText;
                            }
                        }
                        active_buf.fit_view(layout.visible_lines, layout.visible_cols);
                        return ActionEvent::Redraw;
                    }
                }
            }
        } else {
            tabs.focused = false;
            terminal.focused = false;
            return ActionEvent::Redraw;
        }

        ActionEvent::None
    }
}
