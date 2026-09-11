use super::actions::ActionEvent;
use super::{DragState, InputHandler};
use crate::config::*;
use crate::editor::find::FindField;
use crate::editor::TabManager;
use crate::sidebar::{MenuItem, Sidebar};
use crate::terminal::Terminal;
use crate::ui::layout::{calc_thumb, ViewportLayout};
use crate::ui::overlays::compute_modal_layout;
use crate::ui::ContextMenu;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::window::CursorIcon;

fn update_terminal_tab_hover(
    terminal: &mut Terminal,
    mx: usize,
    my: usize,
    content_left: usize,
    screen_w: usize,
    screen_h: usize,
    char_w: usize,
) {
    terminal.hovered_new = false;
    terminal.hovered_tab = None;
    terminal.hovered_close_tab = None;

    if !terminal.is_open || mx < content_left || mx >= screen_w {
        return;
    }
    let term_y = screen_h.saturating_sub(terminal.height);
    let tabbar_y = term_y + 1;
    let tabbar_h = TERMINAL_TAB_BAR_HEIGHT;

    if my >= tabbar_y && my < tabbar_y + tabbar_h {
        let new_btn_w = "NEW".len() * char_w + 20;
        let strip_min_x = content_left + new_btn_w;
        let strip_max_x = screen_w;

        if mx < strip_min_x {
            terminal.hovered_new = true;
        } else {
            let mx_i32 = mx as i32;
            let mut cur_x = strip_min_x as i32 - terminal.tab_scroll_x as i32;

            for (idx, tab) in terminal.tabs.iter().enumerate() {
                let tw = tab.width(char_w) as i32;
                let tab_x0 = cur_x;
                let tab_x1 = cur_x + tw;

                cur_x += tw;

                if mx_i32 >= tab_x0 && mx_i32 < tab_x1 && mx >= strip_min_x && mx < strip_max_x {
                    terminal.hovered_tab = Some(idx);
                    if mx_i32 >= tab_x1 - 27 && mx_i32 <= tab_x1 - 7 {
                        terminal.hovered_close_tab = Some(idx);
                    }
                    break;
                }
            }
        }
    }
}

