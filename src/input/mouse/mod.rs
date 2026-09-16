pub mod click;
pub mod drag;
pub mod hover;
pub mod wheel;

pub use hover::compute_bottom_bars_y;

use super::{DragState, InputHandler};
use crate::config::*;
use crate::editor::TabManager;
use crate::sidebar::{MenuItem, Sidebar};
use crate::terminal::Terminal;
use crate::ui::layout::ViewportLayout;
use crate::ui::overlays::compute_modal_layout;
use hover::{
    update_find_hover, update_quick_open_hover, update_tab_hover, update_terminal_tab_hover,
};
use winit::window::CursorIcon;

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
        if matches!(self.drag, DragState::SidebarResize { .. }) {
            return CursorIcon::ColResize;
        }
        if matches!(self.drag, DragState::TerminalResize { .. }) {
            return CursorIcon::RowResize;
        }
        if self.drag == DragState::SelectingText
            || self.drag == DragState::TerminalSelecting
            || self.drag == DragState::FindSelecting
            || self.drag == DragState::QuickOpenSelecting
        {
            return CursorIcon::Text;
        }
        if self.drag != DragState::None {
            return CursorIcon::Default;
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
                || sidebar.hovered_github_header
                || sidebar.hovered_github_row.is_some()
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

        let (find_y, qo_y) = compute_bottom_bars_y(tabs, layout);
        if tabs.quick_open.is_open {
            if tabs.quick_open.hovered_close || tabs.quick_open.hovered_match.is_some() {
                return CursorIcon::Pointer;
            }
            let input_y = qo_y + 6;
            let close_w = "Close".len() * 9 + 16;
            let close_btn_x =
                (layout.content_right + SCROLLBAR_THICKNESS).saturating_sub(close_w + 6);
            let input_x = layout.content_left + 6;
            if mx >= input_x && mx < close_btn_x && my >= input_y && my < input_y + 24 {
                return CursorIcon::Text;
            }
        }

        if tabs.find.is_open {
            let bar_h = if tabs.find.is_replace { 66 } else { 36 };
            let bar_y = find_y;
            if my >= bar_y && my < bar_y + bar_h && mx >= layout.content_left {
                if tabs.find.hovered_btn.is_some() {
                    return CursorIcon::Pointer;
                }
                let cw = 9;
                let close_w = "Close".len() * cw + 16;
                let strip_min_x = layout.content_left;
                let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
                let toggle_w = (toggle_label.len() * cw + 6) as i32;
                let toggle_x = strip_min_x as i32 + 6;
                let scrollable_min_x = toggle_x as usize + toggle_w as usize + 6;
                let f_input_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
                let f_input_y = bar_y + 6;
                let strip_max_x = layout.content_right.saturating_sub(close_w + 6 + 6);
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
                    let r_input_y = bar_y + 36;
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
        let mx = x.max(0.0) as usize;
        let my = y.max(0.0) as usize;

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

        let is_dragging = self.drag != DragState::None;
        let prev_sh = sidebar.hovered_menu_header;
        let prev_sitem = sidebar.hovered_menu_item;
        let prev_sterm = sidebar.hovered_terminal_header;
        let prev_sgh = sidebar.hovered_github_header;
        let prev_sgh_row = sidebar.hovered_github_row;
        let prev_sroot = sidebar.hovered_root_header;
        let prev_stree = sidebar.hovered_tree_row;
        let prev_th = tabs.hovered_tab;
        let prev_ch = tabs.hovered_close;
        let prev_t_new = terminal.hovered_new;
        let prev_t_tab = terminal.hovered_tab;
        let prev_t_tab_close = terminal.hovered_close_tab;
        let prev_find_hover = tabs.find.hovered_btn;
        let prev_qo_close = tabs.quick_open.hovered_close;
        let prev_qo_match = tabs.quick_open.hovered_match;

        sidebar.hovered_menu_header = false;
        sidebar.hovered_menu_item = None;
        sidebar.hovered_terminal_header = false;
        sidebar.hovered_github_header = false;
        sidebar.hovered_github_row = None;
        sidebar.hovered_root_header = false;
        sidebar.hovered_tree_row = None;

        let (find_y, qo_y) = compute_bottom_bars_y(tabs, layout);
        if !is_dragging {
            update_terminal_tab_hover(
                terminal,
                mx,
                my,
                layout.content_left,
                screen_w,
                screen_h,
                char_w,
            );
            update_find_hover(tabs, layout, screen_w, char_w, mx, my, find_y);
            update_quick_open_hover(tabs, layout, screen_w, char_w, mx, my, qo_y);
        }

        let total_sidebar_h = sidebar.total_content_height();
        let has_sidebar_scroll = total_sidebar_h > screen_h;
        let bar_x = sidebar.width.saturating_sub(SCROLLBAR_THICKNESS);
        let can_save = tabs
            .active_tab()
            .map(|t| t.buffer.is_modified)
            .unwrap_or(false);
        let has_folder = sidebar.root_folder.is_some();

        if !is_dragging {
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
                        } else if cy >= menu_total_h + TAB_BAR_HEIGHT {
                            let mut sec_y = menu_total_h + TAB_BAR_HEIGHT;
                            let has_github = has_folder && sidebar.git_snapshot.has_github_dir;
                            let mut handled = false;
                            if has_github {
                                let gh_total_h = sidebar.github_total_height();
                                if cy >= sec_y && cy < sec_y + TAB_BAR_HEIGHT {
                                    sidebar.hovered_github_header = true;
                                    handled = true;
                                } else if sidebar.github_expanded
                                    && cy >= sec_y + TAB_BAR_HEIGHT
                                    && cy < sec_y + gh_total_h
                                {
                                    let rel_row = cy - (sec_y + TAB_BAR_HEIGHT);
                                    let row_idx = rel_row / SIDEBAR_ROW_HEIGHT;
                                    if !sidebar.git_snapshot.files.is_empty()
                                        && row_idx < sidebar.git_snapshot.files.len()
                                    {
                                        sidebar.hovered_github_row = Some(row_idx);
                                    }
                                    handled = true;
                                }
                                sec_y += gh_total_h;
                            }
                            if !handled && has_folder && cy >= sec_y {
                                let rel_y = cy - sec_y;
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
                }
            } else {
                update_tab_hover(tabs, layout, char_w, mx, my);
            }
        }

        let mut changed = prev_sh != sidebar.hovered_menu_header
            || prev_sitem != sidebar.hovered_menu_item
            || prev_sterm != sidebar.hovered_terminal_header
            || prev_sgh != sidebar.hovered_github_header
            || prev_sgh_row != sidebar.hovered_github_row
            || prev_sroot != sidebar.hovered_root_header
            || prev_stree != sidebar.hovered_tree_row
            || prev_th != tabs.hovered_tab
            || prev_ch != tabs.hovered_close
            || prev_t_new != terminal.hovered_new
            || prev_t_tab != terminal.hovered_tab
            || prev_t_tab_close != terminal.hovered_close_tab
            || prev_find_hover != tabs.find.hovered_btn
            || prev_qo_close != tabs.quick_open.hovered_close
            || prev_qo_match != tabs.quick_open.hovered_match;

        let drag_changed = drag::process_drag(
            self, tabs, sidebar, terminal, layout, char_w, line_h, screen_w, screen_h,
        );
        changed = changed || drag_changed;
        changed
    }
}
