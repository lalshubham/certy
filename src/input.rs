use crate::config::*;
use crate::layout::{calc_thumb, compute_modal_layout, ViewportLayout};
use crate::sidebar::{MenuItem, Sidebar};
use crate::tabs::TabManager;
use crate::terminal::Terminal;
use arboard::Clipboard;
use std::path::PathBuf;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::window::CursorIcon;

#[derive(Default, PartialEq)]
pub enum DragState {
    #[default]
    None,
    Vertical {
        start_y: f64,
        start_line: usize,
    },
    Horizontal {
        start_x: f64,
        start_col: usize,
    },
    SelectingText,
    SidebarResize {
        start_x: f64,
        start_w: usize,
    },
    SidebarScroll {
        start_y: f64,
        start_scroll: usize,
    },
    TerminalResize {
        start_y: f64,
        start_h: usize,
    },
    TerminalVertical {
        start_y: f64,
        start_line: usize,
    },
    TerminalHorizontal {
        start_x: f64,
        start_col: usize,
    },
    TerminalSelecting,
}

#[derive(Default)]
pub struct InputHandler {
    pub drag: DragState,
    pub is_left_down: bool,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub scroll_accum_y: f64,
    pub scroll_accum_x: f64,
    pub modifiers: ModifiersState,
    pub ctrl_down: bool,
    pub shift_down: bool,
}