fn update_find_hover(
    tabs: &mut TabManager,
    layout: &ViewportLayout,
    screen_w: usize,
    char_w: usize,
    mx: usize,
    my: usize,
) {
    tabs.find.hovered_btn = None;
    if !tabs.find.is_open {
        return;
    }

    let bar_h = if tabs.find.is_replace { 66 } else { 36 };
    let bar_x = layout.content_left;
    let bar_w = screen_w.saturating_sub(bar_x);
    let bar_y = layout.content_bottom + SCROLLBAR_THICKNESS;

    if my < bar_y || my >= bar_y + bar_h || mx < bar_x || mx >= bar_x + bar_w {
        return;
    }

    let cw = char_w.max(1);
    let input_h: usize = 24;
    let input_y = bar_y + 6;
    let bottom_row_y = if tabs.find.is_replace {
        bar_y + 36
    } else {
        bar_y + 6
    };

    let close_w = "Close".len() * cw + 16;
    let close_btn_x = (bar_x + bar_w).saturating_sub(close_w + 6);

    if mx >= close_btn_x
        && mx < close_btn_x + close_w
        && my >= bottom_row_y
        && my < bottom_row_y + input_h
    {
        tabs.find.hovered_btn = Some(16);
        return;
    }

    let strip_min_x = bar_x;
    let strip_max_x = close_btn_x.saturating_sub(6);

    let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
    let toggle_w = (toggle_label.len() * cw + 6) as i32;
    let toggle_x = strip_min_x as i32 + 6;

    if my >= bottom_row_y && my < bottom_row_y + input_h {
        let mx_i = mx as i32;
        if mx_i >= toggle_x && mx_i < toggle_x + toggle_w {
            tabs.find.hovered_btn = Some(10);
            return;
        }
    }

    let scrollable_min_x = toggle_x as usize + toggle_w as usize + 6;
    let mut cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
    let find_input_w: usize = 240;

    cur_x += find_input_w as i32 + 6;

    let mc_w = ("Match Case".len() * cw + 16) as i32;
    let ww_w = ("Whole Word".len() * cw + 16) as i32;
    let re_w = ("Regex".len() * cw + 16) as i32;
    let prev_w = ("Previous".len() * cw + 16) as i32;
    let next_w = ("Next".len() * cw + 16) as i32;
    let has_matches = !tabs.find.matches.is_empty();

    if my >= input_y && my < input_y + input_h {
        let mx_i = mx as i32;
        if mx_i >= cur_x && mx_i < cur_x + mc_w && mx >= scrollable_min_x && mx < strip_max_x {
            tabs.find.hovered_btn = Some(11);
            return;
        }
        cur_x += mc_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + ww_w && mx >= scrollable_min_x && mx < strip_max_x {
            tabs.find.hovered_btn = Some(12);
            return;
        }
        cur_x += ww_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + re_w && mx >= scrollable_min_x && mx < strip_max_x {
            tabs.find.hovered_btn = Some(13);
            return;
        }
        cur_x += re_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + prev_w && mx >= scrollable_min_x && mx < strip_max_x {
            if has_matches {
                tabs.find.hovered_btn = Some(14);
            }
            return;
        }
        cur_x += prev_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + next_w && mx >= scrollable_min_x && mx < strip_max_x {
            if has_matches {
                tabs.find.hovered_btn = Some(15);
            }
            return;
        }
    }

    if tabs.find.is_replace {
        let rep_input_y = bottom_row_y;
        if my >= rep_input_y && my < rep_input_y + input_h {
            let mut r_cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
            let rep_input_w: usize = 240;

            r_cur_x += rep_input_w as i32 + 6;
            let rep_w = ("Replace".len() * cw + 16) as i32;
            let all_w = ("Replace All".len() * cw + 16) as i32;

            let mx_i = mx as i32;
            if mx_i >= r_cur_x
                && mx_i < r_cur_x + rep_w
                && mx >= scrollable_min_x
                && mx < strip_max_x
            {
                if has_matches {
                    tabs.find.hovered_btn = Some(20);
                }
                return;
            }
            r_cur_x += rep_w + 6;
            if mx_i >= r_cur_x
                && mx_i < r_cur_x + all_w
                && mx >= scrollable_min_x
                && mx < strip_max_x
            {
                if has_matches {
                    tabs.find.hovered_btn = Some(21);
                }
                return;
            }
        }
    }
}

impl InputHandler {
    pub fn desired_cursor_icon(
        &mut self,
        layout: &ViewportLayout,
        tabs: &TabManager,
        sidebar: &Sidebar,
        terminal: &Terminal,
        screen_h: usize,
    ) -> CursorIcon {
        if !self.is_left_down && self.drag != DragState::None {
            self.drag = DragState::None;
        }

        if let Some(menu) = self.context_menu {
            return if menu.hovered_idx.is_some() {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            };
        }

        if tabs.closing_app || tabs.closing_files || tabs.pending_close.is_some() {
            return if tabs.hovered_modal_btn.is_some() {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            };
        }

        if matches!(self.drag, DragState::SidebarResize { .. }) {
            return CursorIcon::ColResize;
        }
        if matches!(self.drag, DragState::TerminalResize { .. }) {
            return CursorIcon::RowResize;
        }
        if self.drag == DragState::SelectingText || self.drag == DragState::TerminalSelecting {
            return CursorIcon::Text;
        }

        if self.drag != DragState::None {
            return CursorIcon::Default;
        }

        let (mx, my) = (self.mouse_x as usize, self.mouse_y as usize);

        if sidebar.visible && (mx as i32 - sidebar.width as i32).abs() <= 4 {
            return CursorIcon::ColResize;
        }

        if terminal.is_open && mx >= layout.content_left {
            let term_y = screen_h.saturating_sub(terminal.height);
            if (my as i32 - term_y as i32).abs() <= 3 {
                return CursorIcon::RowResize;
            }
            if my >= term_y && my < term_y + TERMINAL_TAB_BAR_HEIGHT {
                return if terminal.hovered_new
                    || terminal.hovered_tab.is_some()
                    || terminal.hovered_close_tab.is_some()
                {
                    CursorIcon::Pointer
                } else {
                    CursorIcon::Default
                };
            }
            let vbar_x = layout.content_right;
            if mx >= vbar_x {
                return CursorIcon::Default;
            }
            if my >= term_y + TERMINAL_TAB_BAR_HEIGHT {
                return CursorIcon::Text;
            }
        }

        if sidebar.visible && mx < layout.content_left {
            return if sidebar.hovered_menu_header
                || sidebar.hovered_menu_item.is_some()
                || sidebar.hovered_terminal_header
                || sidebar.hovered_root_header
                || sidebar.hovered_tree_row.is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            };
        }