pub enum ActionEvent {
    None,
    Redraw,
    Menu(MenuItem),
    ToggleTerminal,
    OpenFile(PathBuf),
    SaveTab(usize),
    DiscardTab(usize),
    SaveAllAndExit,
    DiscardAllAndExit,
    CancelClose,
}

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
        let new_btn_w = "New".len() * char_w + 20;
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
                    if mx_i32 >= tab_x1 - 20 && mx_i32 <= tab_x1 - 2 {
                        terminal.hovered_close_tab = Some(idx);
                    }
                    break;
                }
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

        if tabs.closing_app || tabs.pending_close.is_some() {
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

        if (mx as i32 - sidebar.width as i32).abs() <= 4 {
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
            if my >= term_y + TERMINAL_TAB_BAR_HEIGHT {
                return CursorIcon::Text;
            }
        }

        if mx < layout.content_left {
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
                    if mx_i32 >= tab_x1 - 20 && mx_i32 <= tab_x1 - 4 {
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

        sidebar.clamp_scroll(screen_h);

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

        let total_sidebar_h = sidebar.total_content_height();
        let has_sidebar_scroll = total_sidebar_h > screen_h;
        let bar_x = sidebar.width.saturating_sub(SCROLLBAR_THICKNESS);

        let can_save = tabs
            .active_tab()
            .map(|t| t.buffer.is_modified)
            .unwrap_or(false);
        let has_folder = sidebar.root_folder.is_some();

        if mx < sidebar.width {
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
            || prev_t_tab_close != terminal.hovered_close_tab;

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
                        tab.scroll_col
                    } else {
                        let rel_x = mx - text_left;
                        tab.scroll_col + (rel_x / char_w.max(1))
                    };
                    let line_len = tab
                        .get_line_text(line_idx)
                        .map(|s| s.chars().count())
                        .unwrap_or(0);
                    let clamped_col = col.min(line_len);
                    tab.selection_end = Some((line_idx, clamped_col));

                    let active_line_idx = total_l.saturating_sub(1);
                    if !tab.is_running && line_idx == active_line_idx {
                        let p_len = tab.prompt().chars().count();
                        let cur = clamped_col
                            .saturating_sub(p_len)
                            .min(tab.current_input.chars().count());
                        tab.cursor_col = cur;
                        let vbar_x = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
                        let vis_cols = if char_w > 0 {
                            vbar_x.saturating_sub(text_left) / char_w
                        } else {
                            0
                        };
                        tab.ensure_cursor_visible(vis_cols);
                    }
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
                let track_h = shell_h.saturating_sub(SCROLLBAR_THICKNESS);
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
            DragState::TerminalHorizontal { start_x, start_col } => {
                let term_w = screen_w.saturating_sub(layout.content_left);
                let track_w = term_w.saturating_sub(SCROLLBAR_THICKNESS);
                let text_left = layout.content_left + 14;
                let text_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
                let vis_cols = if char_w > 0 {
                    text_right.saturating_sub(text_left) / char_w
                } else {
                    0
                };
                if let Some(tab) = terminal.active_tab_mut() {
                    let max_c = tab.max_content_cols();
                    if let Some((_, tw)) = calc_thumb(max_c, vis_cols, tab.scroll_col, track_w) {
                        let travel = track_w.saturating_sub(tw) as f64;
                        if travel > 0.0 {
                            let max_s = (max_c - vis_cols) as f64;
                            let target =
                                (start_col as f64 + ((self.mouse_x - start_x) / travel) * max_s)
                                    .clamp(0.0, max_s) as usize;
                            if tab.scroll_col != target {
                                tab.scroll_col = target;
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
                    if let Some((_, th)) = calc_thumb(
                        total,
                        layout.visible_lines,
                        active_buf.scroll_line,
                        usable_h,
                    ) {
                        let travel = usable_h.saturating_sub(th) as f64;
                        if travel > 0.0 {
                            let max_s = (total - layout.visible_lines) as f64;
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

        if let Some(modal) = compute_modal_layout(tabs, screen_w, screen_h, char_w, line_h) {
            for btn in &modal.buttons {
                if mx >= btn.x && mx < btn.x + btn.w && my >= btn.y && my < btn.y + btn.h {
                    if tabs.closing_app {
                        return match btn.id {
                            0 => ActionEvent::SaveAllAndExit,
                            1 => ActionEvent::DiscardAllAndExit,
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

        if (mx as i32 - sidebar.width as i32).abs() <= 4 {
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
                let new_btn_w = "New".len() * char_w + 20;
                let strip_min_x = layout.content_left + new_btn_w;
                let strip_max_x = screen_w;
                let available_w = strip_max_x.saturating_sub(strip_min_x);

                if mx < strip_min_x {
                    terminal.add_terminal(char_w, available_w);
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
                            if mx_i32 >= tab_x1 - 20 && mx_i32 <= tab_x1 - 2 {
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
            let term_w = screen_w.saturating_sub(layout.content_left);
            let vbar_x = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
            let hbar_y = screen_h.saturating_sub(SCROLLBAR_THICKNESS);
            let track_h = shell_h.saturating_sub(SCROLLBAR_THICKNESS);
            let track_w = term_w.saturating_sub(SCROLLBAR_THICKNESS);

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

            if my >= hbar_y && mx >= layout.content_left && mx < layout.content_left + track_w {
                let text_left = layout.content_left + 14;
                let text_right = vbar_x;
                let vis_cols = if char_w > 0 {
                    text_right.saturating_sub(text_left) / char_w
                } else {
                    0
                };
                if let Some(tab) = terminal.active_tab_mut() {
                    let max_c = tab.max_content_cols();
                    if let Some((tx_offset, tw)) =
                        calc_thumb(max_c, vis_cols, tab.scroll_col, track_w)
                    {
                        let tx = layout.content_left + tx_offset;
                        if mx >= tx && mx < tx + tw {
                            self.drag = DragState::TerminalHorizontal {
                                start_x: self.mouse_x,
                                start_col: tab.scroll_col,
                            };
                        } else {
                            let ratio = ((mx.saturating_sub(layout.content_left)) as f64
                                / track_w as f64)
                                .clamp(0.0, 1.0);
                            tab.scroll_col = (ratio * (max_c - vis_cols) as f64) as usize;
                            self.drag = DragState::TerminalHorizontal {
                                start_x: self.mouse_x,
                                start_col: tab.scroll_col,
                            };
                            return ActionEvent::Redraw;
                        }
                    }
                }
                terminal.focused = true;
                return ActionEvent::Redraw;
            }

            if my >= shell_y && my < hbar_y && mx >= layout.content_left && mx < vbar_x {
                terminal.focused = true;
                let text_left = layout.content_left + 14;
                let text_right = vbar_x;
                let vis_cols = if char_w > 0 {
                    text_right.saturating_sub(text_left) / char_w
                } else {
                    0
                };
                let row = my.saturating_sub(shell_y + 4) / line_h.max(1);

                if let Some(tab) = terminal.active_tab_mut() {
                    let line_idx = tab.scroll_line + row;
                    let col = if (mx as i32) < (text_left as i32) {
                        tab.scroll_col
                    } else {
                        tab.scroll_col + ((mx - text_left) / char_w.max(1))
                    };

                    let total_l = tab.total_lines();
                    if line_idx < total_l {
                        let line_len = tab
                            .get_line_text(line_idx)
                            .map(|s| s.chars().count())
                            .unwrap_or(0);
                        let clamped_col = col.min(line_len);
                        tab.selection_anchor = Some((line_idx, clamped_col));
                        tab.selection_end = Some((line_idx, clamped_col));

                        let active_line_idx = total_l.saturating_sub(1);
                        if !tab.is_running && line_idx == active_line_idx {
                            let p_len = tab.prompt().chars().count();
                            let cur = clamped_col
                                .saturating_sub(p_len)
                                .min(tab.current_input.chars().count());
                            tab.cursor_col = cur;
                            tab.ensure_cursor_visible(vis_cols);
                        }
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

        if mx < sidebar.width {
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

        if let Some(active_tab) = tabs.active_tab_mut() {
            let active_buf = &mut active_tab.buffer;
            let total = active_buf.text().len_lines();
            let usable_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT);

            if mx >= layout.content_right
                && mx < screen_w
                && my >= TAB_BAR_HEIGHT
                && my < layout.content_bottom
            {
                terminal.focused = false;
                if let Some((ty, th)) = calc_thumb(
                    total,
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
                        active_buf.scroll_line =
                            (ratio * (total - layout.visible_lines) as f64) as usize;
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
                terminal.focused = false;
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
                terminal.focused = false;
                if line_h > 0 && char_w > 0 {
                    let row = my.saturating_sub(TAB_BAR_HEIGHT + TOP_PADDING) / line_h;
                    let target_line = active_buf.scroll_line + row;
                    let target_col = if mx >= layout.code_x {
                        active_buf.scroll_col + (mx - layout.code_x) / char_w
                    } else {
                        0
                    };
                    active_buf.set_cursor_at(target_line, target_col);
                    active_buf.selection_anchor = Some(active_buf.cursor_char);
                    self.drag = DragState::SelectingText;
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

        if mx < sidebar.width {
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
                let new_btn_w = "New".len() * char_w + 20;
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

                    let text_left = layout.content_left + 14;
                    let text_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
                    let vis_cols = if char_w > 0 {
                        text_right.saturating_sub(text_left) / char_w
                    } else {
                        0
                    };
                    let max_c = tab.max_content_cols().saturating_sub(vis_cols);

                    let next_l = (tab.scroll_line as i32 - lines).clamp(0, max_l as i32) as usize;
                    let next_c = (tab.scroll_col as i32 - cols).clamp(0, max_c as i32) as usize;

                    if tab.scroll_line != next_l || tab.scroll_col != next_c {
                        tab.scroll_line = next_l;
                        tab.scroll_col = next_c;
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

        if let Some(active_tab) = tabs.active_tab_mut() {
            let active_buf = &mut active_tab.buffer;
            let max_l = active_buf
                .text()
                .len_lines()
                .saturating_sub(layout.visible_lines);
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

    pub fn handle_key(
        &mut self,
        event: &KeyEvent,
        tabs: &mut TabManager,
        terminal: &mut Terminal,
        layout: &ViewportLayout,
        char_w: usize,
        line_h: usize,
        screen_w: usize,
        clipboard: &mut Option<Clipboard>,
    ) -> bool {
        if event.state == ElementState::Released {
            match &event.logical_key {
                Key::Named(NamedKey::Control) => self.ctrl_down = false,
                Key::Named(NamedKey::Shift) => self.shift_down = false,
                _ => {}
            }
            if matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight)
            ) {
                self.ctrl_down = false;
            }
            if matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight)
            ) {
                self.shift_down = false;
            }
            return false;
        }

        if tabs.closing_app || tabs.pending_close.is_some() {
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Escape))
            {
                tabs.closing_app = false;
                tabs.pending_close = None;
                return true;
            }
            return false;
        }

        if event.logical_key == Key::Named(NamedKey::Control)
            || matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight)
            )
        {
            self.ctrl_down = true;
            return false;
        }
        if event.logical_key == Key::Named(NamedKey::Shift)
            || matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight)
            )
        {
            self.shift_down = true;
            return false;
        }

        let is_ctrl = self.modifiers.control_key() || self.ctrl_down;
        let is_shift = self.modifiers.shift_key() || self.shift_down;

        if terminal.is_open && terminal.focused {
            let vis_rows = terminal.vis_rows(line_h);
            let text_left = layout.content_left + 14;
            let text_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
            let vis_cols = if char_w > 0 {
                text_right.saturating_sub(text_left) / char_w
            } else {
                0
            };

            let is_c = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyC))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("c") || c == "\u{3}",
                    _ => false,
                };
            let is_v = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyV))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("v") || c == "\u{16}",
                    _ => false,
                };

            let mut should_exit = false;

            if let Some(tab) = terminal.active_tab_mut() {
                if is_ctrl && is_c {
                    if let Some(text) = tab.selected_text() {
                        if let Some(cb) = clipboard.as_mut() {
                            let _ = cb.set_text(text);
                        }
                        return true;
                    }
                    tab.interrupt(vis_rows);
                    return true;
                }

                if is_ctrl && is_v {
                    if let Some(cb) = clipboard.as_mut() {
                        if let Ok(text) = cb.get_text() {
                            for ch in text.chars() {
                                if ch != '\n' && ch != '\r' {
                                    tab.insert_char(ch);
                                }
                            }
                            tab.selection_anchor = None;
                            tab.selection_end = None;
                            tab.ensure_cursor_visible(vis_cols);
                            return true;
                        }
                    }
                    return false;
                }

                if is_ctrl && matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyL)) {
                    tab.lines.clear();
                    tab.partial_line.clear();
                    tab.scroll_line = 0;
                    tab.scroll_col = 0;
                    tab.selection_anchor = None;
                    tab.selection_end = None;
                    return true;
                }

                tab.selection_anchor = None;
                tab.selection_end = None;

                match &event.logical_key {
                    Key::Named(NamedKey::Tab) => {
                        tab.tab_complete(vis_rows);
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::Enter) => {
                        if tab.execute_command(vis_rows) {
                            should_exit = true;
                        } else {
                            return true;
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        tab.delete_backwards();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::Delete) => {
                        tab.delete_forward();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        tab.move_left();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        tab.move_right();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::Home) => {
                        tab.cursor_col = 0;
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::End) => {
                        tab.cursor_col = tab.current_input.chars().count();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        tab.history_up();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        tab.history_down();
                        tab.ensure_cursor_visible(vis_cols);
                        return true;
                    }
                    _ => {
                        if !is_ctrl {
                            if let Some(txt) = &event.text {
                                for ch in txt.chars() {
                                    if !ch.is_control() {
                                        tab.insert_char(ch);
                                    }
                                }
                                tab.ensure_cursor_visible(vis_cols);
                                return true;
                            }
                        }
                    }
                }
            }

            if should_exit {
                let new_btn_w = "New".len() * char_w + 20;
                let strip_min_x = layout.content_left + new_btn_w;
                let strip_max_x = screen_w;
                let available_w = strip_max_x.saturating_sub(strip_min_x);
                let active_idx = terminal.active_idx;
                terminal.remove_terminal(active_idx, char_w, available_w);
                return true;
            }

            return false;
        }

        if let Some(tab) = tabs.active_tab_mut() {
            let buffer = &mut tab.buffer;

            let is_s = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyS))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("s") || c == "\u{13}",
                    _ => false,
                };
            let is_c = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyC))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("c") || c == "\u{3}",
                    _ => false,
                };
            let is_x = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyX))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("x") || c == "\u{18}",
                    _ => false,
                };
            let is_v = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyV))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("v") || c == "\u{16}",
                    _ => false,
                };
            let is_z = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyZ))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("z") || c == "\u{1a}",
                    _ => false,
                };
            let is_y = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyY))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("y") || c == "\u{19}",
                    _ => false,
                };
            let is_a = matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyA))
                || match &event.logical_key {
                    Key::Character(c) => c.eq_ignore_ascii_case("a") || c == "\u{1}",
                    _ => false,
                };

            if is_ctrl && is_s {
                let _ = buffer.save();
                if let Some(p) = &buffer.file_path {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        tab.title = name.to_string();
                    }
                }
                return true;
            }

            if is_ctrl && is_c {
                if let Some(text) = buffer.selected_text() {
                    if let Some(cb) = clipboard.as_mut() {
                        let _ = cb.set_text(text);
                    }
                }
                return false;
            }

            if is_ctrl && is_x {
                if let Some(text) = buffer.selected_text() {
                    if let Some(cb) = clipboard.as_mut() {
                        let _ = cb.set_text(text);
                    }
                    buffer.delete_selection();
                    buffer.fit_view(layout.visible_lines, layout.visible_cols);
                    return true;
                }
                return false;
            }

            if is_ctrl && is_v {
                if let Some(cb) = clipboard.as_mut() {
                    if let Ok(text) = cb.get_text() {
                        buffer.insert_str(&text);
                        buffer.fit_view(layout.visible_lines, layout.visible_cols);
                        return true;
                    }
                }
                return false;
            }

            if is_ctrl && is_z {
                if is_shift {
                    buffer.redo();
                } else {
                    buffer.undo();
                }
                buffer.fit_view(layout.visible_lines, layout.visible_cols);
                return true;
            }

            if is_ctrl && is_y {
                buffer.redo();
                buffer.fit_view(layout.visible_lines, layout.visible_cols);
                return true;
            }

            if is_ctrl && is_a {
                buffer.select_all();
                return true;
            }

            match &event.logical_key {
                Key::Named(NamedKey::Backspace) => buffer.delete_backwards(),
                Key::Named(NamedKey::Delete) => buffer.delete_forward(),
                Key::Named(NamedKey::Enter) => buffer.insert_char('\n'),
                Key::Named(NamedKey::Tab) => {
                    if !is_ctrl {
                        let (_, col) = buffer.cursor_pos();
                        let spaces = 4 - (col % 4);
                        buffer.insert_str(&"    "[..spaces]);
                    }
                }
                Key::Named(NamedKey::ArrowLeft) => buffer.move_left(is_shift),
                Key::Named(NamedKey::ArrowRight) => buffer.move_right(is_shift),
                Key::Named(NamedKey::ArrowUp) => buffer.move_up(is_shift),
                Key::Named(NamedKey::ArrowDown) => buffer.move_down(is_shift),
                _ => {
                    if !is_ctrl {
                        if let Some(txt) = &event.text {
                            for ch in txt.chars() {
                                if !ch.is_control() {
                                    buffer.insert_char(ch);
                                }
                            }
                        }
                    }
                }
            }

            buffer.fit_view(layout.visible_lines, layout.visible_cols);
            return true;
        }

        false
    }
}