        if !tabs.tabs.is_empty() && my < TAB_BAR_HEIGHT {
            return if tabs.hovered_tab.is_some() || tabs.hovered_close.is_some() {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            };
        }

        if tabs.find.is_open {
            let bar_h = if tabs.find.is_replace { 66 } else { 36 };
            let bar_y = layout.content_bottom + SCROLLBAR_THICKNESS;

            if my >= bar_y && my < bar_y + bar_h && mx >= layout.content_left {
                if tabs.find.hovered_btn.is_some() {
                    return CursorIcon::Pointer;
                }

                let cw = 9;
                let close_w = "Close".len() * cw + 16;
                let strip_min_x = layout.content_left;
                let strip_max_x = layout.content_right.saturating_sub(close_w + 6 + 6);

                let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
                let toggle_w = (toggle_label.len() * cw + 6) as i32;
                let toggle_x = strip_min_x as i32 + 6;

                let scrollable_min_x = toggle_x as usize + toggle_w as usize + 6;
                let f_input_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;

                let f_input_y = bar_y + 6;
                let mx_i = mx as i32;

                if my >= f_input_y
                    && my < f_input_y + 24
                    && mx_i >= f_input_x
                    && mx_i < f_input_x + 240
                    && mx >= scrollable_min_x
                    && mx < strip_max_x
                {
                    return CursorIcon::Text;
                }

                if tabs.find.is_replace {
                    let r_input_y = if tabs.find.is_replace {
                        bar_y + 36
                    } else {
                        bar_y + 6
                    };

                    if my >= r_input_y
                        && my < r_input_y + 24
                        && mx_i >= f_input_x
                        && mx_i < f_input_x + 240
                        && mx >= scrollable_min_x
                        && mx < strip_max_x
                    {
                        return CursorIcon::Text;
                    }
                }

                return CursorIcon::Default;
            }
        }

        if tabs.active_tab().is_some()
            && my < layout.content_bottom
            && mx >= layout.code_x
            && mx < layout.content_right
        {
            CursorIcon::Text
        } else {
            CursorIcon::Default
        }
    }

    fn update_tab_hover(&self, tabs: &mut TabManager, layout: &ViewportLayout, char_w: usize) {
        tabs.hovered_tab = None;
        tabs.hovered_close = None;

        let (mx, my) = (self.mouse_x as usize, self.mouse_y as usize);

        if !tabs.tabs.is_empty() && my < TAB_BAR_HEIGHT && mx >= layout.content_left {
            let mx_i32 = mx as i32;
            let mut tx = layout.content_left as i32 - tabs.scroll_x as i32;

            for (idx, tab) in tabs.tabs.iter().enumerate() {
                let tw = tab.width(char_w) as i32;
                let tab_x0 = tx;
                let tab_x1 = tx + tw;

                if mx_i32 >= tab_x0 && mx_i32 < tab_x1 {
                    tabs.hovered_tab = Some(idx);
                    if mx_i32 >= tab_x1 - 27 && mx_i32 <= tab_x1 - 7 {
                        tabs.hovered_close = Some(idx);
                    }
                    break;
                }

                tx += tw;
            }
        }
    }

    pub fn handle_cursor_move(
        &mut self,
        x: f64,
        y: f64,
        tabs: &mut TabManager,
        sidebar: &mut Sidebar,
        terminal: &mut Terminal,
        layout: &ViewportLayout,
        char_w: usize,
        line_h: usize,
        screen_w: usize,
        screen_h: usize,
    ) -> bool {
        self.mouse_x = x;
        self.mouse_y = y;
        let (mx, my) = (x as usize, y as usize);

        if !self.is_left_down && self.drag != DragState::None {
            self.drag = DragState::None;
        }

        if let Some(ref mut menu) = self.context_menu {
            let prev_h = menu.hovered_idx;
            if mx >= menu.x && mx < menu.x + menu.width && my >= menu.y && my < menu.y + menu.height
            {
                let row_h = menu.height / 2;
                let idx = (my - menu.y) / row_h.max(1);
                if idx == 0 {
                    menu.hovered_idx = Some(0);
                } else if idx == 1 && !tabs.tabs.is_empty() {
                    menu.hovered_idx = Some(1);
                } else {
                    menu.hovered_idx = None;
                }
            } else {
                menu.hovered_idx = None;
            }
            return prev_h != menu.hovered_idx;
        }

        if sidebar.visible {
            sidebar.clamp_scroll(screen_h);
        }

        if let Some(modal) = compute_modal_layout(tabs, screen_w, screen_h, char_w, line_h) {
            let prev = tabs.hovered_modal_btn;
            tabs.hovered_modal_btn = None;
            for btn in &modal.buttons {
                if mx >= btn.x && mx < btn.x + btn.w && my >= btn.y && my < btn.y + btn.h {
                    tabs.hovered_modal_btn = Some(btn.id);
                    break;
                }
            }
            return prev != tabs.hovered_modal_btn;
        }

        let prev_sh = sidebar.hovered_menu_header;
        let prev_sitem = sidebar.hovered_menu_item;
        let prev_sterm = sidebar.hovered_terminal_header;
        let prev_sroot = sidebar.hovered_root_header;
        let prev_stree = sidebar.hovered_tree_row;
        let prev_th = tabs.hovered_tab;
        let prev_ch = tabs.hovered_close;
        let prev_t_new = terminal.hovered_new;
        let prev_t_tab = terminal.hovered_tab;
        let prev_t_tab_close = terminal.hovered_close_tab;
        let prev_find_hover = tabs.find.hovered_btn;

        sidebar.hovered_menu_header = false;
        sidebar.hovered_menu_item = None;
        sidebar.hovered_terminal_header = false;
        sidebar.hovered_root_header = false;
        sidebar.hovered_tree_row = None;
        update_terminal_tab_hover(
            terminal,
            mx,
            my,
            layout.content_left,
            screen_w,
            screen_h,
            char_w,
        );
        update_find_hover(tabs, layout, screen_w, char_w, mx, my);

        let total_sidebar_h = sidebar.total_content_height();
        let has_sidebar_scroll = total_sidebar_h > screen_h;
        let bar_x = sidebar.width.saturating_sub(SCROLLBAR_THICKNESS);

        let can_save = tabs
            .active_tab()
            .map(|t| t.buffer.is_modified)
            .unwrap_or(false);
        let has_folder = sidebar.root_folder.is_some();

        if sidebar.visible && mx < sidebar.width {
            tabs.hovered_tab = None;
            tabs.hovered_close = None;

            if !(has_sidebar_scroll && mx >= bar_x) {
                let content_y = my as i32 + sidebar.scroll_y as i32;
                if content_y >= 0 {
                    let cy = content_y as usize;
                    let menu_total_h = sidebar.menu_total_height();

                    if cy < TAB_BAR_HEIGHT {
                        sidebar.hovered_menu_header = true;
                    } else if sidebar.menu_expanded && cy < menu_total_h {
                        let item_idx = (cy - TAB_BAR_HEIGHT) / SIDEBAR_ROW_HEIGHT;
                        let items = sidebar.menu_items();
                        if item_idx < items.len() {
                            let item = items[item_idx].0;
                            let is_disabled = (item == MenuItem::Save && !can_save)
                                || (item == MenuItem::CloseFolder && !has_folder);
                            if !is_disabled {
                                sidebar.hovered_menu_item = Some(item);
                            }
                        }
                    } else if cy >= menu_total_h && cy < menu_total_h + TAB_BAR_HEIGHT {
                        sidebar.hovered_terminal_header = true;
                    } else if has_folder && cy >= menu_total_h + TAB_BAR_HEIGHT {
                        let rel_y = cy - (menu_total_h + TAB_BAR_HEIGHT);
                        if rel_y < TAB_BAR_HEIGHT {
                            sidebar.hovered_root_header = true;
                        } else if sidebar.root_expanded {
                            let tree_y = rel_y - TAB_BAR_HEIGHT;
                            let node_idx = tree_y / SIDEBAR_ROW_HEIGHT;
                            if node_idx < sidebar.nodes.len() {
                                sidebar.hovered_tree_row = Some(node_idx);
                            }
                        }
                    }
                }
            }
        } else {
            self.update_tab_hover(tabs, layout, char_w);
        }

        let mut changed = prev_sh != sidebar.hovered_menu_header
            || prev_sitem != sidebar.hovered_menu_item
            || prev_sterm != sidebar.hovered_terminal_header
            || prev_sroot != sidebar.hovered_root_header
            || prev_stree != sidebar.hovered_tree_row
            || prev_th != tabs.hovered_tab
            || prev_ch != tabs.hovered_close
            || prev_t_new != terminal.hovered_new
            || prev_t_tab != terminal.hovered_tab
            || prev_t_tab_close != terminal.hovered_close_tab
            || prev_find_hover != tabs.find.hovered_btn;

        match self.drag {
            DragState::SidebarResize { start_x, start_w } => {
                let delta = self.mouse_x - start_x;
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
                let delta = start_y - self.mouse_y;
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
                    let col = if (mx as i32) <= (text_left as i32) {
                        0
                    } else {
                        let rel_x = mx - text_left;
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
                            let target =
                                (start_line as f64 + ((self.mouse_y - start_y) / travel) * max_s)
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
                            + ((self.mouse_y - start_y) / travel) * max_s)
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
                        let row = my.saturating_sub(TAB_BAR_HEIGHT + TOP_PADDING) / line_h;
                        let target_line = active_buf.scroll_line + row;
                        let target_col = if mx >= layout.code_x {
                            active_buf.scroll_col + (mx - layout.code_x) / char_w
                        } else {
                            0
                        };
                        active_buf.set_cursor_at(target_line, target_col);
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
                    let active_buf = &mut active_tab.buffer;
                    let total = active_buf.text().len_lines();
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
                            let target =
                                (start_line as f64 + ((self.mouse_y - start_y) / travel) * max_s)
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
                            let target =
                                (start_col as f64 + ((self.mouse_x - start_x) / travel) * max_s)
                                    .clamp(0.0, max_s) as usize;
                            if active_buf.scroll_col != target {
                                active_buf.scroll_col = target;
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
            self.drag = DragState::None;
            return if was_dragging {
                ActionEvent::Redraw
            } else {
                ActionEvent::None
            };
        }

        self.is_left_down = true;
        let (mx, my) = (self.mouse_x as usize, self.mouse_y as usize);

        let is_find_bar = tabs.find.is_open
            && my >= layout.content_bottom + SCROLLBAR_THICKNESS
            && mx >= layout.content_left;

        if my < TAB_BAR_HEIGHT
            || mx < layout.content_left
            || (mx >= layout.content_right && !is_find_bar)
            || (my >= layout.content_bottom && !is_find_bar)
        {
            self.last_click_time = None;
            self.click_count = 0;
        }

        if let Some(menu) = self.context_menu.take() {
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
            return ActionEvent::None;
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
                return ActionEvent::None;
            }

            if my >= tabbar_y && my < tabbar_y + tabbar_h {
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
                                terminal.active_idx = idx;
                                terminal.focused = true;
                                terminal.ensure_active_tab_visible(char_w, available_w);
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
                }
                return ActionEvent::None;
            }

            let shell_y = tabbar_y + TERMINAL_TAB_BAR_HEIGHT;
            let shell_h = screen_h.saturating_sub(shell_y);
            let vbar_x = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
            let track_h = shell_h;

            if mx >= vbar_x && mx < screen_w && my >= shell_y && my < shell_y + track_h {
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
                terminal.focused = true;
                return ActionEvent::Redraw;
            }

            if my >= shell_y && mx >= layout.content_left && mx < vbar_x {
                terminal.focused = true;
                let text_left = layout.content_left + 14;
                let row = my.saturating_sub(shell_y + 4) / line_h.max(1);

                if let Some(tab) = terminal.active_tab_mut() {
                    let line_idx = tab.scroll_line + row;
                    let col = if (mx as i32) < (text_left as i32) {
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
                return ActionEvent::None;
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

                if has_folder && cy >= menu_total_h + TAB_BAR_HEIGHT {
                    let rel_y = cy - (menu_total_h + TAB_BAR_HEIGHT);
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
                                return ActionEvent::OpenFile(sidebar.nodes[node_idx].path.clone());
                            }
                        }
                    }
                }
            }
            return ActionEvent::None;
        }

        if !tabs.tabs.is_empty() && my < TAB_BAR_HEIGHT {
            if mx >= layout.content_left {
                terminal.focused = false;
                let available_w = screen_w.saturating_sub(layout.content_left);
                if let Some(close_idx) = tabs.hovered_close {
                    tabs.closing_app = false;
                    tabs.request_close(close_idx);
                    tabs.clamp_scroll(char_w, available_w);
                    self.update_tab_hover(tabs, layout, char_w);
                    return ActionEvent::Redraw;
                }
                if let Some(tab_idx) = tabs.hovered_tab {
                    tabs.active_idx = Some(tab_idx);
                    tabs.ensure_active_tab_visible(char_w, available_w);
                    self.update_tab_hover(tabs, layout, char_w);
                    return ActionEvent::Redraw;
                }
            }
            return ActionEvent::None;
        }

        if tabs.find.is_open {
            let bar_h = if tabs.find.is_replace { 66 } else { 36 };
            let bar_y = layout.content_bottom + SCROLLBAR_THICKNESS;
            let bar_w = screen_w.saturating_sub(layout.content_left);

            if my >= bar_y
                && my < bar_y + bar_h
                && mx >= layout.content_left
                && mx < layout.content_left + bar_w
            {
                terminal.focused = false;
                tabs.find.focused = true;

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
                    return ActionEvent::Redraw;
                }

                let strip_min_x = layout.content_left;
                let strip_max_x = close_btn_x.saturating_sub(6);

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
                            2 => tabs.find.select_word(),
                            3 => tabs.find.select_all(),
                            _ => {}
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
                                2 => tabs.find.select_word(),
                                3 => tabs.find.select_all(),
                                _ => {}
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

        if let Some(active_tab) = tabs.active_tab_mut() {
            let active_buf = &mut active_tab.buffer;
            let total = active_buf.text().len_lines();
            let usable_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT);

            if mx >= layout.content_right
                && mx < screen_w
                && my >= TAB_BAR_HEIGHT
                && my < layout.content_bottom
            {
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
            } else if my >= layout.content_bottom
                && my < layout.content_bottom + SCROLLBAR_THICKNESS
                && mx >= layout.bar_start_x
                && mx < layout.content_right
            {
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
            } else if my >= TAB_BAR_HEIGHT
                && my < layout.content_bottom
                && mx >= layout.content_left
                && mx < layout.content_right
            {
                if line_h > 0 && char_w > 0 {
                    let row = my.saturating_sub(TAB_BAR_HEIGHT + TOP_PADDING) / line_h;
                    let target_line = active_buf.scroll_line + row;
                    let target_col = if mx >= layout.code_x {
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

                    active_buf.set_cursor_at(target_line, target_col);

                    match self.click_count {
                        2 => {
                            active_buf.select_word_at_cursor(target_col);
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

        ActionEvent::None
    }

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

        let (mx, my) = (self.mouse_x as usize, self.mouse_y as usize);

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
                self.update_tab_hover(tabs, layout, char_w);
                return true;
            }
            return false;
        }

        if tabs.find.is_open {
            let bar_h = if tabs.find.is_replace { 66 } else { 36 };
            let bar_y = layout.content_bottom + SCROLLBAR_THICKNESS;

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

                let rep_y = if tabs.find.is_replace {
                    bar_y + 36
                } else {
                    bar_y + 6
                };
                let rep_hovered = tabs.find.is_replace
                    && my >= rep_y
                    && my < rep_y + 24
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
                    update_find_hover(tabs, layout, screen_w, char_w, mx, my);
                    return true;
                }
                return false;
            }
        }

        if let Some(active_tab) = tabs.active_tab_mut() {
            let active_buf = &mut active_tab.buffer;
            let max_l = active_buf.text().len_lines().saturating_sub(1);
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
